use std::time::Duration;

use serde::{ Deserialize, Deserializer, de::Unexpected };

#[derive(Debug, Deserialize)]
pub struct Tracking {
    pub enable: bool,

    pub enn_pin: u8,
    pub step_pin: u8,
    pub dir_pin: u8,

    #[serde(deserialize_with = "deserialize_leeway_compensation")]
    pub leeway_compensation: isize,
    
    #[serde(deserialize_with = "deserialize_direction", alias = "tracking_direction")]
    pub tracking_direction: u8,

    #[serde(deserialize_with = "deserialize_tracking_speed")]
    pub tracking_speed: Duration,
    #[serde(deserialize_with = "deserialize_cycle")]
    pub cycle: Duration,
}

fn deserialize_leeway_compensation<'de, D>(d: D) -> Result<isize, D::Error> where D: Deserializer<'de> {
    let value = isize::deserialize(d)?;
    if value >= 0 { Ok(value) }
    else { Err(serde::de::Error::invalid_value(Unexpected::Unsigned(value as u64), &"greater then or equal 0")) }
}

fn deserialize_direction<'de, D>(d: D) -> Result<u8, D::Error> where D: Deserializer<'de> {
    let value = u8::deserialize(d)?;
    if value == 0 || value == 1 { Ok(value) }
    else { Err(serde::de::Error::invalid_value(Unexpected::Unsigned(value as u64), &"to be either 1 or 0")) }
}

fn deserialize_tracking_speed<'de, D>(d: D) -> Result<Duration, D::Error> where D: Deserializer<'de> {
    let value = f64::deserialize(d)?;
    if value.is_finite() && value > 0.0 { Ok(Duration::from_secs_f64(1.0 / value)) }
    else { Err(serde::de::Error::invalid_value(Unexpected::Float(value), &"greater than zero")) }
}

fn deserialize_cycle<'de, D>(d: D) -> Result<Duration, D::Error> where D: Deserializer<'de> {
    let value = f64::deserialize(d)?;
    if value.is_finite() && value > 0.0 { Ok(Duration::from_secs_f64(1.0 / value)) }
    else { Err(serde::de::Error::invalid_value(Unexpected::Float(value), &"greater than zero")) }
}