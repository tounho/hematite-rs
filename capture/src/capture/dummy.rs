use std::time::Duration;

use async_trait::async_trait;
use common::capture::FileType;
use log::{info, debug};
use thiserror::Error;
use tokio::time::sleep;
use uuid::Uuid;

use super::{ Capture, CaptureCommand, CaptureResult, CaptureError };

pub struct Dummy {

}

#[derive(Error, Debug)]
enum DummyError {
    #[error("dummy rolled a dice and decided to throw an error.")]
    Error
}

impl Dummy {
    pub fn new() -> Self {
        info!("new dummy created.");
        Dummy { }
    }
}

#[async_trait]
impl Capture for Dummy {
    async fn capture(&mut self, cmd: CaptureCommand) -> Result<CaptureResult, CaptureError> {
        debug!("dummy received a capture command {cmd:?}");
        tokio::select! {
            res = async {
                if rand::random::<f64>() > 0.9 {
                    Err(CaptureError::Module(Box::new(DummyError::Error)))
                } else {
                    match cmd.settings.exposure {
                        common::capture::settings::dntime::Exposure::Auto => {
                            sleep(Duration::from_secs_f64(0.1)).await;
                            debug!("ClickClack");
                        },
                        common::capture::settings::dntime::Exposure::Manual(f) => {
                            debug!("Click");
                            sleep(Duration::from_secs_f64(f)).await;
                            debug!("Clack");
                        },
                    }

                    let uuid = Uuid::new_v4();
                    let time = cmd.time;
                    let is_night = cmd.is_night;

                    Ok(CaptureResult { uuid, time, is_night, file_type: FileType::Dummy, file: vec![0, 0, 0] })
                }
            } => { res },
            () = cmd.cancel_token.notified() => { Err(CaptureError::Cancelled) },
        }
    }
}