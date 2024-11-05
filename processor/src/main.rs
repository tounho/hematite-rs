mod config;
mod logging;

use std::borrow::Cow;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::time::Duration;

use common::capture::settings::dntime::{DNTime, Frame, Exposure, Aperture, Iso};
use futures::future::pending;
use regex::Regex;
use size_format::SizeFormatterBinary;
use thiserror::Error;

use common::capture::{Message as CMsg, CaptureResult};
use common::processor::{Message as PMsg, CancelBehaviour};
use common::{ self, capture::settings::Settings };

use config::Config;

use futures::{ SinkExt, StreamExt };
use log::{ error, warn, info, debug };

use lazy_static::lazy_static;
use tokio::fs;
use tokio::net::{ TcpListener, TcpStream };
use tokio::time::{Instant, sleep};

use uuid::Uuid;
use chrono::{ DateTime, Local };


lazy_static!{
    static ref CONFIG: Config = Config::new();
}

#[tokio::main]
async fn main() {
    logging::init();

    error!("error");
    warn!("warn");
    info!("info");
    debug!("debug");
    
    let listener = TcpListener::bind(&CONFIG.general.socket.as_str()).await.unwrap();

    loop {
        if let Ok((stream, addr)) = listener.accept().await {
            debug!("received new connection from {addr}");
            tokio::spawn(accept_connection(stream, addr));
        }
    }
}

#[derive(Error, Debug)]
enum WebSocketError {
    #[error("handshake failed: {0}")]
    Handshake(tokio_tungstenite::tungstenite::Error),

    #[error("error upon next(): {0}")]
    Read(tokio_tungstenite::tungstenite::Error),

    #[error("error upon next(): {0}")]
    Write(tokio_tungstenite::tungstenite::Error),

    #[error("parse failed")]
    Parse(bincode::ErrorKind),

}

async fn accept_connection(stream: TcpStream, addr: SocketAddr) {
    if let Err(e) = handle_connection(stream, addr).await {
        match e {
            WebSocketError::Handshake(e) => warn!("handshake failed. {e}"),
            WebSocketError::Read(e) => warn!("read error. {e}"),
            WebSocketError::Write(e) => warn!("write error. {e}"),
            WebSocketError::Parse(e) => warn!("parse failed {e}"),
        }
    } else {
        debug!("done.");
    }
}

async fn handle_connection(stream: TcpStream, _addr: SocketAddr) -> Result<(), WebSocketError> {

    let mut name = format!("unknown");
    let mut coordinates = None;

    let callback = |req: &tokio_tungstenite::tungstenite::handshake::server::Request, response: tokio_tungstenite::tungstenite::handshake::server::Response| {
        debug!("query={query:?}", query = req.uri().query());
        let query = req.uri().query().unwrap_or("");

        let name_regex = Regex::new(r"name=(\w+)").unwrap();
        let latitude_regex = Regex::new(r"(?:(?:lat)|(?:latitude))=([+-]?(?:[0-9]*[.])?[0-9]+)").unwrap();
        let longitude_regex = Regex::new(r"(?:(?:lon)|(?:longitude))=([+-]?(?:[0-9]*[.])?[0-9]+)").unwrap();

        if let Some(c) = name_regex.captures(query) {
            name = c.get(1).unwrap().as_str().into();
            debug!("name={name}");
            
            if let (Some(lat), Some(lon))= (latitude_regex.captures(query), longitude_regex.captures(query)) {
                if let (Ok(lat), Ok(lon)) = (lat.get(1).unwrap().as_str().parse::<f64>(), lon.get(1).unwrap().as_str().parse::<f64>()) {
                    if lat.is_finite() && lat >= -90.0 && lat <= 90.0 && lon.is_finite() && lon >= -180.0 && lon <= 180.0 {
                        coordinates = Some((lat, lon));
                        debug!("(lat, lon)=({lat}, {lon})");
                    }
                }
            }
            Ok(response)
        } else {
            Err(tokio_tungstenite::tungstenite::http::Response::builder().status(401).body(None).unwrap())
        }
    };

    let mut ws = tokio_tungstenite::accept_hdr_async(stream, callback)
        .await
        .map_err(|e| { WebSocketError::Handshake(e) })?;


    let mut interval = tokio::time::interval(Duration::from_millis(2000));

    let timeout = sleep(Duration::from_millis(120000000));
    tokio::pin!(timeout);

    let mut open = true;

    let mut once = true;

    let settings = Settings {
        horizon: -0.67,
        daytime: DNTime { frame: Frame::Some(20.0), exposure: Exposure::Auto, iso: Iso::Manual(100), aperture: Aperture::Auto },
        nighttime:  DNTime { frame: Frame::None, exposure: Exposure::Manual(60.0 * 3.0), iso: Iso::Manual(800), aperture: Aperture::Manual(3.5) }
    };

    debug!("{name} connected");

    loop {
        tokio::select! {
            next = ws.next() => {
                match next {
                    Some(item) => {
                        match item.map_err(|e| { WebSocketError::Read(e) })? {
                            tokio_tungstenite::tungstenite::Message::Text(_) => { },
                            tokio_tungstenite::tungstenite::Message::Binary(b) => {
                                let total = b.len();
                                let msg: CMsg = bincode::deserialize(&b)
                                    .map_err(|e| { WebSocketError::Parse(*e) })?;
                                match msg {
                                    CMsg::RequestSettings => {                
                                        if open {
                                            let settings = settings.clone();
                                            ws.send(tokio_tungstenite::tungstenite::Message::Binary(bincode::serialize(
                                                &PMsg::SetSettings{ settings, cancel_behaviour: CancelBehaviour::Allways }
                                            ).unwrap()))
                                                .await
                                                .map_err(|e| { WebSocketError::Write(e) })?;
                                        }
                                    },
                                    CMsg::Upload(b) => {
                                        debug!("received Upload {b:?} total={total}B", total = SizeFormatterBinary::new(total as u64));
                                        let CaptureResult { uuid, time, is_night, file_type, file } = b;

                                        let filename = format!("{uuid}.{ext}", uuid = uuid.as_hyphenated().to_string(), ext = file_type.ext());
                                        let filepath = PathBuf::from(&CONFIG.general.tmp_path)
                                            .join(&filename);
                                        if let Err(e) = fs::write(&filepath, &file).await {
                                            error!("unable to write file. {e}");
                                        }
                                        if let Err(e) = scuffed_postprocesssing(filepath, uuid, time, is_night) {
                                            error!("Error processing image {e:?}")
                                        }
                                    }
                                }
                            },
                            tokio_tungstenite::tungstenite::Message::Ping(_) | tokio_tungstenite::tungstenite::Message::Pong(_) => { },
                            tokio_tungstenite::tungstenite::Message::Close(c) => {
                                debug!("received close frame {c:?}");
                                open = false;
                            },
                            tokio_tungstenite::tungstenite::Message::Frame(_) => unreachable!(),
                        }
                    },
                    None => { return Ok(()); },
                }
            },
            // _ = interval.tick() => {
            //     let settings = settings.clone();

            //     if open && once {
            //         once = false;
            //         debug!("sending settings unsolicited.");
            //         ws.send(tokio_tungstenite::tungstenite::Message::Binary(bincode::serialize(
            //             &PMsg::SetSettings{ settings, cancel_behaviour: CancelBehaviour::Allways }
            //         ).unwrap()))
            //             .await
            //             .map_err(|e| { WebSocketError::Write(e) })?;
            //     }
            // },
            // _ = async { if open { timeout.as_mut().await } else { pending().await } } => {
            //     open = false;
            //     ws.send(tokio_tungstenite::tungstenite::Message::Close(Some(tokio_tungstenite::tungstenite::protocol::CloseFrame {
            //         code: tokio_tungstenite::tungstenite::protocol::frame::coding::CloseCode::Normal ,
            //         reason: Cow::Owned(format!("bye!")) 
            //     }))).await.unwrap();
            // },
        }         
    }
}

fn scuffed_postprocesssing(filepath: PathBuf, uuid: Uuid, time: DateTime<Local>, is_night: bool) -> Result<(), Box<dyn std::error::Error>> {
    let start = Instant::now();
    
    let raw_filepath = filepath;
    let new_raw_filepath = {
        let mut buf = PathBuf::new();
        buf.push(format!("images-raws"));
        buf.push(format!("{ts}.cr2", ts = time.format("%Y-%m-%d %H:%M:%S")));
        buf
    };
    let jpg_filepath = {
        let mut buf = PathBuf::new();
        buf.push(format!("images"));
        buf.push(format!("{ts}.jpg", ts = time.format("%Y-%m-%d %H:%M:%S")));
        buf
    };

    let mut cmd = vec!["-Y"];
    cmd.push("-j90");

    cmd.push("-p"); cmd.push(if is_night { "profiles/nighttime.pp3" } else { "profiles/daytime.pp3" } );
    cmd.push("-o"); cmd.push(jpg_filepath.to_str().unwrap());
    cmd.push("-c"); cmd.push(raw_filepath.to_str().unwrap());

    if let Err(e) = std::process::Command::new("rawtherapee-cli")
        .args(&cmd)
        .output() {
        return Err(Box::new(e));
    }
    
    if !jpg_filepath.exists() {
        error!("no jpg.");
    }

    if let Err(e) = std::fs::copy(&raw_filepath, &new_raw_filepath) {
        error!("cannot copy raw to new location. {e}");
    }

    if let Err(e) = std::fs::remove_file(&raw_filepath) {
        error!("cannot delete raw. {e}");
    }

    Ok(())
}