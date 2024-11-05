use std::path::PathBuf;

use serde::{ Deserialize, Deserializer, de::Unexpected };
use url::Url;

#[derive(Debug, Deserialize)]
pub struct General {
    #[serde(deserialize_with = "deserialize_name")]
    pub name: String,

    pub module: CaptureModule,
    
    #[serde(deserialize_with = "deserialize_processor_url")]
    pub processor_url: Url,

    pub tmp_path: PathBuf,

    #[serde(deserialize_with = "deserialize_queue")]
    pub queue: usize,
}

#[derive(Debug, PartialEq, Clone, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CaptureModule {
    Dummy,
    GPhoto2,
}

fn deserialize_name<'de, D>(d: D) -> Result<String, D::Error> where D: Deserializer<'de> {
    let s = String::deserialize(d)?;
    Ok(s)
}

fn deserialize_processor_url<'de, D>(d: D) -> Result<Url, D::Error> where D: Deserializer<'de> {
    let s = String::deserialize(d)?;
    match Url::parse(&s) {
        Ok(u) => Ok(u),
        Err(e) => Err(serde::de::Error::invalid_value(Unexpected::Str(&s), &format!("to be valid url. (general.processor_url) {e}").as_str())),
    }
}

fn deserialize_queue<'de, D>(d: D) -> Result<usize, D::Error> where D: Deserializer<'de> {
    let s = String::deserialize(d)?;
    match s.parse::<usize>() {
        Ok(u) => Ok(u),
        Err(e) => Err(serde::de::Error::invalid_value(Unexpected::Str(&s), &format!("to be an usize. (general.queue) {e}").as_str())),
    }
}