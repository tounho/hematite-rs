pub mod settings;
mod filetype;

use size_format::SizeFormatterBinary;
use uuid::Uuid;
use chrono::{ DateTime, Local };

use serde::{ Serialize, Deserialize };

pub use filetype::FileType;

#[derive(Debug, Serialize, Deserialize)]
pub enum Message {
    RequestSettings,
    Upload(CaptureResult)
}

#[derive(Serialize, Deserialize)]
pub struct CaptureResult {
    pub uuid: Uuid,

    pub time: DateTime<Local>,
    pub is_night: bool,

    pub file_type: FileType,
    pub file: Vec<u8>,
}

impl std::fmt::Debug for CaptureResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CaptureResult")
            .field("uuid", &self.uuid)
            .field("time", &self.time)
            .field("is_night", &self.is_night)
            .field("file_type", &self.file_type)
            .field("file", &format!("Vec<u8> of length {}B", SizeFormatterBinary::new(self.file.len() as u64)))
            .finish()
    }
}