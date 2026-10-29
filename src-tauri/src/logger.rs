use serde::{Deserialize, Serialize};
use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tauri::Emitter;

const MAX_LOG_BYTES: u64 = 10 * 1024 * 1024;
const RETENTION: Duration = Duration::from_secs(7 * 24 * 60 * 60);

static LOG_FILE: OnceLock<Mutex<Option<File>>> = OnceLock::new();
static APP_HANDLE: OnceLock<tauri::AppHandle> = OnceLock::new();

pub fn init_app_handle(handle: tauri::AppHandle) {
    let _ = APP_HANDLE.set(handle);
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogEntry {
    pub timestamp: String,
    pub level: String,
    pub source: String,
    #[serde(default)]
    pub target: String,
    pub message: String,
}

#[derive(Debug, Clone, Copy)]
pub enum Level {
    Debug,
    Info,
    Warn,
    Error,
}

impl Level {
    pub fn as_str(self) -> &'static str {
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
    let ts = timestamp();
    let line = format!(
        "[{}] [{}] [{}] {}{}\n",
        ts,
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
            if let Some(handle) = APP_HANDLE.get() {
                let _ = handle.emit(
                    "log_entry",
                    LogEntry {
                        timestamp: ts,
                        level: level.as_str().to_string(),
                        source: source.to_string(),
                        target: source.to_string(),
                        message: format!("{}{}", message, suffix),
                    },
                );
            }
        }
    }
}

pub fn parse_log_line(line: &str) -> Option<LogEntry> {
    let line = line.trim();
    if !line.starts_with('[') {
        return None;
    }
    let mut parts = line.splitn(4, ']');
    let ts_part = parts.next()?.strip_prefix('[')?;
    let level_part = parts.next()?.trim().strip_prefix('[')?;
    let source_part = parts.next()?.trim().strip_prefix('[')?;
    let message_part = parts.next()?.trim();
    Some(LogEntry {
        timestamp: ts_part.to_string(),
        level: level_part.to_string(),
        source: source_part.to_string(),
        target: source_part.to_string(),
        message: message_part.to_string(),
    })
}

pub fn read_recent_logs(max_lines: usize) -> Vec<LogEntry> {
    let mut entries = Vec::new();
    let dir = log_dir();
    let mut files = Vec::new();
    for i in (1..=6).rev() {
        let p = dir.join(format!("application.{i}.log"));
        if p.exists() {
            files.push(p);
        }
    }
    let main_log = log_path();
    if main_log.exists() {
        files.push(main_log);
    }

    for file_path in files {
        if let Ok(content) = fs::read_to_string(&file_path) {
            for line in content.lines() {
                if let Some(entry) = parse_log_line(line) {
                    entries.push(entry);
                }
            }
        }
    }

    if entries.len() > max_lines {
        let start = entries.len() - max_lines;
        entries.drain(..start);
    }
    entries
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
