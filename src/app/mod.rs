//! Application state structures and state transitions.
//!
//! **Taxonomy Classification**: Business Logic (AppState).

use std::time::{Duration, Instant};
use ratatui::text::Line;
use library::interface::tui::widgets::TextBox;
use library::lifecycle::background::file_log::log_message;
use crate::win32::{self, WlanNetwork, GlyphMap};

pub mod keys;
pub mod keys_overlays;
pub mod mouse;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FocusedSection {
    NetworkList,
    InfoPanel,
}

pub struct AppState {
    pub networks: Vec<WlanNetwork>,
    pub selected_network_idx: usize,
    pub focus: FocusedSection,
    
    pub password_box: TextBox,
    pub show_password_overlay: bool,
    pub password_visible: bool,
    
    pub search_box: TextBox,
    pub search_active: bool,

    // EAP Enterprise sequential prompt fields
    pub eap_username: String,
    pub eap_prompt_username: bool,

    // Hidden SSID Connection overlay fields
    pub hidden_ssid: String,
    pub hidden_secured: bool,
    pub hidden_prompt_step: u8, // 0: idle, 1: SSID, 2: Security (y/n), 3: Password
    pub show_hidden_overlay: bool,
    pub hidden_box: TextBox,

    // Share Wifi Network QR Code overlay fields
    pub show_share_overlay: bool,
    pub share_ssid: String,
    pub share_auth: String,
    pub share_qr_lines: Vec<String>,
    pub share_box: TextBox,
    pub share_prompt_password: bool,

    // Manage Offline Known Networks (Profiles) overlay fields
    pub show_profiles_overlay: bool,
    pub profiles_list: Vec<(String, windows_sys::core::GUID)>,
    pub profiles_selected_idx: usize,
    
    pub status_msg: String,
    pub status_ttl: u32,
    pub should_quit: bool,
    
    pub glyphs: GlyphMap,
    pub on_battery: bool,
    pub last_power_check: Instant,
    pub last_scan: Instant,
    
    pub show_help: bool,
    pub show_markdown: Option<String>,
    pub markdown_lines: Vec<Line<'static>>,
    pub markdown_scroll: usize,
    
    // Mouse Selection Info
    pub selection_start: Option<(u16, u16)>,
    pub selection_end: Option<(u16, u16)>,
    pub selection_pending_copy: bool,
    pub quit_btn_bounds: Option<(u16, u16, u16)>,
    pub help_btn_bounds: Option<(u16, u16, u16)>,
    pub drag_active: bool,
    pub drag_start_cursor: Option<(i32, i32)>,
    pub drag_start_window: Option<(i32, i32)>,
    pub is_scanning: bool,
    pub scan_rx: Option<std::sync::mpsc::Receiver<Result<Vec<WlanNetwork>, u32>>>,
    pub username: String,
    pub hostname: String,
    pub os_version: String,
}

impl AppState {
    pub fn new() -> Self {
        let on_battery = win32::query_power_status().map(|p| !p.ac_online).unwrap_or(false);
        let username = std::env::var("USERNAME").unwrap_or_else(|_| "user".to_string());
        let hostname = std::env::var("COMPUTERNAME").unwrap_or_else(|_| "localhost".to_string());
        let os_version = win32::query_os_version();

        Self {
            networks: Vec::new(),
            selected_network_idx: 0,
            is_scanning: false,
            scan_rx: None,
            focus: FocusedSection::NetworkList,
            password_box: TextBox::default(),
            show_password_overlay: false,
            password_visible: false,
            search_box: TextBox::default(),
            search_active: false,
            eap_username: String::new(),
            eap_prompt_username: false,
            hidden_ssid: String::new(),
            hidden_secured: true,
            hidden_prompt_step: 0,
            show_hidden_overlay: false,
            hidden_box: TextBox::default(),
            show_share_overlay: false,
            share_ssid: String::new(),
            share_auth: String::new(),
            share_qr_lines: Vec::new(),
            share_box: TextBox::default(),
            share_prompt_password: false,
            show_profiles_overlay: false,
            profiles_list: Vec::new(),
            profiles_selected_idx: 0,
            status_msg: "Ready. Press Tab to cycle focus. Press Space/r to scan. (? for help)".to_string(),
            status_ttl: 0,
            should_quit: false,
            glyphs: GlyphMap::load(),
            on_battery,
            last_power_check: Instant::now(),
            last_scan: Instant::now() - Duration::from_secs(10), // Force scan on startup
            show_help: false,
            show_markdown: None,
            markdown_lines: Vec::new(),
            markdown_scroll: 0,
            selection_start: None,
            selection_end: None,
            selection_pending_copy: false,
            quit_btn_bounds: None,
            help_btn_bounds: None,
            drag_active: false,
            drag_start_cursor: None,
            drag_start_window: None,
            username,
            hostname,
            os_version,
        }
    }

    pub fn set_status(&mut self, text: String, is_error: bool) {
        self.status_msg = text;
        self.status_ttl = 6000; // 6 seconds duration
        log_message(if is_error { "ERROR" } else { "INFO" }, &self.status_msg);
    }

    pub fn sync_power_status_if_needed(&mut self) {
        if self.last_power_check.elapsed() > Duration::from_millis(5000) {
            self.last_power_check = Instant::now();
            let power = win32::query_power_status();
            let current_on_battery = power.as_ref().map(|p| !p.ac_online).unwrap_or(false);
            if current_on_battery != self.on_battery {
                self.on_battery = current_on_battery;
                let state = if current_on_battery {
                    "Battery (Power-Saving Throttling Enabled)"
                } else {
                    "AC Power (Full Speed)"
                };
                self.set_status(format!("Power source changed. Status: {}", state), false);
            }
        }
    }

    pub fn scan_wifi(&mut self, force: bool) {
        if self.is_scanning {
            return;
        }
        self.is_scanning = true;
        self.last_scan = Instant::now();

        if force {
            self.set_status("Scanning for Wi-Fi networks...".to_string(), false);
        }

        let (tx, rx) = std::sync::mpsc::channel();
        self.scan_rx = Some(rx);

        std::thread::spawn(move || {
            let res = win32::query_wifi_networks(force);
            let _ = tx.send(res);
        });
    }

    pub fn check_scan_results(&mut self) {
        if let Some(ref rx) = self.scan_rx
            && let Ok(res) = rx.try_recv() {
                self.is_scanning = false;
                self.scan_rx = None;
                match res {
                    Ok(nets) => {
                        self.networks = nets;
                        if self.selected_network_idx >= self.networks.len() {
                            self.selected_network_idx = self.networks.len().saturating_sub(1);
                        }
                        
                        if self.networks.is_empty()
                            && let Ok(guid) = win32::get_first_interface_guid()
                                && let Ok(state) = win32::query_radio_state(&guid)
                                    && !state.software_on {
                                        self.set_status("Wi-Fi Radio is Off. Press 't' to turn it On.".to_string(), false);
                                        return;
                                    }
                        
                        self.set_status("Wi-Fi network scan completed successfully.".to_string(), false);
                    }
                    Err(e) => {
                        self.set_status(format!("Wi-Fi scan query failed: Code {}", e), true);
                    }
                }
            }
    }
}
