use std::io::Write;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};

static EVENT_LOG_ENABLED: AtomicBool = AtomicBool::new(false);

/// Enable or disable Windows Event Log syncing globally.
pub fn set_event_log_enabled(enabled: bool) {
    EVENT_LOG_ENABLED.store(enabled, Ordering::Relaxed);
}

/// Check if Windows Event Log syncing is globally enabled.
pub fn is_event_log_enabled() -> bool {
    EVENT_LOG_ENABLED.load(Ordering::Relaxed)
}

/// Helper to resolve the standard AppData folder for diagnostics logging.
pub fn get_appdata_log_path() -> Option<PathBuf> {
    std::env::var("APPDATA").ok().map(|appdata| {
        std::path::PathBuf::from(appdata)
            .join("rWifi")
            .join("log.txt")
    })
}

/// Thread-safe silent logger helper that appends diagnostic logs to a local file.
pub fn log_message(level: &str, msg: &str) {
    if let Some(path) = get_appdata_log_path() {
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        if let Ok(mut file) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
        {
            let epoch = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0);
            let _ = writeln!(file, "[{}] [{}] {}", epoch, level, msg);
        }
    }

    if is_event_log_enabled() {
        let event_type = match level {
            "ERROR" | "PANIC" => 0x0001, // EVENTLOG_ERROR_TYPE
            "WARNING" => 0x0002,         // EVENTLOG_WARNING_TYPE
            _ => 0x0004,                 // EVENTLOG_INFORMATION_TYPE
        };
        rcommon::event_log::log_system_event("rwifi", event_type, 1000, msg);
    }
}
