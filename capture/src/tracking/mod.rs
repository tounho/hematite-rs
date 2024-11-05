mod driver;

use std::sync::Arc;

use log::{debug, error};
use tokio::{runtime::Runtime, task::JoinHandle, sync::{mpsc::{Sender, self, Receiver}, Notify}, time::{MissedTickBehavior, interval}};

use driver::Driver;

use crate::tracking::driver::Direction;

pub struct Tracking {
    _rt: Runtime,
    _handle: JoinHandle<()>,

    tx: Sender<Mode>,
    ack: Arc<Notify>,
}

#[derive(Debug)]
enum Mode {
    Standby,
    Home,
    LC,
    Track,
}

impl Tracking {
    pub fn new() -> Self {
        let (tx, rx) = mpsc::channel(1);
        let ack = Arc::new(Notify::new());

        let driver = match Driver::new() {
            Ok(d) => d,
            Err(e) => {
                error!("Unable to initialize driver. {e}");
                panic!();
            },
        };

        let rt = tokio::runtime::Builder::new_multi_thread()
            .thread_name("tracking-rt")
            .worker_threads(1)
            .enable_all()
            .build()
            .unwrap();

        let handle: JoinHandle<()> = {
            let ack = ack.clone();

            rt.spawn( async move {
                worker(ack, rx, driver).await;
            })
        };

        Tracking { _rt: rt, _handle: handle, tx, ack }
    }

    pub async fn start_homing(&mut self) {
        self.tx.send(Mode::Home).await.unwrap();
    }

    pub async fn track(&mut self) {
        if crate::CONFIG.tracking.leeway_compensation > 0 {
            self.tx.send(Mode::LC).await.unwrap();
        } else {
            self.tx.send(Mode::Track).await.unwrap();
        }
        self.ack.notified().await;
    }
}

async fn worker(ack: Arc<Notify>, mut rx: Receiver<Mode>, mut driver: Driver) {
    let mut mode = Mode::Standby;

    let mut tracking_timer = interval(crate::CONFIG.tracking.tracking_speed);
    tracking_timer.set_missed_tick_behavior(MissedTickBehavior::Burst);

    driver.enable();

    loop {
        match mode {
            Mode::Standby => {
                debug!("standby");
                match rx.recv().await.unwrap() {
                    Mode::Standby => { },
                    Mode::Home => { mode = Mode::Home; },
                    Mode::LC => { mode = Mode::LC; },
                    Mode::Track => { mode = Mode::Track; },
                }
            },
            Mode::Home => {
                debug!("homing...");
                driver.goto(0);
                mode = Mode::Standby;
            },
            Mode::LC => {
                if driver.pos() < crate::CONFIG.tracking.leeway_compensation {
                    debug!("now leeway compensating");
                    driver.goto(crate::CONFIG.tracking.leeway_compensation);
                }
                mode = Mode::Track;
            },
            Mode::Track => {
                debug!("tracking...");
                tracking_timer.reset();
                ack.notify_one();
                'tracking: loop {
                    tokio::select! {
                        _ = tracking_timer.tick() => {
                            driver.step(Direction::Track);
                        },
                        msg = rx.recv() => {
                            let msg = msg.unwrap();
                            match msg {
                                Mode::Standby => mode = Mode::Standby,
                                Mode::Home => { mode = Mode::Home; },
                                Mode::LC => { },
                                Mode::Track => { },
                            }
                            break 'tracking;
                        }
                    }
                }
            },
        }
    }
}