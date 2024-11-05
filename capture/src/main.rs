mod config;
mod logging;

mod line;
mod tracking;
mod sun;
mod capture;

use chrono::{Local, DateTime};

use std::{sync::Mutex, time::Duration};
use tokio::time::sleep;
use log::{ error, warn, info, debug };

use lazy_static::lazy_static;

use common::{ self, capture::settings::Settings };
use config::Config;

use line::Line;
use tracking::Tracking;

use crate::capture::{CaptureCommand, CaptureError};

lazy_static!{
    static ref CONFIG: Config = Config::new();
    static ref SETTINGS: Mutex<Option<Settings>> = Mutex::new(None);
}

#[tokio::main(flavor = "current_thread")]
async fn main() {
    // initializing logger
    logging::init();

    // inittialize module
    let mut camera = capture::build();

    // initializing async websocket
    let mut line = Line::new();
    // get settings from remote. will block until SETTINGS is set to Some().
    // SETTINGS will allways be Some after this point.
    line.init_settings().await;
    assert!(SETTINGS.lock().unwrap().is_some(), "SETTNGS were None");

    // initialize tracking
    let mut tracking = match CONFIG.tracking.enable {
        true => Some(Tracking::new()),
        false => None,
    };
    if let Some(t) = tracking.as_mut() {
        t.start_homing().await;
    } 

    let mut is_night = {
        let timestamp_millis_now = Local::now().timestamp_millis();
        let is_night = sun::is_night(timestamp_millis_now);
        info!("using {} settings. sun is at {} while horizon is at {}",
            if is_night { "nighttime" } else { "daytime "},
            sun::altitude(timestamp_millis_now),
            SETTINGS.lock().unwrap().as_ref().unwrap().horizon
        );
        is_night
    };

    let mut now = Local::now();

    loop {
        update_is_night(now, &mut is_night);

        if let Some(t) = tracking.as_mut() {
            t.track().await;
        }

        match camera.capture(CaptureCommand {
            cancel_token: line.subscribe_settings(),
            time: now,
            is_night,
            settings: if is_night { SETTINGS.lock().unwrap().as_ref().unwrap().nighttime }
                else { SETTINGS.lock().unwrap().as_ref().unwrap().daytime }
        }).await {
            Ok(c) => {
                info!("capture complete!");
                line.upload(c).await;
                debug!("sent.");
            },
            Err(CaptureError::Cancelled) => info!("capture was cancelled."),
            Err(CaptureError::Module(e)) => error!("capture encountured an error. {e}"),
        }

        if let Some(t) = tracking.as_mut() {
            t.start_homing().await;
        }
        now = delay(is_night, now).await;
    }
}

fn update_is_night(time: DateTime<Local>, last_is_night: &mut bool) -> bool {
    debug!("updating is_night");
    let is_night = sun::is_night(time.timestamp_millis());
    if is_night != *last_is_night {
        info!("now using {} settings. sun is at {} while horizon is at {}",
            if is_night { "nighttime" } else { "daytime "},
            sun::altitude(time.timestamp_millis()),
            SETTINGS.lock().unwrap().as_ref().unwrap().horizon
            );
        *last_is_night = is_night;
        true
    } else { false }
}

async fn delay(is_night: bool, last: DateTime<Local>) -> DateTime<Local> {
    let frame = {
        if is_night { SETTINGS.lock().unwrap().as_ref().unwrap().nighttime.frame }
        else { SETTINGS.lock().unwrap().as_ref().unwrap().daytime.frame }
    };
    let now = Local::now();

    match frame {
        common::capture::settings::dntime::Frame::None => now,
        common::capture::settings::dntime::Frame::Some(f) => {
            let dif_sec = ((now - last).num_milliseconds() as f64) / 1000.0;
            if dif_sec > f {
                warn!("frame of {f:.1}s exceeded by {excess:.1}s", excess = dif_sec - f);
                now
            } else {
                let delay = f - dif_sec;
                debug!("delaying by {delay:.1}s");
                sleep(Duration::from_secs_f64(delay)).await;
                last + chrono::Duration::milliseconds((f * 1000.0) as i64)
            }
        },
    }
}