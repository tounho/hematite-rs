pub mod general;
pub mod logging;
pub mod gps;
pub mod tracking;

use serde::Deserialize;
use config_rs;

use general::General;
use logging::Logging;
use gps::GPS;
use tracking::Tracking;

const CONFIGS: &[&str] = &["capture.toml", "nonexistant.toml"];

#[derive(Debug, Deserialize)]
pub struct Config {
    pub general: General,
    pub logging: Logging,
    pub gps: GPS,
    pub tracking: Tracking,
}

impl Config {
    pub fn new() -> Config {
        let mut config_rs_builder = config_rs::Config::builder()
        .add_source(config_rs::File::from_str(include_str!("defaults/general.toml"), config_rs::FileFormat::Toml))
            .add_source(config_rs::File::from_str(include_str!("defaults/logging.toml"), config_rs::FileFormat::Toml))
            .add_source(config_rs::File::from_str(include_str!("defaults/gps.toml"), config_rs::FileFormat::Toml))
            .add_source(config_rs::File::from_str(include_str!("defaults/tracking.toml"), config_rs::FileFormat::Toml))
            ;
        for s in CONFIGS {
            config_rs_builder = config_rs_builder.add_source(config_rs::File::with_name(s).required(false));
        }
        match config_rs_builder.build() {
            Ok(c) => {
                match c.try_deserialize::<Config>() {
                    Ok(c) => c,
                    Err(e) => panic!("deserializing config failed. {e}"),
                }
            },
            Err(e) => panic!("building config failed. {e}"),
        }
    }
}