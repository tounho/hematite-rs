use serde::{Serialize, Deserialize};

#[derive(Debug, Serialize, Deserialize)]
pub enum FileType {
    Dummy,
    Cr2,
}

impl FileType {
    pub fn ext(&self) -> String {
        match self {
            FileType::Dummy => format!("dummy"),
            FileType::Cr2 => format!("cr2"),
        }
    }

    pub fn dotext(&self) -> String {
        format!(".{ext}", ext = self.ext())
    }
}