pub fn altitude(unixtime_in_ms: i64) -> f64 {
    let (latitude, longitude) = (crate::CONFIG.gps.latitude, crate::CONFIG.gps.longitude);
    altitude_from_args(unixtime_in_ms, latitude, longitude)
}

pub fn is_night(unixtime_in_ms: i64) -> bool {
    let (latitude, longitude) = (crate::CONFIG.gps.latitude, crate::CONFIG.gps.longitude);
    let horizon = crate::SETTINGS.lock().unwrap().as_ref().unwrap().horizon;

    altitude_from_args(unixtime_in_ms, latitude, longitude) < horizon
}

fn altitude_from_args(unixtime_in_ms: i64, latitude: f64, longitude: f64) -> f64 {
    let pos = sun::pos(unixtime_in_ms, latitude, longitude);
    pos.altitude.to_degrees()
}