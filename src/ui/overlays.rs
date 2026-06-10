#![allow(clippy::vec_init_then_push)]
//! Overlay popups and modal dialogues drawing functions.
//!
//! **Taxonomy Classification**: UI Rendering (UI Overlays).

use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Margin},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Clear},
};
use crate::app::AppState;
use crate::ui::ThemeColors;
use crate::ui::utils::{centered_rect, centered_rect_fixed, wrap_text};
use crate::win32;

pub fn draw_password_overlay(f: &mut Frame, app: &mut AppState, theme: &ThemeColors) {
    let filtered_nets: Vec<win32::WlanNetwork> = if app.search_active {
        app.networks
            .iter()
            .filter(|n| n.ssid.to_lowercase().contains(&app.search_box.text.to_lowercase()))
            .cloned()
            .collect()
    } else {
        app.networks.clone()
    };

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

pub fn draw_share_overlay(f: &mut Frame, app: &mut AppState, theme: &ThemeColors) {
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

pub fn draw_hidden_overlay(f: &mut Frame, app: &mut AppState, theme: &ThemeColors) {
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

pub fn draw_profiles_overlay(f: &mut Frame, app: &mut AppState, theme: &ThemeColors) {
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

pub fn draw_help_overlay(f: &mut Frame, _app: &mut AppState, theme: &ThemeColors) {
    let area = centered_rect_fixed(70, 25, f.area());
    f.render_widget(Clear, area);

    let help_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.accent))
        .title(Span::styled(" scout Keyboard Shortcuts ", Style::default().fg(theme.accent).add_modifier(Modifier::BOLD)));

    let mut help_lines = Vec::new();
    help_lines.push(Line::from(""));
    
    let mut add_shortcut = |key: &'static str, desc: &'static str| {
        let prefix_width = 17;
        let max_desc_width = (area.width as usize).saturating_sub(prefix_width + 4);
        let desc_wrapped = wrap_text(desc, max_desc_width);

        for (i, line) in desc_wrapped.iter().enumerate() {
            if i == 0 {
                help_lines.push(Line::from(vec![
                    Span::styled(format!("  {:<12}", key), Style::default().fg(theme.accent).add_modifier(Modifier::BOLD)),
                    Span::styled(" │ ", Style::default().fg(theme.border)),
                    Span::styled(line.clone(), Style::default().fg(theme.text_main)),
                ]));
            } else {
                help_lines.push(Line::from(vec![
                    Span::styled("              ", Style::default().fg(theme.accent)),
                    Span::styled(" │ ", Style::default().fg(theme.border)),
                    Span::styled(line.clone(), Style::default().fg(theme.text_main)),
                ]));
            }
        }
    };

    add_shortcut("Tab", "Cycle focus between Station List and Info Panel");
    add_shortcut("Up / Down", "Navigate Wi-Fi networks");
    add_shortcut("k / j", "Navigate Wi-Fi networks (vim keys)");
    add_shortcut("Enter", "Connect to network (asks password if unsecured/no profile)");
    add_shortcut("d / Del", "Disconnect from active station or delete saved profile");
    add_shortcut("Space / r", "Force scan & refresh Wi-Fi networks list");
    add_shortcut("/", "Enter SSID filter/search mode");
    add_shortcut("t / T", "Toggle Wi-Fi radio hardware state (On/Off)");
    add_shortcut("s / S", "Share selected network credentials via QR Code");
    add_shortcut("p / P", "Manage saved offline profiles (known networks)");
    add_shortcut("h", "Toggle keyboard shortcuts overlay modal");
    add_shortcut("w / W", "Connect to a hidden network");
    add_shortcut("F1", "View README.md documentation");
    add_shortcut("F2", "View SUPPORT.md documentation");
    add_shortcut("F3", "View LICENSE.md documentation");
    add_shortcut("F4", "View COPYRIGHT.md documentation");
    add_shortcut("F5", "View PRIVACY.md documentation");
    add_shortcut("F6", "View SECURITY.md documentation");
    add_shortcut("F7", "View CONTRIBUTING.md documentation");
    add_shortcut("Esc / q", "Close active popups / Quit application");

    help_lines.push(Line::from(""));
    help_lines.push(Line::from(Span::styled("  Press Esc / q / h to close this help window.", Style::default().fg(theme.text_dim))));

    f.render_widget(Paragraph::new(help_lines).block(help_block).wrap(ratatui::widgets::Wrap { trim: false }), area);
}

pub fn draw_markdown_overlay(f: &mut Frame, app: &mut AppState, theme: &ThemeColors) {
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
}
