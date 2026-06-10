//! Keyboard input event handler.
//!
//! **Taxonomy Classification**: Controller (Keyboard Controller).

use crossterm::event::KeyCode;
use crate::app::AppState;
use crate::ui::{ThemeColors, parse_markdown_to_lines};
use crate::win32;

pub fn handle_keypress(app: &mut AppState, code: KeyCode, theme: &ThemeColors) {
    if crate::app::keys_overlays::handle_overlay_keypress(app, code, theme) {
        return;
    }

    // Global override for F1-F7 documentation keys
    if let Some(name) = library::apps::chrome::open_embedded_markdown(code) {
        app.show_help = false;
        app.show_markdown = Some(name.to_string());
        app.markdown_lines = parse_markdown_to_lines(doc_content(name), theme);
        app.markdown_scroll = 0;
        return;
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

    if app.show_markdown.is_some() {
        match code {
            KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('Q') => {
                app.show_markdown = None;
                app.markdown_lines.clear();
            }
            KeyCode::Up | KeyCode::Char('k') => {
                app.markdown_scroll = app.markdown_scroll.saturating_sub(1);
            }
            KeyCode::Down | KeyCode::Char('j') => {
                let (_, term_h) = crossterm::terminal::size().unwrap_or((110, 38));
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

    let filtered_nets: Vec<win32::WlanNetwork> = if app.search_active {
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
                crate::app::FocusedSection::NetworkList => crate::app::FocusedSection::InfoPanel,
                crate::app::FocusedSection::InfoPanel => crate::app::FocusedSection::NetworkList,
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
                            win32::show_toast_notification("scout - Disconnected", &format!("Disconnected from {}", net.ssid));
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
                            win32::show_toast_notification("scout - Connected", &format!("Connected to {}", net.ssid));
                            win32::log_windows_event("scout", 4, 1001, &format!("Successfully connected to {}", net.ssid));
                            app.scan_wifi(false);
                        }
                        Err(e) => {
                            app.set_status(format!("Failed to connect: {}", e), true);
                            win32::show_toast_notification("scout - Connection Failed", &format!("Failed to connect to {}: {}", net.ssid, e));
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
                                "scout - Radio Toggled",
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
                        app.share_qr_lines = crate::ui::layout::generate_qr_code_lines(&net.ssid, &password, &net.auth_algorithm);
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
                    app.share_qr_lines = crate::ui::layout::generate_qr_code_lines(&net.ssid, "", &net.auth_algorithm);
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

/// Resolve a doc filename (e.g. "README.md") to its embedded markdown content.
/// Each app embeds its own copy of the 7 docs at compile time, so this lookup
/// lives in the app crate (not in library).
fn doc_content(name: &str) -> &'static str {
    match name {
        "README.md" => crate::README_CONTENT,
        "SUPPORT.md" => crate::SUPPORT_CONTENT,
        "LICENSE.md" => crate::LICENSE_CONTENT,
        "COPYRIGHT.md" => crate::COPYRIGHT_CONTENT,
        "PRIVACY.md" => crate::PRIVACY_CONTENT,
        "SECURITY.md" => crate::SECURITY_CONTENT,
        "CONTRIBUTING.md" => crate::CONTRIBUTING_CONTENT,
        _ => "",
    }
}
