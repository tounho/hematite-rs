use std::path::PathBuf;

use serde::{ Deserialize, Deserializer, de::Unexpected };
use url::Url;

#[derive(Debug, Deserialize)]
pub struct General {
    #[serde(deserialize_with = "deserialize_socket")]
    pub socket: Url,

    pub tmp_path: PathBuf,

    #[serde(deserialize_with = "deserialize_queue")]
    pub queue: usize,
}

fn deserialize_socket<'de, D>(d: D) -> Result<Url, D::Error> where D: Deserializer<'de> {
    let s = String::deserialize(d)?;
    match Url::parse(&s) {
        Ok(u) => Ok(u),
        Err(e) => Err(serde::de::Error::invalid_value(Unexpected::Str(&s), &format!("to be valid url. (general.socket) {e}").as_str())),
    }
}

fn deserialize_queue<'de, D>(d: D) -> Result<usize, D::Error> where D: Deserializer<'de> {
    let s = String::deserialize(d)?;
    match s.parse::<usize>() {
        Ok(u) => Ok(u),
        Err(e) => Err(serde::de::Error::invalid_value(Unexpected::Str(&s), &format!("to be an usize. (general.queue) {e}").as_str())),
    }
}