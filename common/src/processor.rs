use crate::capture::settings::Settings;

use serde::{ Serialize, Deserialize };

#[derive(Debug, Serialize, Deserialize)]
pub enum Message {
    SetSettings { settings: Settings, cancel_behaviour: CancelBehaviour },
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum CancelBehaviour {
    Allways,
    IfUnequal,
    Never,
}