use std::{
    io,
    time::{Duration, Instant},
};

use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{
    Terminal,
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Clear},
};

mod config;
mod input;
mod logger;
mod wlan;

#[cfg(not(windows))]
pub use crate::wlan::windows_sys;

mod win32 {
    pub use rcommon::win32::*;
    pub use crate::wlan::*;
}
mod widgets {
    pub use rcommon::widgets::*;
}

use input::TextBox;
use logger::log_message;
use win32::{BorderlessConsole, ConsoleTitleGuard, SingleInstanceGuard, WlanNetwork};

// Embedded markdown documentation files
const README_CONTENT: &str = include_str!("../README.md");
const SUPPORT_CONTENT: &str = include_str!("../SUPPORT.md");
const LICENSE_CONTENT: &str = include_str!("../LICENSE.md");
const COPYRIGHT_CONTENT: &str = include_str!("../COPYRIGHT.md");
const PRIVACY_CONTENT: &str = include_str!("../PRIVACY.md");
const SECURITY_CONTENT: &str = include_str!("../SECURITY.md");
const CONTRIBUTING_CONTENT: &str = include_str!("../CONTRIBUTING.md");

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FocusedSection {
    NetworkList,
    InfoPanel,
}

#[derive(Debug, Clone, Copy)]
pub struct ThemeColors {
    pub border: Color,
    pub border_active: Color,
    pub text_main: Color,
    pub text_dim: Color,
    pub accent: Color,
}

fn get_theme(dark: bool, accent_color: Color) -> ThemeColors {
    if dark {
        ThemeColors {
            border: Color::Rgb(68, 68, 84),
            border_active: accent_color,
            text_main: Color::Rgb(248, 248, 242),
            text_dim: Color::Rgb(136, 136, 153),
            accent: accent_color,
        }
    } else {
        ThemeColors {
            border: Color::Rgb(180, 180, 190),
            border_active: accent_color,
            text_main: Color::Rgb(40, 42, 54),
            text_dim: Color::Rgb(100, 100, 115),
            accent: accent_color,
        }
    }
}

// Custom Markdown renderer
fn parse_markdown_to_lines(content: &str, theme: &ThemeColors) -> Vec<Line<'static>> {
    let mut lines = Vec::new();
    let mut current_paragraph = String::new();

    let flush_paragraph = |para: &mut String, lines: &mut Vec<Line<'static>>| {
        if !para.is_empty() {
            lines.push(Line::from(Span::styled(
                para.clone(),
                Style::default().fg(theme.text_main),
            )));
            para.clear();
        }
    };

    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            flush_paragraph(&mut current_paragraph, &mut lines);
            lines.push(Line::from(""));
            continue;
        }

        if trimmed.starts_with("# ") {
            flush_paragraph(&mut current_paragraph, &mut lines);
            let header = trimmed[2..].to_string();
            lines.push(Line::from(""));
            lines.push(Line::from(Span::styled(
                format!("=== {} ===", header.to_uppercase()),
                Style::default()
                    .fg(theme.accent)
                    .add_modifier(Modifier::BOLD),
            )));
            lines.push(Line::from(""));
        } else if trimmed.starts_with("## ") {
            flush_paragraph(&mut current_paragraph, &mut lines);
            let header = trimmed[3..].to_string();
            lines.push(Line::from(""));
            lines.push(Line::from(Span::styled(
                format!("--- {} ---", header),
                Style::default()
                    .fg(theme.accent)
                    .add_modifier(Modifier::BOLD),
            )));
            lines.push(Line::from(""));
        } else if trimmed.starts_with("* ") || trimmed.starts_with("- ") {
            flush_paragraph(&mut current_paragraph, &mut lines);
            let item = trimmed[2..].to_string();
            lines.push(Line::from(vec![
                Span::styled(" * ", Style::default().fg(theme.accent)),
                Span::styled(item, Style::default().fg(theme.text_main)),
            ]));
        } else {
            if !current_paragraph.is_empty() {
                current_paragraph.push(' ');
            }
            current_paragraph.push_str(trimmed);
        }
    }

    flush_paragraph(&mut current_paragraph, &mut lines);
    lines
}

struct AppState {
    networks: Vec<WlanNetwork>,
    selected_network_idx: usize,
    focus: FocusedSection,
    
    password_box: TextBox,
    show_password_overlay: bool,
    password_visible: bool,
    
    search_box: TextBox,
    search_active: bool,

    // EAP Enterprise sequential prompt fields
    eap_username: String,
    eap_prompt_username: bool,

    // Hidden SSID Connection overlay fields
    hidden_ssid: String,
    hidden_secured: bool,
    hidden_prompt_step: u8, // 0: idle, 1: SSID, 2: Security (y/n), 3: Password
    show_hidden_overlay: bool,
    hidden_box: TextBox,

    // Share Wifi Network QR Code overlay fields
    show_share_overlay: bool,
    share_ssid: String,
    share_auth: String,
    share_qr_lines: Vec<String>,
    share_box: TextBox,
    share_prompt_password: bool,

    // Manage Offline Known Networks (Profiles) overlay fields
    show_profiles_overlay: bool,
    profiles_list: Vec<(String, windows_sys::core::GUID)>,
    profiles_selected_idx: usize,
    
    status_msg: String,
    status_ttl: u32,
    should_quit: bool,
    
    glyphs: win32::GlyphMap,
    on_battery: bool,
    last_power_check: Instant,
    last_scan: Instant,
    
    show_help: bool,
    show_markdown: Option<String>,
    markdown_lines: Vec<Line<'static>>,
    markdown_scroll: usize,
    
    // Mouse Selection Info
    selection_start: Option<(u16, u16)>,
    selection_end: Option<(u16, u16)>,
    selection_pending_copy: bool,
    quit_btn_bounds: Option<(u16, u16, u16)>,
    help_btn_bounds: Option<(u16, u16, u16)>,
    drag_active: bool,
    drag_start_cursor: Option<(i32, i32)>,
    drag_start_window: Option<(i32, i32)>,
    is_scanning: bool,
    scan_rx: Option<std::sync::mpsc::Receiver<Result<Vec<WlanNetwork>, u32>>>,
}

impl AppState {
    fn new() -> Self {
        let on_battery = win32::query_power_status().map(|p| !p.ac_online).unwrap_or(false);

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
            glyphs: win32::GlyphMap::load(),
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
        }
    }

    fn set_status(&mut self, text: String, is_error: bool) {
        self.status_msg = text;
        self.status_ttl = 6000; // 6 seconds duration
        log_message(if is_error { "ERROR" } else { "INFO" }, &self.status_msg);
    }

    fn sync_power_status_if_needed(&mut self) {
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

    fn scan_wifi(&mut self, force: bool) {
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

    fn check_scan_results(&mut self) {
        if let Some(ref rx) = self.scan_rx {
            if let Ok(res) = rx.try_recv() {
                self.is_scanning = false;
                self.scan_rx = None;
                match res {
                    Ok(nets) => {
                        self.networks = nets;
                        if self.selected_network_idx >= self.networks.len() {
                            self.selected_network_idx = self.networks.len().saturating_sub(1);
                        }
                        
                        if self.networks.is_empty() {
                            if let Ok(guid) = win32::get_first_interface_guid() {
                                if let Ok(state) = win32::query_radio_state(&guid) {
                                    if !state.software_on {
                                        self.set_status("Wi-Fi Radio is Off. Press 't' to turn it On.".to_string(), false);
                                        return;
                                    }
                                }
                            }
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
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = config::AppConfig::load();
    win32::relaunch_in_conhost_if_needed();

    logger::set_event_log_enabled(config.enable_event_log);

    logger::log_message("INFO", "rwifi application starting up...");
    
    let _instance_guard = match SingleInstanceGuard::try_new() {
        Ok(g) => g,
        Err(e) => {
            eprintln!("Error: {}", e);
            std::process::exit(1);
        }
    };

    let _title_guard = ConsoleTitleGuard::new("rWif");

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    let _ = execute!(stdout, ratatui::crossterm::terminal::SetSize(100, 35));
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    
    let _borderless = if config.enable_borderless {
        Some(BorderlessConsole::enable())
    } else {
        None
    };
    std::thread::sleep(Duration::from_millis(50)); // Allow dimensions settle delay

    if _borderless.is_none() {
        win32::center_console_window();
    }

    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = AppState::new();
    
    // Initial Scan
    app.scan_wifi(true);

    let mut last_tick = Instant::now();

    while !app.should_quit {
        app.check_scan_results();
        app.sync_power_status_if_needed();

        // Check periodic scanning
        let scan_interval = if app.on_battery {
            Duration::from_secs(12)
        } else {
            Duration::from_secs(6)
        };
        if app.last_scan.elapsed() > scan_interval && !app.show_password_overlay {
            app.scan_wifi(false);
        }

        // Render UI
        let dark_mode = match config.theme_mode.as_str() {
            "dark" => true,
            "light" => false,
            _ => win32::query_dark_mode(),
        };
        let accent_color = win32::get_dwm_accent_color();
        let theme = get_theme(dark_mode, accent_color);

        terminal.draw(|f| draw_ui(f, &mut app, &theme))?;

        let tick_rate = if app.on_battery {
            Duration::from_millis(500)
        } else {
            Duration::from_millis(250)
        };
        let timeout = tick_rate
            .checked_sub(last_tick.elapsed())
            .unwrap_or_else(|| Duration::from_secs(0));

        if event::poll(timeout)? {
            match event::read()? {
                Event::Key(key) => {
                    if key.kind == KeyEventKind::Press {
                        handle_keypress(&mut app, key.code, &theme);
                    }
                }
                Event::Mouse(mouse_event) => {
                    match mouse_event.kind {
                        event::MouseEventKind::Down(event::MouseButton::Left) => {
                            let mut clicked_btn = false;
                            if let Some((btn_y, btn_start, btn_end)) = app.quit_btn_bounds {
                                if mouse_event.row == btn_y && mouse_event.column >= btn_start && mouse_event.column < btn_end {
                                    app.should_quit = true;
                                    clicked_btn = true;
                                }
                            }
                            if !clicked_btn {
                                if let Some((btn_y, btn_start, btn_end)) = app.help_btn_bounds {
                                    if mouse_event.row == btn_y && mouse_event.column >= btn_start && mouse_event.column < btn_end {
                                        app.show_help = !app.show_help;
                                        app.set_status(if app.show_help {
                                            "Help overlay active. Press ESC/q to close.".to_string()
                                        } else {
                                            "Help overlay closed.".to_string()
                                        }, false);
                                        clicked_btn = true;
                                    }
                                }
                            }
                            if !clicked_btn {
                                if mouse_event.row <= 2 {
                                    if let Some(cursor_pos) = win32::query_cursor_pos() {
                                        if let Some(rect) = win32::get_window_rect() {
                                            app.drag_active = true;
                                            app.drag_start_cursor = Some(cursor_pos);
                                            app.drag_start_window = Some((rect.left, rect.top));
                                        }
                                    }
                                } else {
                                    app.selection_start = Some((mouse_event.column, mouse_event.row));
                                    app.selection_end = Some((mouse_event.column, mouse_event.row));
                                    app.selection_pending_copy = false;
                                }
                            }
                        }
                        event::MouseEventKind::Drag(event::MouseButton::Left) => {
                            if app.drag_active {
                                if let (Some(start_cursor), Some(start_window)) = (app.drag_start_cursor, app.drag_start_window) {
                                    if let Some(curr_cursor) = win32::query_cursor_pos() {
                                        let dx = curr_cursor.0 - start_cursor.0;
                                        let dy = curr_cursor.1 - start_cursor.1;
                                        win32::set_window_pos(start_window.0 + dx, start_window.1 + dy);
                                    }
                                }
                            } else if app.selection_start.is_some() {
                                app.selection_end = Some((mouse_event.column, mouse_event.row));
                            }
                        }
                        event::MouseEventKind::Up(event::MouseButton::Left) => {
                            if app.drag_active {
                                app.drag_active = false;
                                app.drag_start_cursor = None;
                                app.drag_start_window = None;
                            } else if let (Some(start), Some(end)) = (app.selection_start, app.selection_end) {
                                if start != end {
                                    app.selection_pending_copy = true;
                                } else {
                                    app.selection_start = None;
                                    app.selection_end = None;
                                }
                            }
                        }
                        event::MouseEventKind::ScrollUp => {
                            if app.show_markdown.is_some() {
                                app.markdown_scroll = app.markdown_scroll.saturating_sub(3);
                            }
                        }
                        event::MouseEventKind::ScrollDown => {
                            if app.show_markdown.is_some() {
                                let (_, term_h) = ratatui::crossterm::terminal::size().unwrap_or((110, 38));
                                let inner_h = ((term_h * 80) / 100).saturating_sub(2) as usize;
                                let max_scroll = app.markdown_lines.len().saturating_sub(inner_h);
                                if app.markdown_scroll < max_scroll {
                                    app.markdown_scroll = (app.markdown_scroll + 3).min(max_scroll);
                                }
                            }
                        }
                        _ => {}
                    }
                }
                _ => {}
            }
        }

        if last_tick.elapsed() >= tick_rate {
            last_tick = Instant::now();
            if app.status_ttl > 0 {
                app.status_ttl = app.status_ttl.saturating_sub(tick_rate.as_millis() as u32);
                if app.status_ttl == 0 {
                    app.status_msg = "Ready. Press Tab to cycle focus. Press Space/r to scan. (? for help)".to_string();
                }
            }
        }
    }

    // Restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture,
        ratatui::crossterm::cursor::Show
    )?;
    terminal.show_cursor()?;

    Ok(())
}

fn handle_keypress(app: &mut AppState, code: KeyCode, theme: &ThemeColors) {
    if app.show_share_overlay {
        match code {
            KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('Q') | KeyCode::Char('s') | KeyCode::Char('S') => {
                app.show_share_overlay = false;
                app.share_box.active = false;
                app.share_box.clear();
            }
            KeyCode::Enter => {
                if app.share_prompt_password {
                    let pwd = app.share_box.text.clone();
                    app.share_qr_lines = generate_qr_code_lines(&app.share_ssid, &pwd, &app.share_auth);
                    app.share_prompt_password = false;
                    app.share_box.active = false;
                    app.share_box.clear();
                } else {
                    app.show_share_overlay = false;
                }
            }
            other => {
                if app.share_prompt_password {
                    app.share_box.handle_key(other);
                }
            }
        }
        return;
    }

    if app.show_hidden_overlay {
        match code {
            KeyCode::Esc => {
                app.show_hidden_overlay = false;
                app.hidden_box.active = false;
                app.hidden_box.clear();
                app.hidden_prompt_step = 0;
            }
            KeyCode::Enter => {
                match app.hidden_prompt_step {
                    1 => {
                        let ssid = app.hidden_box.text.trim().to_string();
                        if !ssid.is_empty() {
                            app.hidden_ssid = ssid;
                            app.hidden_prompt_step = 2;
                            app.hidden_box.clear();
                        }
                    }
                    2 => {
                        let ans = app.hidden_box.text.trim().to_lowercase();
                        if ans.starts_with('n') {
                            app.hidden_secured = false;
                            app.show_hidden_overlay = false;
                            app.hidden_box.active = false;
                            app.hidden_box.clear();
                            app.hidden_prompt_step = 0;
                            app.set_status(format!("Connecting to hidden network {}...", app.hidden_ssid), false);
                            if let Ok(guid) = win32::get_first_interface_guid() {
                                match win32::connect_to_hidden_wifi(&app.hidden_ssid, None, false, "Open", "None", &guid) {
                                    Ok(_) => {
                                        app.set_status(format!("Successfully connected to hidden network {}", app.hidden_ssid), false);
                                        app.scan_wifi(false);
                                    }
                                    Err(e) => {
                                        app.set_status(format!("Failed to connect to hidden network: {}", e), true);
                                    }
                                }
                            }
                        } else {
                            app.hidden_secured = true;
                            app.hidden_prompt_step = 3;
                            app.hidden_box.clear();
                        }
                    }
                    3 => {
                        let pwd = app.hidden_box.text.clone();
                        app.show_hidden_overlay = false;
                        app.hidden_box.active = false;
                        app.hidden_box.clear();
                        app.hidden_prompt_step = 0;
                        app.set_status(format!("Connecting to hidden network {}...", app.hidden_ssid), false);
                        if let Ok(guid) = win32::get_first_interface_guid() {
                            match win32::connect_to_hidden_wifi(&app.hidden_ssid, Some(&pwd), true, "WPA2-Personal", "AES", &guid) {
                                Ok(_) => {
                                    app.set_status(format!("Successfully connected to hidden network {}", app.hidden_ssid), false);
                                    app.scan_wifi(false);
                                }
                                Err(e) => {
                                    app.set_status(format!("Failed to connect to hidden network: {}", e), true);
                                }
                            }
                        }
                    }
                    _ => {}
                }
            }
            other => {
                app.hidden_box.handle_key(other);
            }
        }
        return;
    }

    if app.show_profiles_overlay {
        match code {
            KeyCode::Esc | KeyCode::Char('p') | KeyCode::Char('P') | KeyCode::Char('q') | KeyCode::Char('Q') => {
                app.show_profiles_overlay = false;
            }
            KeyCode::Up | KeyCode::Char('k') => {
                if app.profiles_selected_idx > 0 {
                    app.profiles_selected_idx -= 1;
                }
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if app.profiles_selected_idx + 1 < app.profiles_list.len() {
                    app.profiles_selected_idx += 1;
                }
            }
            KeyCode::Char('d') | KeyCode::Char('D') | KeyCode::Delete => {
                if !app.profiles_list.is_empty() {
                    let (name, guid) = app.profiles_list[app.profiles_selected_idx].clone();
                    app.set_status(format!("Deleting offline profile {}...", name), false);
                    match win32::delete_wifi_profile(&name, &guid) {
                        Ok(_) => {
                            app.set_status(format!("Successfully deleted profile {}", name), false);
                            if let Ok(list) = win32::query_saved_profiles() {
                                app.profiles_list = list;
                            }
                            if app.profiles_selected_idx >= app.profiles_list.len() {
                                app.profiles_selected_idx = app.profiles_list.len().saturating_sub(1);
                            }
                        }
                        Err(e) => {
                            app.set_status(format!("Failed to delete profile: {}", e), true);
                        }
                    }
                }
            }
            _ => {}
        }
        return;
    }

    if app.show_password_overlay {
        match code {
            KeyCode::Enter => {
                let text_val = app.password_box.text.clone();
                
                if let Some(net) = app.networks.get(app.selected_network_idx).cloned() {
                    let is_enterprise = net.auth_algorithm.contains("Enterprise") || net.auth_algorithm.contains("RSNA");
                    
                    if is_enterprise && app.eap_prompt_username {
                        if text_val.trim().is_empty() {
                            app.set_status("Username cannot be empty for enterprise networks.".to_string(), true);
                            return;
                        }
                        app.eap_username = text_val;
                        app.password_box.clear();
                        app.eap_prompt_username = false;
                        app.password_visible = false;
                        app.set_status("Username saved. Enter password.".to_string(), false);
                        return;
                    }
                    
                    let is_empty = text_val.trim().is_empty();
                    if !is_enterprise && is_empty && !net.has_profile {
                        app.set_status("Password cannot be empty for a new secured network.".to_string(), true);
                        return;
                    }
                    
                    app.show_password_overlay = false;
                    app.password_box.active = false;
                    app.password_box.clear();
                    
                    if is_enterprise {
                        app.set_status(format!("Connecting to enterprise network {}...", net.ssid), false);
                        match win32::connect_to_enterprise_wifi(&net.ssid, &app.eap_username, &text_val, &net) {
                            Ok(_) => {
                                app.set_status(format!("Successfully connected to enterprise {}", net.ssid), false);
                                win32::show_toast_notification("rWifi - Connected", &format!("Connected to {}", net.ssid));
                                win32::log_windows_event("rWifi", 4, 1001, &format!("Successfully connected to {}", net.ssid));
                                app.scan_wifi(false);
                            }
                            Err(err) => {
                                app.set_status(format!("Failed to connect: {}", err), true);
                                win32::show_toast_notification("rWifi - Connection Failed", &format!("Failed to connect to {}: {}", net.ssid, err));
                                win32::log_windows_event("rWifi", 1, 1002, &format!("Failed to connect to {}: {}", net.ssid, err));
                            }
                        }
                    } else {
                        let pwd_param = if is_empty { None } else { Some(&text_val) };
                        
                        app.set_status(format!("Connecting to {}...", net.ssid), false);
                        
                        let mut net_param = net.clone();
                        if !is_empty {
                            net_param.has_profile = false;
                        }
                        
                        match win32::connect_to_wifi(&net.ssid, pwd_param.map(|s| s.as_str()), &net_param) {
                            Ok(_) => {
                                app.set_status(format!("Successfully connected to {}", net.ssid), false);
                                win32::show_toast_notification("rWifi - Connected", &format!("Connected to {}", net.ssid));
                                win32::log_windows_event("rWifi", 4, 1001, &format!("Successfully connected to {}", net.ssid));
                                app.scan_wifi(false);
                            }
                            Err(err) => {
                                app.set_status(format!("Failed to connect: {}", err), true);
                                win32::show_toast_notification("rWifi - Connection Failed", &format!("Failed to connect to {}: {}", net.ssid, err));
                                win32::log_windows_event("rWifi", 1, 1002, &format!("Failed to connect to {}: {}", net.ssid, err));
                            }
                        }
                    }
                } else {
                    app.show_password_overlay = false;
                    app.password_box.active = false;
                    app.password_box.clear();
                }
            }
            KeyCode::Esc => {
                app.show_password_overlay = false;
                app.password_box.active = false;
                app.password_box.clear();
                app.eap_prompt_username = false;
                app.set_status("Connection cancelled.".to_string(), false);
            }
            KeyCode::Char('v') | KeyCode::Char('V') if event::poll(Duration::from_millis(0)).unwrap_or(false) => {
                app.password_box.handle_key(code);
            }
            other => {
                if other == KeyCode::Tab {
                    app.password_visible = !app.password_visible;
                } else {
                    app.password_box.handle_key(other);
                }
            }
        }
        return;
    }

    // Global override for F1-F7 documentation keys
    match code {
        KeyCode::F(1) => {
            app.show_help = false;
            app.show_markdown = Some("README.md".to_string());
            app.markdown_lines = parse_markdown_to_lines(README_CONTENT, theme);
            app.markdown_scroll = 0;
            return;
        }
        KeyCode::F(2) => {
            app.show_help = false;
            app.show_markdown = Some("SUPPORT.md".to_string());
            app.markdown_lines = parse_markdown_to_lines(SUPPORT_CONTENT, theme);
            app.markdown_scroll = 0;
            return;
        }
        KeyCode::F(3) => {
            app.show_help = false;
            app.show_markdown = Some("LICENSE.md".to_string());
            app.markdown_lines = parse_markdown_to_lines(LICENSE_CONTENT, theme);
            app.markdown_scroll = 0;
            return;
        }
        KeyCode::F(4) => {
            app.show_help = false;
            app.show_markdown = Some("COPYRIGHT.md".to_string());
            app.markdown_lines = parse_markdown_to_lines(COPYRIGHT_CONTENT, theme);
            app.markdown_scroll = 0;
            return;
        }
        KeyCode::F(5) => {
            app.show_help = false;
            app.show_markdown = Some("PRIVACY.md".to_string());
            app.markdown_lines = parse_markdown_to_lines(PRIVACY_CONTENT, theme);
            app.markdown_scroll = 0;
            return;
        }
        KeyCode::F(6) => {
            app.show_help = false;
            app.show_markdown = Some("SECURITY.md".to_string());
            app.markdown_lines = parse_markdown_to_lines(SECURITY_CONTENT, theme);
            app.markdown_scroll = 0;
            return;
        }
        KeyCode::F(7) => {
            app.show_help = false;
            app.show_markdown = Some("CONTRIBUTING.md".to_string());
            app.markdown_lines = parse_markdown_to_lines(CONTRIBUTING_CONTENT, theme);
            app.markdown_scroll = 0;
            return;
        }
        _ => {}
    }

    if app.show_help {
        match code {
            KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('Q') | KeyCode::Char('h') => {
                app.show_help = false;
            }
            _ => {}
        }
        return;
    }

    if let Some(_) = app.show_markdown {
        match code {
            KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('Q') => {
                app.show_markdown = None;
                app.markdown_lines.clear();
            }
            KeyCode::Up | KeyCode::Char('k') => {
                app.markdown_scroll = app.markdown_scroll.saturating_sub(1);
            }
            KeyCode::Down | KeyCode::Char('j') => {
                let (_, term_h) = ratatui::crossterm::terminal::size().unwrap_or((110, 38));
                let inner_h = ((term_h * 80) / 100).saturating_sub(2) as usize;
                let max_scroll = app.markdown_lines.len().saturating_sub(inner_h);
                if app.markdown_scroll < max_scroll {
                    app.markdown_scroll += 1;
                }
            }
            _ => {}
        }
        return;
    }

    let filtered_nets: Vec<WlanNetwork> = if app.search_active {
        app.networks
            .iter()
            .filter(|n| n.ssid.to_lowercase().contains(&app.search_box.text.to_lowercase()))
            .cloned()
            .collect()
    } else {
        app.networks.clone()
    };

    if app.search_active {
        match code {
            KeyCode::Esc => {
                app.search_active = false;
                app.search_box.active = false;
                app.search_box.clear();
                app.selected_network_idx = 0;
                app.set_status("Filter cleared.".to_string(), false);
                return;
            }
            KeyCode::Enter | KeyCode::Up | KeyCode::Down | KeyCode::Tab => {
                // Fall through
            }
            other => {
                app.search_box.handle_key(other);
                app.selected_network_idx = 0;
                return;
            }
        }
    }

    match code {
        KeyCode::Char('q') | KeyCode::Char('Q') | KeyCode::Esc => {
            app.should_quit = true;
        }
        KeyCode::Tab => {
            app.focus = match app.focus {
                FocusedSection::NetworkList => FocusedSection::InfoPanel,
                FocusedSection::InfoPanel => FocusedSection::NetworkList,
            };
        }
        KeyCode::Up | KeyCode::Char('k') => {
            if app.selected_network_idx > 0 {
                app.selected_network_idx -= 1;
            }
        }
        KeyCode::Down | KeyCode::Char('j') => {
            if app.selected_network_idx + 1 < filtered_nets.len() {
                app.selected_network_idx += 1;
            }
        }
        KeyCode::Char('r') | KeyCode::Char('R') | KeyCode::Char(' ') => {
            app.set_status("Scanning Wi-Fi networks... Please wait.".to_string(), false);
            app.scan_wifi(true);
        }
        KeyCode::Enter => {
            if let Some(net) = filtered_nets.get(app.selected_network_idx).cloned() {
                if app.search_active {
                    let selected_ssid = net.ssid.clone();
                    app.search_active = false;
                    app.search_box.active = false;
                    app.search_box.clear();
                    if let Some(idx) = app.networks.iter().position(|n| n.ssid == selected_ssid) {
                        app.selected_network_idx = idx;
                    }
                }
                if net.is_connected {
                    app.set_status(format!("Disconnecting from {}...", net.ssid), false);
                    match win32::disconnect_wifi(&net.interface_guid) {
                        Ok(_) => {
                            app.set_status(format!("Disconnected from {}", net.ssid), false);
                            win32::show_toast_notification("rWifi - Disconnected", &format!("Disconnected from {}", net.ssid));
                            app.scan_wifi(false);
                        }
                        Err(e) => {
                            app.set_status(format!("Failed to disconnect: {}", e), true);
                        }
                    }
                } else if !net.security_enabled {
                    app.set_status(format!("Connecting to {}...", net.ssid), false);
                    match win32::connect_to_wifi(&net.ssid, None, &net) {
                        Ok(_) => {
                            app.set_status(format!("Successfully connected to {}", net.ssid), false);
                            win32::show_toast_notification("rWifi - Connected", &format!("Connected to {}", net.ssid));
                            win32::log_windows_event("rWifi", 4, 1001, &format!("Successfully connected to {}", net.ssid));
                            app.scan_wifi(false);
                        }
                        Err(e) => {
                            app.set_status(format!("Failed to connect: {}", e), true);
                            win32::show_toast_notification("rWifi - Connection Failed", &format!("Failed to connect to {}: {}", net.ssid, e));
                        }
                    }
                } else {
                    let is_enterprise = net.auth_algorithm.contains("Enterprise") || net.auth_algorithm.contains("RSNA");
                    app.show_password_overlay = true;
                    app.password_box.active = true;
                    if is_enterprise {
                        app.eap_prompt_username = true;
                        app.password_visible = true;
                        app.password_box.clear();
                    } else {
                        app.eap_prompt_username = false;
                        app.password_visible = false;
                        app.password_box.clear();
                    }
                }
            }
        }
        KeyCode::Char('d') | KeyCode::Char('D') | KeyCode::Delete => {
            if let Some(net) = filtered_nets.get(app.selected_network_idx).cloned() {
                if app.search_active {
                    let selected_ssid = net.ssid.clone();
                    app.search_active = false;
                    app.search_box.active = false;
                    app.search_box.clear();
                    if let Some(idx) = app.networks.iter().position(|n| n.ssid == selected_ssid) {
                        app.selected_network_idx = idx;
                    }
                }
                if net.is_connected {
                    app.set_status(format!("Disconnecting from {}...", net.ssid), false);
                    let _ = win32::disconnect_wifi(&net.interface_guid);
                    app.scan_wifi(false);
                } else if net.has_profile {
                    app.set_status(format!("Deleting profile for {}...", net.ssid), false);
                    match win32::delete_wifi_profile(&net.ssid, &net.interface_guid) {
                        Ok(_) => {
                            app.set_status(format!("Deleted profile for {}", net.ssid), false);
                            app.scan_wifi(false);
                        }
                        Err(e) => {
                            app.set_status(format!("Failed to delete profile: {}", e), true);
                        }
                    }
                }
            }
        }
        KeyCode::Char('/') => {
            app.search_active = true;
            app.search_box.active = true;
            app.search_box.clear();
            app.selected_network_idx = 0;
            app.set_status("Search mode active. Type SSID to filter.".to_string(), false);
        }
        KeyCode::Char('t') | KeyCode::Char('T') => {
            if let Ok(guid) = win32::get_first_interface_guid() {
                if let Ok(state) = win32::query_radio_state(&guid) {
                    let target_state = !state.software_on;
                    app.set_status(format!("Toggling Wi-Fi Radio to {}...", if target_state { "On" } else { "Off" }), false);
                    match win32::set_radio_state(&guid, target_state) {
                        Ok(_) => {
                            app.set_status(format!("Wi-Fi Radio successfully turned {}.", if target_state { "On" } else { "Off" }), false);
                            win32::show_toast_notification(
                                "rWifi - Radio Toggled",
                                &format!("Wi-Fi Radio turned {}", if target_state { "On" } else { "Off" })
                            );
                            app.scan_wifi(true);
                        }
                        Err(e) => {
                            app.set_status(format!("Failed to toggle Wi-Fi radio: Code {}", e), true);
                        }
                    }
                } else {
                    app.set_status("Failed to query Wi-Fi radio state.".to_string(), true);
                }
            } else {
                app.set_status("No wireless interface found to toggle.".to_string(), true);
            }
        }
        KeyCode::Char('s') | KeyCode::Char('S') => {
            if let Some(net) = filtered_nets.get(app.selected_network_idx).cloned() {
                app.share_ssid = net.ssid.clone();
                app.share_auth = net.auth_algorithm.clone();
                app.share_qr_lines.clear();
                if net.security_enabled {
                    if let Some(password) = win32::query_saved_password(&net.ssid) {
                        app.share_qr_lines = generate_qr_code_lines(&net.ssid, &password, &net.auth_algorithm);
                        app.show_share_overlay = true;
                        app.share_prompt_password = false;
                        app.share_box.active = false;
                        app.set_status(format!("Generated QR code for network {} (loaded saved password).", net.ssid), false);
                    } else {
                        app.show_share_overlay = true;
                        app.share_prompt_password = true;
                        app.share_box.clear();
                        app.share_box.active = true;
                        app.set_status(format!("Enter password to generate QR code for network {}.", net.ssid), false);
                    }
                } else {
                    app.share_qr_lines = generate_qr_code_lines(&net.ssid, "", &net.auth_algorithm);
                    app.show_share_overlay = true;
                    app.share_prompt_password = false;
                    app.share_box.active = false;
                    app.set_status(format!("Generated QR code for open network {}.", net.ssid), false);
                }
            }
        }
        KeyCode::Char('p') | KeyCode::Char('P') => {
            app.set_status("Querying saved offline network profiles...".to_string(), false);
            match win32::query_saved_profiles() {
                Ok(list) => {
                    app.profiles_list = list;
                    app.profiles_selected_idx = 0;
                    app.show_profiles_overlay = true;
                    app.set_status("Known profiles loaded. Press 'd' or 'Delete' to remove profile.".to_string(), false);
                }
                Err(e) => {
                    app.set_status(format!("Failed to retrieve offline profiles: Code {}", e), true);
                }
            }
        }
        KeyCode::Char('w') | KeyCode::Char('W') => {
            app.show_hidden_overlay = true;
            app.hidden_ssid.clear();
            app.hidden_box.clear();
            app.hidden_box.active = true;
            app.hidden_prompt_step = 1;
            app.set_status("Enter hidden network SSID.".to_string(), false);
        }
        KeyCode::Char('h') => {
            app.show_help = true;
        }
        _ => {}
    }
}

fn draw_ui(f: &mut ratatui::Frame, app: &mut AppState, theme: &ThemeColors) {
    let size = f.area();
    if size.width < 100 || size.height < 35 {
        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Rgb(255, 85, 85)))
            .title(Span::styled(
                " ⚠️  Terminal Sizing Warning ",
                Style::default().fg(Color::Rgb(255, 85, 85)).add_modifier(Modifier::BOLD),
            ));

        let text = vec![
            Line::from(""),
            Line::from(Span::styled("Layout Constraints Not Met", Style::default().fg(Color::Rgb(255, 85, 85)).add_modifier(Modifier::BOLD))),
            Line::from(""),
            Line::from(format!("  Current Terminal Size: {}x{}", size.width, size.height)),
            Line::from("  Minimum Required Size: 100x35"),
            Line::from(""),
            Line::from("  Please resize or maximize your terminal window to resume standard rendering."),
        ];
        let p = Paragraph::new(text)
            .block(block)
            .alignment(ratatui::layout::Alignment::Center);

        let area = centered_rect(80, 50, size);
        f.render_widget(Clear, area);
        f.render_widget(p, area);
        return;
    }

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // 0: Header
            Constraint::Min(5),    // 1: Body
            Constraint::Length(3), // 2: Footer Status
        ])
        .split(size);

    // 1. Header
    let username = std::env::var("USERNAME").unwrap_or_else(|_| "user".to_string());
    let hostname = std::env::var("COMPUTERNAME").unwrap_or_else(|_| "localhost".to_string());
    let os_version = win32::query_os_version();

    let header_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.border))
        .title(Span::styled(" Rust Wi-Fi Manager ", Style::default().fg(theme.accent).add_modifier(Modifier::BOLD)));

    let ver_str = format!(" rWifi v{} ", env!("CARGO_PKG_VERSION"));
    let user_host_str = format!("{}@{}", username, hostname);
    let os_str_val = os_version.clone();

    let button_y = chunks[0].y + 1;
    let inner_width = chunks[0].width.saturating_sub(2) as usize;
    let left_len = ver_str.len() + 3 + user_host_str.len() + 3 + os_str_val.len();
    let right_len = 6 + 3 + 6;

    let header_line = if inner_width > left_len + right_len {
        let padding_len = inner_width - (left_len + right_len);
        let padding_str = " ".repeat(padding_len);

        let help_offset = 1 + left_len + padding_len;
        let help_start_x = chunks[0].x + help_offset as u16;
        let help_end_x = help_start_x + 6;
        app.help_btn_bounds = Some((button_y, help_start_x, help_end_x));

        let quit_offset = help_offset + 6 + 3;
        let quit_start_x = chunks[0].x + quit_offset as u16;
        let quit_end_x = quit_start_x + 6;
        app.quit_btn_bounds = Some((button_y, quit_start_x, quit_end_x));

        Line::from(vec![
            Span::styled(ver_str, Style::default().fg(theme.accent).add_modifier(Modifier::BOLD)),
            Span::styled(" │ ", Style::default().fg(theme.border)),
            Span::styled(user_host_str, Style::default().fg(Color::Rgb(255, 215, 0)).add_modifier(Modifier::BOLD)),
            Span::styled(" │ ", Style::default().fg(theme.border)),
            Span::styled(os_str_val, Style::default().fg(theme.accent).add_modifier(Modifier::BOLD)),
            Span::styled(padding_str, Style::default()),
            // Help button
            Span::styled(" ", Style::default().bg(Color::Rgb(250, 210, 50)).fg(Color::Black).add_modifier(Modifier::BOLD)),
            Span::styled("h", Style::default().bg(Color::Rgb(250, 210, 50)).fg(Color::Black).add_modifier(Modifier::BOLD | Modifier::UNDERLINED)),
            Span::styled("elp ", Style::default().bg(Color::Rgb(250, 210, 50)).fg(Color::Black).add_modifier(Modifier::BOLD)),
            Span::styled(" │ ", Style::default().fg(theme.border)),
            // Quit button
            Span::styled(" ", Style::default().bg(Color::Rgb(255, 85, 85)).fg(Color::White).add_modifier(Modifier::BOLD)),
            Span::styled("q", Style::default().bg(Color::Rgb(255, 85, 85)).fg(Color::White).add_modifier(Modifier::BOLD | Modifier::UNDERLINED)),
            Span::styled("uit ", Style::default().bg(Color::Rgb(255, 85, 85)).fg(Color::White).add_modifier(Modifier::BOLD)),
        ])
    } else {
        let help_offset = 1 + ver_str.len() + 3 + user_host_str.len() + 3 + os_str_val.len() + 3;
        let help_start_x = chunks[0].x + help_offset as u16;
        let help_end_x = help_start_x + 6;
        app.help_btn_bounds = Some((button_y, help_start_x, help_end_x));

        let quit_offset = help_offset + 6 + 3;
        let quit_start_x = chunks[0].x + quit_offset as u16;
        let quit_end_x = quit_start_x + 6;
        app.quit_btn_bounds = Some((button_y, quit_start_x, quit_end_x));

        Line::from(vec![
            Span::styled(ver_str, Style::default().fg(theme.accent).add_modifier(Modifier::BOLD)),
            Span::styled(" │ ", Style::default().fg(theme.border)),
            Span::styled(user_host_str, Style::default().fg(Color::Rgb(255, 215, 0)).add_modifier(Modifier::BOLD)),
            Span::styled(" │ ", Style::default().fg(theme.border)),
            Span::styled(os_str_val, Style::default().fg(theme.accent).add_modifier(Modifier::BOLD)),
            Span::styled(" │ ", Style::default().fg(theme.border)),
            // Help button
            Span::styled(" ", Style::default().bg(Color::Rgb(250, 210, 50)).fg(Color::Black).add_modifier(Modifier::BOLD)),
            Span::styled("h", Style::default().bg(Color::Rgb(250, 210, 50)).fg(Color::Black).add_modifier(Modifier::BOLD | Modifier::UNDERLINED)),
            Span::styled("elp ", Style::default().bg(Color::Rgb(250, 210, 50)).fg(Color::Black).add_modifier(Modifier::BOLD)),
            Span::styled(" │ ", Style::default().fg(theme.border)),
            // Quit button
            Span::styled(" ", Style::default().bg(Color::Rgb(255, 85, 85)).fg(Color::White).add_modifier(Modifier::BOLD)),
            Span::styled("q", Style::default().bg(Color::Rgb(255, 85, 85)).fg(Color::White).add_modifier(Modifier::BOLD | Modifier::UNDERLINED)),
            Span::styled("uit ", Style::default().bg(Color::Rgb(255, 85, 85)).fg(Color::White).add_modifier(Modifier::BOLD)),
        ])
    };

    f.render_widget(header_block, chunks[0]);
    f.render_widget(Paragraph::new(header_line), chunks[0].inner(Margin { horizontal: 1, vertical: 1 }));

    // 2. Body
    let body_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(60), // Left: Station List
            Constraint::Percentage(40), // Right: Info Panel
        ])
        .split(chunks[1]);

    let filtered_nets: Vec<&WlanNetwork> = if app.search_active {
        app.networks
            .iter()
            .filter(|n| n.ssid.to_lowercase().contains(&app.search_box.text.to_lowercase()))
            .collect()
    } else {
        app.networks.iter().collect()
    };

    // Left: WiFi Stations List
    let left_border = if app.focus == FocusedSection::NetworkList { theme.border_active } else { theme.border };
    let left_title = if app.search_active {
        format!(" Available Wi-Fi Networks [Filter: {}_] ", app.search_box.text)
    } else {
        " Available Wi-Fi Networks ".to_string()
    };
    let left_block = Block::default()
        .borders(Borders::ALL)
        .title(left_title)
        .title_style(Style::default().fg(left_border).add_modifier(Modifier::BOLD))
        .border_style(Style::default().fg(left_border));

    let mut list_lines = Vec::new();
    if filtered_nets.is_empty() {
        list_lines.push(Line::from(""));
        if app.is_scanning {
            list_lines.push(Line::from(Span::styled("  Scanning for Wi-Fi networks... Please wait.", Style::default().fg(theme.accent).add_modifier(Modifier::BOLD))));
        } else if app.search_active {
            list_lines.push(Line::from(Span::styled("  No matching Wi-Fi networks found. Press Esc to clear filter.", Style::default().fg(theme.text_dim))));
        } else {
            list_lines.push(Line::from(Span::styled("  No wireless stations discovered. Press Space to refresh.", Style::default().fg(theme.text_dim))));
        }
    } else {
        for (idx, net) in filtered_nets.iter().enumerate() {
            let is_selected = idx == app.selected_network_idx;
            
            let bullet = if net.is_connected {
                " * "
            } else if is_selected {
                app.glyphs.play
            } else {
                app.glyphs.play_empty
            };

            let bullet_style = if net.is_connected {
                Style::default().fg(Color::Rgb(80, 250, 80)).add_modifier(Modifier::BOLD)
            } else if is_selected {
                Style::default().fg(theme.accent).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(theme.text_dim)
            };

            let name_style = if is_selected {
                Style::default().fg(theme.text_main).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(theme.text_dim)
            };

            // Format signal strength bar
            let bars_filled = (net.signal_quality as f32 / 20.0).round() as usize;
            let bar_char = if app.glyphs.status_ok == "✔️" { "█" } else { "#" };
            let empty_char = if app.glyphs.status_ok == "✔️" { "░" } else { "-" };
            let bar_str = format!(
                "[{}{}]",
                bar_char.repeat(bars_filled),
                empty_char.repeat(5 - bars_filled)
            );
            let bar_color = if net.signal_quality > 70 {
                Color::Rgb(80, 250, 80)
            } else if net.signal_quality > 40 {
                Color::Rgb(250, 250, 80)
            } else {
                Color::Rgb(250, 80, 80)
            };

            let lock_icon = if net.security_enabled { " [Secured]" } else { " [Open]" };
            let lock_color = if net.security_enabled { Color::Rgb(250, 180, 80) } else { Color::Rgb(80, 250, 80) };

            list_lines.push(Line::from(vec![
                Span::styled(bullet, bullet_style),
                Span::styled(format!("{:<25}", win32::truncate(&net.ssid, 25)), name_style),
                Span::styled(format!(" {:>3}% ", net.signal_quality), Style::default().fg(theme.text_main)),
                Span::styled(bar_str, Style::default().fg(bar_color)),
                Span::styled(lock_icon, Style::default().fg(lock_color)),
            ]));
        }
    }

    f.render_widget(Paragraph::new(list_lines).block(left_block), body_chunks[0]);

    // Right: Detailed Info Panel
    let right_border = if app.focus == FocusedSection::InfoPanel { theme.border_active } else { theme.border };
    let right_block = Block::default()
        .borders(Borders::ALL)
        .title(" Connection Details ")
        .title_style(Style::default().fg(right_border).add_modifier(Modifier::BOLD))
        .border_style(Style::default().fg(right_border));

    let mut right_lines = Vec::new();
    if let Some(net) = filtered_nets.get(app.selected_network_idx) {
        right_lines.push(Line::from(""));
        right_lines.push(Line::from(vec![
            Span::styled("  SSID:        ", Style::default().fg(theme.text_dim)),
            Span::styled(&net.ssid, Style::default().fg(theme.text_main).add_modifier(Modifier::BOLD)),
        ]));
        
        let conn_status = if net.is_connected { "Connected" } else { "Disconnected" };
        let conn_color = if net.is_connected { Color::Rgb(80, 250, 80) } else { theme.text_dim };
        right_lines.push(Line::from(vec![
            Span::styled("  Status:      ", Style::default().fg(theme.text_dim)),
            Span::styled(conn_status, Style::default().fg(conn_color).add_modifier(Modifier::BOLD)),
        ]));

        right_lines.push(Line::from(vec![
            Span::styled("  Signal:      ", Style::default().fg(theme.text_dim)),
            Span::styled(format!("{}% ", net.signal_quality), Style::default().fg(theme.text_main)),
        ]));

        right_lines.push(Line::from(vec![
            Span::styled("  Security:    ", Style::default().fg(theme.text_dim)),
            Span::styled(&net.auth_algorithm, Style::default().fg(theme.text_main)),
        ]));

        right_lines.push(Line::from(vec![
            Span::styled("  Encryption:  ", Style::default().fg(theme.text_dim)),
            Span::styled(&net.cipher_algorithm, Style::default().fg(theme.text_main)),
        ]));

        let profile_status = if net.has_profile { "Saved" } else { "Not Saved" };
        right_lines.push(Line::from(vec![
            Span::styled("  Profile:     ", Style::default().fg(theme.text_dim)),
            Span::styled(profile_status, Style::default().fg(theme.text_main)),
        ]));

        right_lines.push(Line::from(""));
        right_lines.push(Line::from("  Signal Strength Gauge:"));
        right_lines.push(Line::from(""));

        f.render_widget(Paragraph::new(right_lines).block(right_block), body_chunks[1]);

        let gauge_area = Rect {
            x: body_chunks[1].x + 2,
            y: body_chunks[1].y + 16,
            width: body_chunks[1].width.saturating_sub(4),
            height: 1,
        };
        let use_unicode = app.glyphs.status_ok == "✔️";
        let gauge = widgets::AccentGauge::new(
            net.signal_quality as f64 / 100.0,
            "",
            theme.accent,
            theme.border,
            use_unicode,
        );
        f.render_widget(gauge, gauge_area);
    } else {
        right_lines.push(Line::from(""));
        right_lines.push(Line::from(Span::styled("  Select a network from the left to view details.", Style::default().fg(theme.text_dim))));
        f.render_widget(Paragraph::new(right_lines).block(right_block), body_chunks[1]);
    }

    // 3. Footer Status
    let footer_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.border))
        .title(Span::styled(" Status ", Style::default().fg(theme.accent).add_modifier(Modifier::BOLD)));

    let status_color = if app.status_msg.contains("failed") || app.status_msg.contains("failed:") || app.status_msg.contains("Code") {
        Color::Rgb(250, 80, 80)
    } else if app.status_msg.contains("Successfully") || app.status_msg.contains("Connected") {
        Color::Rgb(80, 250, 80)
    } else {
        theme.text_dim
    };

    let status_icon = if status_color == Color::Rgb(250, 80, 80) {
        app.glyphs.status_err
    } else if status_color == Color::Rgb(80, 250, 80) {
        app.glyphs.status_ok
    } else {
        app.glyphs.info
    };

    let footer_line = Line::from(vec![
        Span::styled(format!("{} ", status_icon), Style::default().fg(status_color)),
        Span::styled(&app.status_msg, Style::default().fg(status_color).add_modifier(Modifier::BOLD)),
    ]);

    f.render_widget(footer_block, chunks[2]);
    f.render_widget(Paragraph::new(footer_line), chunks[2].inner(Margin { horizontal: 1, vertical: 1 }));

    // Password overlay popup
    if app.show_password_overlay {
        if let Some(net) = filtered_nets.get(app.selected_network_idx) {
            let area = centered_rect_fixed(60, 13, f.area());
            f.render_widget(Clear, area);
            
            let overlay_block = Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(theme.accent))
                .title(Span::styled(" Wi-Fi Password Required ", Style::default().fg(theme.accent).add_modifier(Modifier::BOLD)));
            
            let inner_rect = overlay_block.inner(area);
            f.render_widget(overlay_block, area);

            let popup_chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(3), // 0: Details (SSID, Security)
                    Constraint::Length(3), // 1: Input text box
                    Constraint::Length(3), // 2: Operations controls text
                ])
                .split(inner_rect);

            let mut overlay_lines = Vec::new();
            overlay_lines.push(Line::from(vec![
                Span::styled("  SSID:     ", Style::default().fg(theme.text_dim)),
                Span::styled(&net.ssid, Style::default().fg(theme.text_main).add_modifier(Modifier::BOLD)),
            ]));
            overlay_lines.push(Line::from(vec![
                Span::styled("  Security: ", Style::default().fg(theme.text_dim)),
                Span::styled(&net.auth_algorithm, Style::default().fg(theme.text_main)),
            ]));
            f.render_widget(Paragraph::new(overlay_lines), popup_chunks[0]);

            let is_enterprise = net.auth_algorithm.contains("Enterprise") || net.auth_algorithm.contains("RSNA");
            let input_title = if is_enterprise && app.eap_prompt_username {
                " Enter EAP Username "
            } else if net.has_profile {
                " Enter Password (leave empty to use saved profile) "
            } else {
                " Enter Password "
            };
            let input_block = Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(theme.border_active))
                .title(Span::styled(input_title, Style::default().fg(theme.accent)));
            
            let display_pwd = if app.password_visible {
                app.password_box.text.clone()
            } else {
                "*".repeat(app.password_box.text.len())
            };

            let cursor_str = if app.password_box.cursor_pos == app.password_box.text.len() {
                "_"
            } else {
                ""
            };

            f.render_widget(
                Paragraph::new(Line::from(vec![
                    Span::styled(format!(" {}{}", display_pwd, cursor_str), Style::default().fg(theme.text_main)),
                ])).block(input_block),
                popup_chunks[1].inner(Margin { horizontal: 2, vertical: 0 }),
            );

            let mut controls_lines = Vec::new();
            controls_lines.push(Line::from(""));
            controls_lines.push(Line::from(vec![
                Span::styled("  Enter ", Style::default().fg(theme.accent).add_modifier(Modifier::BOLD)),
                Span::styled("Connect   ", Style::default().fg(theme.text_main)),
                Span::styled("Tab ", Style::default().fg(theme.accent).add_modifier(Modifier::BOLD)),
                Span::styled("Toggle visibility   ", Style::default().fg(theme.text_main)),
                Span::styled("Esc ", Style::default().fg(theme.accent).add_modifier(Modifier::BOLD)),
                Span::styled("Cancel", Style::default().fg(theme.text_main)),
            ]));
            f.render_widget(Paragraph::new(controls_lines), popup_chunks[2]);
        }
    }

    // Share Wi-Fi Network QR Code overlay
    if app.show_share_overlay {
        let area = centered_rect_fixed(50, 24, f.area());
        f.render_widget(Clear, area);
        
        let overlay_block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(theme.accent))
            .title(Span::styled(" Share Wi-Fi Network ", Style::default().fg(theme.accent).add_modifier(Modifier::BOLD)));
        
        let inner_rect = overlay_block.inner(area);
        f.render_widget(overlay_block, area);
        
        if app.share_prompt_password {
            let popup_chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(3),
                    Constraint::Length(3),
                    Constraint::Length(3),
                ])
                .split(inner_rect);
            
            let mut overlay_lines = Vec::new();
            overlay_lines.push(Line::from(vec![
                Span::styled("  SSID: ", Style::default().fg(theme.text_dim)),
                Span::styled(&app.share_ssid, Style::default().fg(theme.text_main).add_modifier(Modifier::BOLD)),
            ]));
            f.render_widget(Paragraph::new(overlay_lines), popup_chunks[0]);
            
            let input_block = Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(theme.border_active))
                .title(Span::styled(" Enter Password to Generate QR Code ", Style::default().fg(theme.accent)));
            
            let display_pwd = app.share_box.text.clone();
            let cursor_str = if app.share_box.cursor_pos == app.share_box.text.len() { "_" } else { "" };
            f.render_widget(
                Paragraph::new(Line::from(vec![
                    Span::styled(format!(" {}{}", display_pwd, cursor_str), Style::default().fg(theme.text_main)),
                ])).block(input_block),
                popup_chunks[1].inner(Margin { horizontal: 2, vertical: 0 }),
            );
            
            let mut controls_lines = Vec::new();
            controls_lines.push(Line::from(""));
            controls_lines.push(Line::from(vec![
                Span::styled("  Enter ", Style::default().fg(theme.accent).add_modifier(Modifier::BOLD)),
                Span::styled("Generate   ", Style::default().fg(theme.text_main)),
                Span::styled("Esc ", Style::default().fg(theme.accent).add_modifier(Modifier::BOLD)),
                Span::styled("Cancel", Style::default().fg(theme.text_main)),
            ]));
            f.render_widget(Paragraph::new(controls_lines), popup_chunks[2]);
        } else {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(2),
                    Constraint::Min(10),
                    Constraint::Length(2),
                ])
                .split(inner_rect);
            
            f.render_widget(
                Paragraph::new(Line::from(vec![
                    Span::styled("  SSID: ", Style::default().fg(theme.text_dim)),
                    Span::styled(&app.share_ssid, Style::default().fg(theme.text_main).add_modifier(Modifier::BOLD)),
                ])).alignment(ratatui::layout::Alignment::Center),
                chunks[0],
            );
            
            let mut qr_lines = Vec::new();
            for line in &app.share_qr_lines {
                qr_lines.push(Line::from(Span::styled(line.clone(), Style::default().bg(Color::White).fg(Color::Black))));
            }
            
            f.render_widget(
                Paragraph::new(qr_lines).alignment(ratatui::layout::Alignment::Center),
                chunks[1],
            );
            
            f.render_widget(
                Paragraph::new(Line::from(vec![
                    Span::styled("Press Esc / s / q to close this popup", Style::default().fg(theme.text_dim)),
                ])).alignment(ratatui::layout::Alignment::Center),
                chunks[2],
            );
        }
    }

    // Connect to Hidden Wi-Fi overlay
    if app.show_hidden_overlay {
        let area = centered_rect_fixed(60, 12, f.area());
        f.render_widget(Clear, area);
        
        let overlay_block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(theme.accent))
            .title(Span::styled(" Connect to Hidden Wi-Fi ", Style::default().fg(theme.accent).add_modifier(Modifier::BOLD)));
        
        let inner_rect = overlay_block.inner(area);
        f.render_widget(overlay_block, area);
        
        let popup_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),
                Constraint::Length(3),
                Constraint::Length(2),
            ])
            .split(inner_rect);
        
        let (step_title, input_title, mask_char) = match app.hidden_prompt_step {
            1 => ("  Step 1: Specify Network SSID Name", " Network SSID Name ", false),
            2 => ("  Step 2: Is the network secured? (y/n)", " Secured? [y = Yes, n = No] (Default: y) ", false),
            3 => ("  Step 3: Specify Network Password", " Network Password ", true),
            _ => ("", "", false),
        };
        
        f.render_widget(
            Paragraph::new(Line::from(vec![
                Span::styled(step_title, Style::default().fg(theme.text_main).add_modifier(Modifier::BOLD)),
            ])),
            popup_chunks[0].inner(Margin { horizontal: 1, vertical: 1 }),
        );
        
        let input_block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(theme.border_active))
            .title(Span::styled(input_title, Style::default().fg(theme.accent)));
        
        let display_val = if mask_char {
            "*".repeat(app.hidden_box.text.len())
        } else {
            app.hidden_box.text.clone()
        };
        let cursor_str = if app.hidden_box.cursor_pos == app.hidden_box.text.len() { "_" } else { "" };
        
        f.render_widget(
            Paragraph::new(Line::from(vec![
                Span::styled(format!(" {}{}", display_val, cursor_str), Style::default().fg(theme.text_main)),
            ])).block(input_block),
            popup_chunks[1].inner(Margin { horizontal: 2, vertical: 0 }),
        );
        
        let mut controls_lines = Vec::new();
        controls_lines.push(Line::from(vec![
            Span::styled("  Enter ", Style::default().fg(theme.accent).add_modifier(Modifier::BOLD)),
            Span::styled("Next / Submit   ", Style::default().fg(theme.text_main)),
            Span::styled("Esc ", Style::default().fg(theme.accent).add_modifier(Modifier::BOLD)),
            Span::styled("Cancel", Style::default().fg(theme.text_main)),
        ]));
        f.render_widget(Paragraph::new(controls_lines), popup_chunks[2]);
    }

    // Offline Saved Profiles overlay
    if app.show_profiles_overlay {
        let area = centered_rect_fixed(70, 20, f.area());
        f.render_widget(Clear, area);
        
        let overlay_block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(theme.accent))
            .title(Span::styled(" Saved Offline Profiles (Known Networks) ", Style::default().fg(theme.accent).add_modifier(Modifier::BOLD)));
        
        let inner_rect = overlay_block.inner(area);
        f.render_widget(overlay_block, area);
        
        let popup_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Min(1),
                Constraint::Length(3),
            ])
            .split(inner_rect);
        
        let mut list_lines = Vec::new();
        if app.profiles_list.is_empty() {
            list_lines.push(Line::from(""));
            list_lines.push(Line::from(Span::styled("  No saved wireless profiles found on this system.", Style::default().fg(theme.text_dim))));
        } else {
            for (idx, (name, _)) in app.profiles_list.iter().enumerate() {
                let is_selected = idx == app.profiles_selected_idx;
                let bullet = if is_selected { app.glyphs.play } else { app.glyphs.play_empty };
                let bullet_style = if is_selected {
                    Style::default().fg(theme.accent).add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(theme.text_dim)
                };
                let name_style = if is_selected {
                    Style::default().fg(theme.text_main).add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(theme.text_dim)
                };
                list_lines.push(Line::from(vec![
                    Span::styled(bullet, bullet_style),
                    Span::styled(name, name_style),
                ]));
            }
        }
        
        f.render_widget(Paragraph::new(list_lines), popup_chunks[0].inner(Margin { horizontal: 2, vertical: 1 }));
        
        let mut controls_lines = Vec::new();
        controls_lines.push(Line::from(vec![
            Span::styled("  Up/Down ", Style::default().fg(theme.accent).add_modifier(Modifier::BOLD)),
            Span::styled("Navigate   ", Style::default().fg(theme.text_main)),
            Span::styled("d / Del ", Style::default().fg(theme.accent).add_modifier(Modifier::BOLD)),
            Span::styled("Delete Profile   ", Style::default().fg(theme.text_main)),
            Span::styled("Esc / p ", Style::default().fg(theme.accent).add_modifier(Modifier::BOLD)),
            Span::styled("Close Manager", Style::default().fg(theme.text_main)),
        ]));
        f.render_widget(Paragraph::new(controls_lines), popup_chunks[1].inner(Margin { horizontal: 1, vertical: 1 }));
    }

    // Help overlay popup
    if app.show_help {
        let area = centered_rect_fixed(70, 25, f.area());
        f.render_widget(Clear, area);

        let help_block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(theme.accent))
            .title(Span::styled(" rWifi Keyboard Shortcuts ", Style::default().fg(theme.accent).add_modifier(Modifier::BOLD)));

        let mut help_lines = Vec::new();
        help_lines.push(Line::from(""));
        
        let add_shortcut = |key: &'static str, desc: &'static str, lines: &mut Vec<Line<'static>>, width: u16| {
            let prefix_width = 17; // 2 spaces + 12 key + 3 separator
            let max_desc_width = (width as usize).saturating_sub(prefix_width + 4);
            let desc_wrapped = wrap_text(desc, max_desc_width);

            for (i, line) in desc_wrapped.iter().enumerate() {
                if i == 0 {
                    lines.push(Line::from(vec![
                        Span::styled(format!("  {:<12}", key), Style::default().fg(theme.accent).add_modifier(Modifier::BOLD)),
                        Span::styled(" │ ", Style::default().fg(theme.border)),
                        Span::styled(line.clone(), Style::default().fg(theme.text_main)),
                    ]));
                } else {
                    lines.push(Line::from(vec![
                        Span::styled("              ", Style::default().fg(theme.accent)),
                        Span::styled(" │ ", Style::default().fg(theme.border)),
                        Span::styled(line.clone(), Style::default().fg(theme.text_main)),
                    ]));
                }
            }
        };

        add_shortcut("Tab", "Cycle focus between Station List and Info Panel", &mut help_lines, area.width);
        add_shortcut("Up / Down", "Navigate Wi-Fi networks", &mut help_lines, area.width);
        add_shortcut("k / j", "Navigate Wi-Fi networks (vim keys)", &mut help_lines, area.width);
        add_shortcut("Enter", "Connect to network (asks password if unsecured/no profile)", &mut help_lines, area.width);
        add_shortcut("d / Del", "Disconnect from active station or delete saved profile", &mut help_lines, area.width);
        add_shortcut("Space / r", "Force scan & refresh Wi-Fi networks list", &mut help_lines, area.width);
        add_shortcut("/", "Enter SSID filter/search mode", &mut help_lines, area.width);
        add_shortcut("t / T", "Toggle Wi-Fi radio hardware state (On/Off)", &mut help_lines, area.width);
        add_shortcut("s / S", "Share selected network credentials via QR Code", &mut help_lines, area.width);
        add_shortcut("p / P", "Manage saved offline profiles (known networks)", &mut help_lines, area.width);
        add_shortcut("h", "Toggle keyboard shortcuts overlay modal", &mut help_lines, area.width);
        add_shortcut("w / W", "Connect to a hidden network", &mut help_lines, area.width);
        add_shortcut("F1", "View README.md documentation", &mut help_lines, area.width);
        add_shortcut("F2", "View SUPPORT.md documentation", &mut help_lines, area.width);
        add_shortcut("F3", "View LICENSE.md documentation", &mut help_lines, area.width);
        add_shortcut("F4", "View COPYRIGHT.md documentation", &mut help_lines, area.width);
        add_shortcut("F5", "View PRIVACY.md documentation", &mut help_lines, area.width);
        add_shortcut("F6", "View SECURITY.md documentation", &mut help_lines, area.width);
        add_shortcut("F7", "View CONTRIBUTING.md documentation", &mut help_lines, area.width);
        add_shortcut("Esc / q", "Close active popups / Quit application", &mut help_lines, area.width);

        help_lines.push(Line::from(""));
        help_lines.push(Line::from(Span::styled("  Press Esc / q / h to close this help window.", Style::default().fg(theme.text_dim))));

        f.render_widget(Paragraph::new(help_lines).block(help_block).wrap(ratatui::widgets::Wrap { trim: false }), area);
    }

    // Documentation text viewer popup
    if let Some(ref filename) = app.show_markdown {
        let area = centered_rect(80, 80, f.area());
        f.render_widget(Clear, area);

        let doc_block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(theme.accent))
            .title(Span::styled(format!(" Documentation: {} (Press Esc/q to Close) ", filename), Style::default().fg(theme.accent).add_modifier(Modifier::BOLD)));

        let paragraph = Paragraph::new(app.markdown_lines.clone())
            .block(doc_block)
            .wrap(ratatui::widgets::Wrap { trim: true })
            .alignment(ratatui::layout::Alignment::Left)
            .scroll((app.markdown_scroll as u16, 0));

        f.render_widget(paragraph, area);
    }

    // Handle Mouse Selection Highlights & Clipboard Copy
    if let (Some(start), Some(end)) = (app.selection_start, app.selection_end) {
        let buf = f.buffer_mut();
        let width = buf.area.width;
        let height = buf.area.height;

        let (col1, row1) = start;
        let (col2, row2) = end;

        let is_selected = |x: u16, y: u16| -> bool {
            let (c1, r1) = (col1, row1);
            let (c2, r2) = (col2, row2);
            if r1 == r2 {
                y == r1 && x >= c1.min(c2) && x <= c1.max(c2)
            } else if r1 < r2 {
                (y == r1 && x >= c1) || (y > r1 && y < r2) || (y == r2 && x <= c2)
            } else {
                (y == r2 && x >= c2) || (y > r2 && y < r1) || (y == r1 && x <= c1)
            }
        };

        // 1. Draw Highlight
        for y in 0..height {
            for x in 0..width {
                if is_selected(x, y) {
                    let cell = &mut buf[(x, y)];
                    cell.set_bg(Color::Rgb(0, 120, 215));
                    cell.set_fg(Color::White);
                }
            }
        }

        // 2. Perform Copy on Release
        if app.selection_pending_copy {
            let mut selected_text = String::new();
            let mut current_row: Option<u16> = None;
            let mut current_line = String::new();

            for y in 0..height {
                for x in 0..width {
                    if is_selected(x, y) {
                        let cell = &buf[(x, y)];
                        if current_row != Some(y) {
                            if current_row.is_some() {
                                selected_text.push_str(current_line.trim_end());
                                selected_text.push('\n');
                                current_line.clear();
                            }
                            current_row = Some(y);
                        }
                        current_line.push_str(cell.symbol());
                    }
                }
            }
            if !current_line.is_empty() {
                selected_text.push_str(current_line.trim_end());
            }

            if !selected_text.is_empty() {
                let _ = win32::copy_text_to_clipboard(&selected_text);
                let truncated = if selected_text.len() > 30 {
                    format!("{}...", &selected_text[..27].replace('\n', " "))
                } else {
                    selected_text.replace('\n', " ")
                };
                app.status_msg = format!("📋 Copied selection to clipboard: {}", truncated);
                app.status_ttl = 6000;
            }

            app.selection_start = None;
            app.selection_end = None;
            app.selection_pending_copy = false;
        }
    }
}

// Layout helper
fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}

// Fixed-size layout helper
fn centered_rect_fixed(width: u16, height: u16, r: Rect) -> Rect {
    let x = r.x + (r.width.saturating_sub(width)) / 2;
    let y = r.y + (r.height.saturating_sub(height)) / 2;
    Rect {
        x,
        y,
        width: width.min(r.width),
        height: height.min(r.height),
    }
}

use ratatui::layout::Margin;

fn wrap_text(text: &str, max_width: usize) -> Vec<String> {
    let mut words = text.split_whitespace();
    let mut lines = Vec::new();
    let mut current_line = String::new();

    while let Some(word) = words.next() {
        if current_line.is_empty() {
            current_line.push_str(word);
        } else if current_line.len() + 1 + word.len() <= max_width {
            current_line.push(' ');
            current_line.push_str(word);
        } else {
            lines.push(current_line);
            current_line = word.to_string();
        }
    }
    if !current_line.is_empty() {
        lines.push(current_line);
    }
    lines
}

fn generate_qr_code_lines(ssid: &str, password: &str, auth_type: &str) -> Vec<String> {
    use qrcodegen::{QrCode, QrCodeEcc};
    let auth = match auth_type {
        "WPA2-Personal" | "WPA-Personal" | "WPA3-Personal" | "WPA2-Enterprise" | "WPA3-Enterprise" | "WPA2PSK" | "WPAPSK" | "WPA3SAE" => "WPA",
        "Open" | "none" => "nopass",
        "WEP" => "WEP",
        _ => "WPA",
    };
    let payload = if auth == "nopass" {
        format!("WIFI:T:nopass;S:{};;", ssid)
    } else {
        format!("WIFI:T:{};S:{};P:{};;", auth, ssid, password)
    };
    
    let qr = match QrCode::encode_text(&payload, QrCodeEcc::Medium) {
        Ok(code) => code,
        Err(_) => return vec!["Failed to generate QR code".to_string()],
    };
    
    let size = qr.size();
    let border = 2; // quiet zone
    let total_size = size + border * 2;
    
    let mut lines = Vec::new();
    
    for y_idx in (0..total_size).step_by(2) {
        let mut line = String::new();
        for x_idx in 0..total_size {
            let x = x_idx as i32 - border;
            let y1 = y_idx as i32 - border;
            let y2 = y_idx as i32 + 1 - border;
            
            let val1 = if x >= 0 && x < size && y1 >= 0 && y1 < size {
                qr.get_module(x, y1)
            } else {
                false
            };
            
            let val2 = if x >= 0 && x < size && y2 >= 0 && y2 < size {
                qr.get_module(x, y2)
            } else {
                false
            };
            
            let ch = match (val1, val2) {
                (true, true) => "█",
                (true, false) => "▀",
                (false, true) => "▄",
                (false, false) => " ",
            };
            line.push_str(ch);
        }
        lines.push(line);
    }
    
    lines
}
