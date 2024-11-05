mod general;
mod logging;

use serde::Deserialize;
use config_rs;

use general::General;
use logging::Logging;

const CONFIGS: &[&str] = &["test.toml", "nonexistant.toml"];

#[derive(Debug, Deserialize)]
pub struct Config {
    pub general: General,
    pub logging: Logging,
}

impl Config {
    pub fn new() -> Config {
        let mut config_rs_builder = config_rs::Config::builder()
            .add_source(config_rs::File::from_str(include_str!("defaults/general.toml"), config_rs::FileFormat::Toml))
            .add_source(config_rs::File::from_str(include_str!("defaults/logging.toml"), config_rs::FileFormat::Toml))
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