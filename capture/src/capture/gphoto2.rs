use std::{path::PathBuf, sync::Arc, process::Stdio};

use async_trait::async_trait;
use chrono::{Local, DateTime};
use common::capture::{settings::dntime::{DNTime, Exposure, Iso}, FileType};
use log::{info, debug, error};
use thiserror::Error;
use tokio::{time::Instant, fs, process::Command, sync::Notify};
use uuid::Uuid;

use super::{ Capture, CaptureCommand, CaptureResult, CaptureError };

pub struct GPhoto2 {

}

#[derive(Error, Debug)]
enum GPhoto2Error {
    #[error("IO Error. {0}")]
    IO(std::io::Error),

    #[error("Process IO Error. {0}")]
    Process(std::io::Error),
    
    #[error("GPhoto2 exit with a non zero exit code")]
    GPhoto2,
}

impl GPhoto2 {
    pub fn new() -> Self {
        GPhoto2 { }
    }
}

#[async_trait]
impl Capture for GPhoto2 {
    async fn capture(&mut self, cmd: CaptureCommand) -> Result<CaptureResult, CaptureError> {
        let CaptureCommand { cancel_token, time, is_night, settings } = cmd;
        do_capture(cancel_token, time, is_night, settings).await
    }
}

async fn do_capture(cancel_token: Arc<Notify>, time: DateTime<Local>, is_night: bool, settings: DNTime) -> Result<CaptureResult, CaptureError> {
    debug!("capturing...");
    let start = Instant::now();
    
    let uuid = Uuid::new_v4();

    let tmp_path = &crate::CONFIG.general.tmp_path;
    let filename = format!("capture.cr2");
    let filepath = PathBuf::from(tmp_path).join(&filename);

    fs::create_dir_all(tmp_path).await
        .map_err(|e| CaptureError::Module(Box::new(GPhoto2Error::IO(e))) )?;

    let mut args = vec![];
    args.push(format!("--force-overwrite"));
    args.push(format!("--filename")); args.push(filename.clone());

    args.append(&mut generate_args(&settings));

    //debug!("running command: \"gphoto2 {}\"", args.join(" "));

    let mut child = Command::new("gphoto2")
        .current_dir(&tmp_path)
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .spawn()
        .map_err(|e| CaptureError::Module(Box::new(GPhoto2Error::Process(e))) )?;

    tokio::select! {
        exit = child.wait() => {
            let code = exit.map_err(|e| CaptureError::Module(Box::new(GPhoto2Error::Process(e))) )?
                .code();
            if code.is_none() || code.unwrap() != 0 {
                error!("gphoto2 did exit with exit code: {}", if code.is_none() { format!("none") } else { code.unwrap().to_string() });
                return Err(CaptureError::Module(Box::new(GPhoto2Error::GPhoto2)));
            }
        },
        () = cancel_token.notified() => {
            if let Some(id) = child.id() {
                debug!("sending SIGTERM to gphoto2 {id}");
                Command::new("kill")
                    .arg(format!("-SIGTERM"))
                    .arg(format!("{id}"))
                    .stdin(Stdio::null())
                    .stdout(Stdio::null())
                    .output()
                    .await
                    .map_err(|e| CaptureError::Module(Box::new(GPhoto2Error::Process(e))) )?;
                child.wait().await
                    .map_err(|e| CaptureError::Module(Box::new(GPhoto2Error::Process(e))) )?;
                return Err(CaptureError::Cancelled);
            } else {
                return Err(CaptureError::Cancelled);
            }
        }
    }

    let file = fs::read(&filepath).await
        .map_err(|e| CaptureError::Module(Box::new(GPhoto2Error::IO(e))) )?;

    fs::remove_file(&filepath).await
        .map_err(|e| CaptureError::Module(Box::new(GPhoto2Error::IO(e))) )?;

    info!("capture complete after {:.1} seconds.", start.elapsed().as_secs_f64());
    
    Ok(CaptureResult { uuid, time, is_night, file_type: FileType::Cr2, file })
}

fn generate_args(settings: &DNTime) -> Vec<String> {
    let mut args = vec![];
    let mut mode_args = vec![];
    let mut capture_args = vec![];

    let DNTime { frame: _, exposure, iso, aperture } = settings;

    // TODO: Aperture
    
    match (exposure, aperture) {
        (Exposure::Manual(f), _) => {
            mode_args.push(format!("--set-config-value")); mode_args.push(format!("autoexposuremode=Manual"));
            capture_args.push(String::from("--set-config")); capture_args.push(String::from("bulb=1"));
            capture_args.push(format!("--wait-event={}ms", f * 1000.0));
            capture_args.push(String::from("--set-config")); capture_args.push(String::from("bulb=0"));
            capture_args.push(String::from("--wait-event-and-download=2s"));
        },
        (Exposure::Auto, _) => {
            mode_args.push(format!("--set-config-value")); mode_args.push(format!("autoexposuremode=AV"));
            capture_args.push(String::from("--capture-image-and-download"));
        },
    }

    args.append(&mut mode_args);

    args.push(format!("--set-config-value")); args.push(format!("imageformat=RAW", ));

    match iso {
        Iso::Auto => { args.push(format!("--set-config-value")); args.push(format!("iso=auto", )); },
        Iso::Manual(u) => { args.push(format!("--set-config-value")); args.push(format!("iso={u}", )); },
    }

    args.append(&mut capture_args);

    args
}