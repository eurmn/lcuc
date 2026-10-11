use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

const MAX_LOG_BYTES: u64 = 10 * 1024 * 1024;
const RETENTION: Duration = Duration::from_secs(7 * 24 * 60 * 60);

static LOG_FILE: OnceLock<Mutex<Option<File>>> = OnceLock::new();

#[derive(Debug, Clone, Copy)]
pub enum Level {
    Debug,
    Info,
    Warn,
    Error,
}

impl Level {
    fn as_str(self) -> &'static str {
        match self {
            Self::Debug => "DEBUG",
            Self::Info => "INFO",
            Self::Warn => "WARN",
            Self::Error => "ERROR",
        }
    }
}

fn data_dir() -> PathBuf {
    dirs::data_local_dir()
        .or_else(dirs::home_dir)
        .unwrap_or_else(|| PathBuf::from("."))
        .join("lcu-companion")
}

fn log_dir() -> PathBuf {
    data_dir().join("logs")
}

fn log_path() -> PathBuf {
    log_dir().join("application.log")
}

fn timestamp() -> String {
    let total_seconds = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let milliseconds = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .subsec_millis();
    let days = total_seconds / 86_400;
    let seconds = total_seconds % 86_400;
    let (year, month, day) = civil_date(days as i64);
    format!(
        "{year:04}-{month:02}-{day:02}T{:02}:{:02}:{:02}.{milliseconds:03}Z",
        seconds / 3600,
        (seconds % 3600) / 60,
        seconds % 60
    )
}

fn civil_date(days_since_epoch: i64) -> (i64, i64, i64) {
    let shifted = days_since_epoch + 719_468;
    let era = (if shifted >= 0 {
        shifted
    } else {
        shifted - 146_096
    }) / 146_097;
    let day_of_era = shifted - era * 146_097;
    let year_of_era =
        (day_of_era - day_of_era / 1_460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let year = year_of_era + era * 400;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let month_part = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * month_part + 2) / 5 + 1;
    let month = month_part + if month_part < 10 { 3 } else { -9 };
    (year + if month <= 2 { 1 } else { 0 }, month, day)
}

fn cleanup_old_logs() {
    let now = SystemTime::now();
    let Ok(entries) = fs::read_dir(log_dir()) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_file() || path.extension().and_then(|value| value.to_str()) != Some("log") {
            continue;
        }
        if path == log_path() {
            continue;
        }
        let expired = entry
            .metadata()
            .and_then(|metadata| metadata.modified())
            .ok()
            .and_then(|modified| now.duration_since(modified).ok())
            .map(|age| age > RETENTION)
            .unwrap_or(false);
        if expired {
            let _ = fs::remove_file(path);
        }
    }
}

fn rotate_if_needed(file: &mut File) {
    if file.metadata().map(|metadata| metadata.len()).unwrap_or(0) < MAX_LOG_BYTES {
        return;
    }
    let path = log_path();
    let _ = file.flush();
    for index in (1..=6).rev() {
        let current = log_dir().join(format!("application.{index}.log"));
        let next = log_dir().join(format!("application.{}.log", index + 1));
        let _ = fs::remove_file(&next);
        let _ = fs::rename(current, next);
    }
    let _ = fs::rename(path, log_dir().join("application.1.log"));
    if let Ok(new_file) = OpenOptions::new()
        .create(true)
        .append(true)
        .open(log_path())
    {
        *file = new_file;
    }
}

fn redact(value: &str) -> String {
    let mut redacted = value.to_string();
    for marker in [
        "--remoting-auth-token=",
        "Authorization: Basic ",
        "\"token\":\"",
        "\"password\":\"",
    ] {
        while let Some(start) = redacted.find(marker) {
            let value_start = start + marker.len();
            let value_end = redacted[value_start..]
                .find(|character: char| matches!(character, ' ' | '"' | ',' | '\n' | '\r'))
                .map(|offset| value_start + offset)
                .unwrap_or(redacted.len());
            redacted.replace_range(value_start..value_end, "[REDACTED]");
        }
    }
    redacted
}

fn file_sink() -> Option<&'static Mutex<Option<File>>> {
    Some(LOG_FILE.get_or_init(|| {
        let _ = fs::create_dir_all(log_dir());
        cleanup_old_logs();
        Mutex::new(
            OpenOptions::new()
                .create(true)
                .append(true)
                .open(log_path())
                .ok(),
        )
    }))
}

pub fn write(level: Level, source: &str, message: &str, context: Option<&str>) {
    let message = redact(message);
    let suffix = context
        .filter(|value| !value.is_empty())
        .map(|value| format!(" | {}", redact(value)))
        .unwrap_or_default();
    let line = format!(
        "[{}] [{}] [{}] {}{}\n",
        timestamp(),
        level.as_str(),
        source,
        message,
        suffix
    );

    match level {
        Level::Debug if !cfg!(debug_assertions) => {}
        _ => {
            match level {
                Level::Debug | Level::Info => eprint!("{line}"),
                Level::Warn | Level::Error => eprint!("{line}"),
            }
            if let Some(sink) = file_sink() {
                if let Ok(mut guard) = sink.lock() {
                    if let Some(file) = guard.as_mut() {
                        rotate_if_needed(file);
                        let _ = file.write_all(line.as_bytes());
                        let _ = file.flush();
                    }
                }
            }
        }
    }
}

pub fn debug(source: &str, message: &str) {
    write(Level::Debug, source, message, None);
}

pub fn info(source: &str, message: &str) {
    write(Level::Info, source, message, None);
}

pub fn warn(source: &str, message: &str) {
    write(Level::Warn, source, message, None);
}

pub fn error(source: &str, message: &str) {
    write(Level::Error, source, message, None);
}

pub fn with_context(level: Level, source: &str, message: &str, context: &str) {
    write(level, source, message, Some(context));
}
