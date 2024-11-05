mod dummy;
mod gphoto2;

use std::sync::Arc;

use async_trait::async_trait;
use chrono::{DateTime, Local};
use common::capture::settings::dntime::DNTime;
use thiserror::Error;
use tokio::sync::Notify;

use dummy::Dummy;
use crate::config::general::CaptureModule;

#[derive(Debug)]
pub struct CaptureCommand {
    pub cancel_token: Arc<Notify>,
    pub time: DateTime<Local>,
    pub is_night: bool,
    pub settings: DNTime,
}

pub use common::capture::CaptureResult;

use self::gphoto2::GPhoto2;


#[derive(Error, Debug)]
pub enum CaptureError {
    #[error("cancelled due to settings update.")]
    Cancelled,
    
    #[error("module encountered an error. {0}")]
    Module(Box<dyn std::error::Error>)
}

pub fn build() -> Box<dyn Capture> {
    match crate::CONFIG.general.module {
        CaptureModule::Dummy => Box::new(Dummy::new()),
        CaptureModule::GPhoto2 => Box::new(GPhoto2::new()),
    }
}

#[async_trait]
pub trait Capture {
    async fn capture(&mut self, cmd: CaptureCommand) -> Result<CaptureResult, CaptureError>;
}