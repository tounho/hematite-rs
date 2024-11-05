use std::path::Path;

use log4rs::{
    self,
    append::{ console::{ConsoleAppender, Target}, rolling_file::{ policy::compound::{ roll::fixed_window::FixedWindowRoller, trigger::size::SizeTrigger, CompoundPolicy, }, RollingFileAppender, }, },
    config::{Appender, Root},
    encode::pattern::PatternEncoder
};

type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send>>;


pub fn init() {
    let log_cnf = generate_config();
    log4rs::init_config(log_cnf.unwrap()).unwrap();
}

fn generate_config() -> Result<log4rs::Config> {
    let level = crate::CONFIG.logging.level;
    let path = crate::CONFIG.logging.path.clone();
    let size = crate::CONFIG.logging.size.checked_mul(1024 * 1024).unwrap_or(u64::MAX);
    let count = crate::CONFIG.logging.count;
    let pattern = "{d(%Y-%m-%d %H:%M:%S)} {h({l}):5} {t} {T} - {m}{n}";

    let console_appender = ConsoleAppender::builder()
        .target(Target::Stdout)
        .encoder(Box::new(PatternEncoder::new(&pattern)))
        .build();

    let rolling_file_appender = RollingFileAppender::builder()
        .encoder(Box::new(PatternEncoder::new(&pattern)))
        .build(
            Path::new(&path)
                .join("hematite-capture.log")
                .to_str()
                .unwrap()
                .to_string(),
            Box::new(CompoundPolicy::new(
                Box::new(SizeTrigger::new(size)),
                Box::new(
                    FixedWindowRoller::builder()
                        .build(
                            Path::new(&path)
                                .join("{}.log")
                                .to_str()
                                .unwrap()
                                .to_string()
                                .as_str(),
                            count,
                        )
                        ?,
                ),
            )),
        ).map_err(|e| -> Box<dyn std::error::Error + Send> { Box::new(e) } )?;

    let log_cnf = log4rs::Config::builder()
        .appender(
            Appender::builder().build("rolling_file_appender", Box::new(rolling_file_appender)),
        )
        .appender(Appender::builder().build("console_appender", Box::new(console_appender)))
        .build(
            Root::builder()
                .appender("rolling_file_appender")
                .appender("console_appender")
                .build(level),
        ).map_err(|e| -> Box<dyn std::error::Error + Send> { Box::new(e) } )?;

    Ok(log_cnf)
}
