use serde::{ Deserialize, Deserializer };

#[derive(Debug, Deserialize)]
pub struct GPS {
    #[serde(deserialize_with = "deserialize_latitude")]
    pub latitude: f64,

    #[serde(deserialize_with = "deserialize_longitude")]
    pub longitude: f64,
}

fn deserialize_latitude<'de, D>(d: D) -> Result<f64, D::Error> where D: Deserializer<'de> {
    let value = f64::deserialize(d)?;
    if value.is_finite() && value >= -90.0 && value <= 90.0 { Ok(value) }
    else { Err(serde::de::Error::invalid_value(serde::de::Unexpected::Float(value), &"to be -90.0 <= x <= 90.0")) }
}

fn deserialize_longitude<'de, D>(d: D) -> Result<f64, D::Error> where D: Deserializer<'de> {
    let value = f64::deserialize(d)?;
    if value.is_finite() && value >= -180.0 && value <= 180.0 { Ok(value) }
    else { Err(serde::de::Error::invalid_value(serde::de::Unexpected::Float(value), &"to be -180.0 <= x <= 180.0")) }
}