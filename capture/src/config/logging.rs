use log::LevelFilter;
use serde::{ Deserialize, Deserializer, de::Unexpected };

#[derive(Debug, Deserialize)]
pub struct Logging {
    #[serde(deserialize_with = "deserialize_level")]
    pub level: LevelFilter,
    pub path: String,
    #[serde(deserialize_with = "deserialize_size")]
    pub size: u64,
    #[serde(deserialize_with = "deserialize_count")]
    pub count: u32,
}

fn deserialize_level<'de, D>(d: D) -> Result<LevelFilter, D::Error> where D: Deserializer<'de> {
    match String::deserialize(d)?.as_str() {
        "error" => Ok(LevelFilter::Error),
        "warn" => Ok(LevelFilter::Warn),
        "info" => Ok(LevelFilter::Info),
        "debug" => Ok(LevelFilter::Debug),
        "trace" => Ok(LevelFilter::Trace),
        invalid => Err(serde::de::Error::invalid_value(Unexpected::Str(invalid), &"error, warn, info or debug")),
    }
}

fn deserialize_size<'de, D>(d: D) -> Result<u64, D::Error> where D: Deserializer<'de> {
    let s = String::deserialize(d)?;
    match s.parse::<u64>() {
        Ok(v) if v > 0 => Ok(v),
        Ok(v) => Err(serde::de::Error::invalid_value(Unexpected::Unsigned(v), &"to be greater than zero (logging.size)")),
        Err(e) => Err(serde::de::Error::invalid_value(Unexpected::Str(&s), &format!("to a u64. (logging.size) {e}").as_str())),
    }
}

fn deserialize_count<'de, D>(d: D) -> Result<u32, D::Error> where D: Deserializer<'de> {
    let s = String::deserialize(d)?;
    match s.parse::<u32>() {
        Ok(v) if v > 0 => Ok(v),
        Ok(v) => Err(serde::de::Error::invalid_value(Unexpected::Unsigned(v as u64), &"to be greater than zero. (logging.count)")),
        Err(e) => Err(serde::de::Error::invalid_value(Unexpected::Str(&s), &format!("to a u32. (logging.count) {e}").as_str())),
    }
}