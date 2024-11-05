use std::borrow::Cow;
use std::sync::Arc;
use std::time::Duration;

use futures::{ SinkExt, StreamExt };
use log::{debug, error, info, warn};
use tokio::sync::Notify;
use tokio::sync::mpsc::{Sender, Receiver};
use tokio::time::sleep;
use tokio::{runtime::Runtime, net::TcpStream, task::JoinHandle, sync::mpsc};
use tokio_tungstenite::tungstenite::protocol::CloseFrame;
use tokio_tungstenite::tungstenite::protocol::frame::coding::CloseCode;
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream, tungstenite::Message};

use common::capture::{Message as CMsg, CaptureResult};
use common::processor::{Message as PMsg, CancelBehaviour};

const CONNECTION_FAILURE_RETRY_SLEEP: &[f64] = &[1.0, 1.0, 1.0, 10.0, 30.0, 60.0];

pub struct Line {
    _rt: Runtime,
    _handle: JoinHandle<()>,

    request_settings_tx: Sender<()>,
    upload_tx: Sender<CaptureResult>,

    settings_changed_notify: Arc<Notify>
}

impl Line {
    pub fn new() -> Line {
        let (request_settings_tx, request_settings_rx) = mpsc::channel(1);
        let (upload_tx, upload_rx) = mpsc::channel(crate::CONFIG.general.queue);

        let settings_changed_notify = Arc::new(Notify::new());

        let rt = tokio::runtime::Builder::new_multi_thread()
            .thread_name("ws-rt")
            .worker_threads(1)
            .enable_all()
            .build()
            .unwrap();

        let handle: JoinHandle<()> = {
            let settings_changed_notify = settings_changed_notify.clone();

            rt.spawn( async move {
                worker(settings_changed_notify, upload_rx, request_settings_rx).await;
            })
        };


        Line { _rt: rt, _handle: handle, request_settings_tx, upload_tx, settings_changed_notify }
    }

    pub fn subscribe_settings(& self) -> Arc<Notify> {
        self.settings_changed_notify.clone()
    }

    pub async fn init_settings(&mut self) {
        tokio::select! {
            () = self.settings_changed_notify.notified() => { },
            _ = async {
                self.request_settings_tx.send(()).await.unwrap();
                loop {
                    sleep(Duration::from_secs_f64(10.0)).await;
                    if self.request_settings_tx.capacity() > 0 { self.request_settings_tx.send(()).await.unwrap(); }
                }
            } => { }
        }
    }

    pub async fn upload(&mut self, upload: CaptureResult) {
        let max = crate::CONFIG.general.queue;
        let cap = self.upload_tx.capacity();
        let len = max - cap;
        if len > 0 { debug!("queue has {len} items; max is {max}"); }
        if cap > 0 {
            self.upload_tx.send(upload).await.unwrap();
        } else {
            error!("upload channel full. message dropped.")
        }
    }
}

async fn worker(settings_changed_notify: Arc<Notify>, mut upload_rx: Receiver<CaptureResult>, mut request_settings_rx: Receiver<()>) {
    let (mut failure_cnt, mut open, mut ws) = connect_ws(0).await;

    loop {
        if let Some(some_ws) = &mut ws {
            tokio::select! {
                msg = some_ws.next() => {
                    match msg {
                        Some(Ok(Message::Binary(b))) => {
                            match bincode::deserialize::<PMsg>(&b) {
                                Ok(msg) => match msg {
                                    PMsg::SetSettings{ settings, cancel_behaviour } => {
                                        info!("received new settings.");
                                        let (changed, init) = {
                                            let mut m = crate::SETTINGS.lock().unwrap();
                                            if let Some(s) = m.as_ref() {
                                                if *s != settings {
                                                    *m = Some(settings);
                                                    (true, false)
                                                } else { (false, false) }
                                                
                                            } else {
                                                *m = Some(settings);
                                                (true, true)
                                            }
                                        };
                                        match (cancel_behaviour, changed, init) {
                                            (_, _, true) | (CancelBehaviour::Allways, _, false) | (CancelBehaviour::IfUnequal, true, false) => {
                                                debug!("sending cancelation token due to behaviour={cancel_behaviour:?} changed={changed}, init={init}");
                                                settings_changed_notify.notify_waiters()
                                            },
                                            _ => { },
                                        }
                                    },
                                },
                                Err(e) => {
                                    error!("cannot deserialize message. {e}");
                                    open = false;
                                    if let Err(e) = some_ws.send(Message::Close(Some(CloseFrame {
                                            code: CloseCode::Unsupported,
                                            reason: Cow::Owned(format!("cannot deserialize message. {e}"))
                                        }))).await {
                                        error!("unable to send close frame. {e}");
                                        ws = None
                                    }
                                },
                            }
                        },
                        Some(Ok(Message::Text(_))) => {
                            debug!("received text. unsupported. closing connection.");
                            if open {
                                open = false;
                                if let Err(e) = some_ws.send(Message::Close(Some(CloseFrame {
                                        code: CloseCode::Unsupported,
                                        reason: Cow::Owned(format!("text unsupported"))
                                    }))).await {
                                    error!("unable to send close frame. {e}");
                                    ws = None
                                }
                            }
                        },
                        Some(Ok(Message::Close(c))) => {
                            debug!("received close frame. {c:?}");
                            open = false;
                        },
                        Some(Ok(Message::Ping(_) | Message::Pong(_))) => { },
                        Some(Ok(Message::Frame(_))) => { unreachable!() },
                        Some(Err(e)) => {
                            warn!("websocket threw an error. dropping connection. {e}");
                            ws = None
                        },
                        None => {
                            warn!("remote disconnected.");
                            ws = None
                        },
                    }
                },
                rq = request_settings_rx.recv(), if open => {
                    rq.unwrap();
                    match some_ws.send(Message::Binary(bincode::serialize(&CMsg::RequestSettings).unwrap())).await {
                        Ok(()) => { debug!("sent RequestSettings"); },
                        Err(e) => {
                            error!("unable to send message. message dropped. dropping connection. {e}");
                            ws = None;
                        },
                    }
                }
                item = upload_rx.recv(), if open => {
                    let item = item.unwrap();
                    debug!("sending {item:?}");
                    match bincode::serialize(&CMsg::Upload(item)) {
                        Ok(b) => match some_ws.send(Message::Binary(b)).await {
                            Ok(()) => { },
                            Err(e) => {
                                error!("unable to send message. message dropped. dropping connection. {e}");
                                ws = None;

                            },
                        }
                        Err(e) => error!("unable to serialize message. message dropped. {e}"),
                    }
                }
            }
        } else {
            (failure_cnt, open, ws) = connect_ws(failure_cnt).await;
        }
    }
}

async fn connect_ws(failure_cnt: usize) -> (usize, bool, Option<WebSocketStream<MaybeTlsStream<TcpStream>>>) {
    let url = {
        let mut url = crate::CONFIG.general.processor_url.clone();
        let name = &crate::CONFIG.general.name;
        let latitude = &crate::CONFIG.gps.latitude;
        let longitude = &crate::CONFIG.gps.longitude;
        url.set_query(Some(format!("name={name}&latitude={latitude}&longitude={longitude}").as_str()));
        url
    };
    match tokio_tungstenite::connect_async(&url).await {
        Ok((ws, _)) => {
            info!("Connected to {url}");
            (0, true, Some(ws))
        },
        Err(e) => {
            let failure_cnt = failure_cnt + 1;
            let delay = *CONNECTION_FAILURE_RETRY_SLEEP.get(failure_cnt - 1).unwrap_or(CONNECTION_FAILURE_RETRY_SLEEP.last().unwrap());
            warn!("Failed to connect to {url} {failure_cnt} times so far. retry in {delay}s. {e}");
            sleep(Duration::from_secs_f64(delay)).await;
            (failure_cnt, false, None)
        },
    }
}