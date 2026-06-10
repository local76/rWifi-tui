use crossterm::event::KeyCode;
use crate::app::AppState;
use crate::ui::ThemeColors;
use crate::ui::layout::generate_qr_code_lines;
use crate::win32;

/// Handles overlay keyboard input. Returns `true` if an overlay was active and handled the keypress.
pub fn handle_overlay_keypress(app: &mut AppState, code: KeyCode, _theme: &ThemeColors) -> bool {
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
        return true;
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
        return true;
    }

    if app.show_profiles_overlay {
        match code {
            KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('Q') | KeyCode::Char('p') | KeyCode::Char('P') => {
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
            KeyCode::Char('d') | KeyCode::Char('D') | KeyCode::Delete
                if !app.profiles_list.is_empty() => {
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
            _ => {}
        }
        return true;
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
                            return true;
                        }
                        app.eap_username = text_val;
                        app.password_box.clear();
                        app.eap_prompt_username = false;
                        app.password_visible = false;
                        app.set_status("Username saved. Enter password.".to_string(), false);
                        return true;
                    }
                    
                    let is_empty = text_val.trim().is_empty();
                    if !is_enterprise && is_empty && !net.has_profile {
                        app.set_status("Password cannot be empty for a new secured network.".to_string(), true);
                        return true;
                    }
                    
                    app.show_password_overlay = false;
                    app.password_box.active = false;
                    app.password_box.clear();
                    
                    if is_enterprise {
                        app.set_status(format!("Connecting to enterprise network {}...", net.ssid), false);
                        match win32::connect_to_enterprise_wifi(&net.ssid, &app.eap_username, &text_val, &net) {
                            Ok(_) => {
                                app.set_status(format!("Successfully connected to enterprise {}", net.ssid), false);
                                win32::show_toast_notification("scout - Connected", &format!("Connected to {}", net.ssid));
                                win32::log_windows_event("scout", 4, 1001, &format!("Successfully connected to {}", net.ssid));
                                app.scan_wifi(false);
                            }
                            Err(err) => {
                                app.set_status(format!("Failed to connect: {}", err), true);
                                win32::show_toast_notification("scout - Connection Failed", &format!("Failed to connect to {}: {}", net.ssid, err));
                                win32::log_windows_event("scout", 1, 1002, &format!("Failed to connect to {}: {}", net.ssid, err));
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
                                win32::show_toast_notification("scout - Connected", &format!("Connected to {}", net.ssid));
                                win32::log_windows_event("scout", 4, 1001, &format!("Successfully connected to {}", net.ssid));
                                app.scan_wifi(false);
                            }
                            Err(err) => {
                                app.set_status(format!("Failed to connect: {}", err), true);
                                win32::show_toast_notification("scout - Connection Failed", &format!("Failed to connect to {}: {}", net.ssid, err));
                                win32::log_windows_event("scout", 1, 1002, &format!("Failed to connect to {}: {}", net.ssid, err));
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
            other => {
                if other == KeyCode::Tab {
                    app.password_visible = !app.password_visible;
                } else {
                    app.password_box.handle_key(other);
                }
            }
        }
        return true;
    }

    false
}
