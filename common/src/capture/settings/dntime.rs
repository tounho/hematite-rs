use serde::{ Deserialize, Deserializer, Serialize };

#[derive(Copy, Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DNTime {
    #[serde(deserialize_with = "deserialize_frame")]
    pub frame: Frame,
    #[serde(deserialize_with = "deserialize_exposure")]
    pub exposure: Exposure,
    #[serde(deserialize_with = "deserialize_iso")]
    pub iso: Iso,
    #[serde(deserialize_with = "deserialize_apature")]
    pub aperture: Aperture,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum Frame {
    None,
    Some(f64),
}

fn deserialize_frame<'de, D>(d: D) -> Result<Frame, D::Error> where D: Deserializer<'de> {
    let value = Frame::deserialize(d)?;
    match value {
        Frame::None => Ok(value),
        Frame::Some(f) if f.is_finite() && f > 0.0 => Ok(value),
        Frame::Some(f) => Err(serde::de::Error::invalid_value(serde::de::Unexpected::Float(f), &"to be greater than 0"))
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum Exposure {
    Auto,
    Manual(f64),
}

fn deserialize_exposure<'de, D>(d: D) -> Result<Exposure, D::Error> where D: Deserializer<'de> {
    let value = Exposure::deserialize(d)?;
    match value {
        Exposure::Auto => Ok(value),
        Exposure::Manual(f) if f.is_finite() && f > 0.0 => Ok(value),
        Exposure::Manual(f) => Err(serde::de::Error::invalid_value(serde::de::Unexpected::Float(f), &"to be greater than 0"))
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum Iso {
    Auto,
    Manual(u32),
}

const VALID_ISO: &[u32] = &[100, 200, 400, 800, 1600];
fn deserialize_iso<'de, D>(d: D) -> Result<Iso, D::Error> where D: Deserializer<'de> {
    let value = Iso::deserialize(d)?;
    match value {
        Iso::Auto => Ok(value),
        Iso::Manual(u) if VALID_ISO.contains(&u) => Ok(value),
        Iso::Manual(u) => Err(serde::de::Error::invalid_value(serde::de::Unexpected::Unsigned(u as u64), &format!("to be one of {VALID_ISO:?}").as_str()))
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum Aperture {
    Auto,
    Implicit,
    Manual(f64),
}

fn deserialize_apature<'de, D>(d: D) -> Result<Aperture, D::Error> where D: Deserializer<'de> {
    let value = Aperture::deserialize(d)?;
    match value {
        Aperture::Auto => Ok(value),
        Aperture::Implicit => Ok(value),
        Aperture::Manual(f) if f.is_finite() => Ok(value),
        Aperture::Manual(f) => Err(serde::de::Error::invalid_value(serde::de::Unexpected::Float(f), &"to be valid"))
    }
}