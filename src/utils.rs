use std::time::{UNIX_EPOCH, SystemTime, Duration};
use std::path::Path;
use chrono::Local;
use chrono::format::{DelayedFormat, StrftimeItems};

pub fn systime() -> Duration {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("Fatal error occurred: System time moved backwards! Are you a time traveler?")
}

pub fn to_epoch(time: SystemTime) -> Duration {
    time.duration_since(UNIX_EPOCH)
        .expect("Fatal error occurred: System time moved backwards! Are you a time traveler?")
}

pub fn unwrap_or_default<T>(opt: Option<T>, default: T) -> T {
    match opt {
        Some(t) => t,
        None => default
    }
}