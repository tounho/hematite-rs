pub mod dntime;

use dntime::DNTime;
use serde::{ Serialize, Deserialize, Deserializer };

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Settings {
    #[serde(deserialize_with = "deserialize_horizon")]
    pub horizon: f64,

    pub daytime: DNTime,
    pub nighttime: DNTime,
}

fn deserialize_horizon<'de, D>(d: D) -> Result<f64, D::Error> where D: Deserializer<'de> {
    let value = f64::deserialize(d)?;
    if value.is_finite() && value >= -90.0 && value <= 90.0 { Ok(value) }
    else { Err(serde::de::Error::invalid_value(serde::de::Unexpected::Float(value), &"to be -90.0 <= x <= 90.0")) }
}