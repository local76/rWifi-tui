#![allow(dead_code)]
use std::io;
use std::path::PathBuf;

/// Application configuration structure.
#[derive(Debug, Clone)]
pub struct AppConfig {
    /// Forces theme: "auto" (DWM sync), "dark", or "light"
    pub theme_mode: String,
    /// TUI event loop refresh frequency in milliseconds.
    pub refresh_rate_ms: u32,
    /// Whether to strip console window decorations.
    pub enable_borderless: bool,
    /// Whether to enable native Windows toast notifications.
    pub enable_toasts: bool,
    /// Whether to enable Windows Event Log syncing.
    pub enable_event_log: bool,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            theme_mode: "auto".to_string(),
            refresh_rate_ms: 100,
            enable_borderless: false,
            enable_toasts: true,
            enable_event_log: true,
        }
    }
}

impl AppConfig {
    /// Resolves path to `%APPDATA%\rWifi\config.yaml`
    pub fn config_path() -> Option<PathBuf> {
        std::env::var("APPDATA").ok().map(|appdata| {
            std::path::PathBuf::from(appdata)
                .join("rWifi-tui")
                .join("config.yaml")
        })
    }

    /// Load config from file, fallback to default if missing or invalid.
    pub fn load() -> Self {
        let Some(path) = Self::config_path() else {
            return Self::default();
        };
        let Ok(content) = std::fs::read_to_string(&path) else {
            return Self::default();
        };

        let mut config = Self::default();
        for line in content.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }

            if let Some(pos) = line.find(':') {
                let key = line[..pos].trim();
                let val = line[pos + 1..].trim();
                match key {
                    "theme_mode" => {
                        config.theme_mode = val.to_string();
                    }
                    "refresh_rate_ms" => {
                        if let Ok(ms) = val.parse::<u32>() {
                            config.refresh_rate_ms = ms;
                        }
                    }
                    "enable_borderless" => {
                        config.enable_borderless = val == "true";
                    }
                    "enable_toasts" => {
                        config.enable_toasts = val == "true";
                    }
                    "enable_event_log" => {
                        config.enable_event_log = val == "true";
                    }
                    _ => {}
                }
            }
        }
        config
    }

    /// Save current config properties to file.
    pub fn save(&self) -> io::Result<()> {
        let Some(path) = Self::config_path() else {
            return Ok(());
        };
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let content = format!(
            "# rWifi Local Configuration\n\
			 # -------------------------\n\n\
			 theme_mode: {}\n\
			 refresh_rate_ms: {}\n\
			 enable_borderless: {}\n\
			 enable_toasts: {}\n\
			 enable_event_log: {}\n",
            self.theme_mode,
            self.refresh_rate_ms,
            self.enable_borderless,
            self.enable_toasts,
            self.enable_event_log,
        );
        std::fs::write(path, content)
    }
}
