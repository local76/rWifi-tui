//! UI module initialization, themes, and primary drawing entry point.
//!
//! **Taxonomy Classification**: UI Rendering (UI Dispatcher).

use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Clear},
};
use crate::app::AppState;

pub mod widgets;
pub mod overlays;
pub mod layout;
pub mod accent_gauge;

pub use library::interface::app::design::prelude::{ThemeColors, get_theme, parse_markdown_to_lines};

pub fn draw_ui(f: &mut Frame, app: &mut AppState, theme: &ThemeColors) {
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

        let area = layout::centered_rect(80, 50, size);
        f.render_widget(Clear, area);
        f.render_widget(p, area);
        return;
    }

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Header
            Constraint::Min(5),    // Body
            Constraint::Length(3), // Footer
        ])
        .split(size);

    // Render static panels
    widgets::draw_header(f, app, theme, chunks[0]);
    
    let body_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(60), // Left: Station List
            Constraint::Percentage(40), // Right: Info Panel
        ])
        .split(chunks[1]);

    widgets::draw_network_list(f, app, theme, body_chunks[0]);
    widgets::draw_info_panel(f, app, theme, body_chunks[1]);
    widgets::draw_footer(f, app, theme, chunks[2]);

    // Render overlay modals in order of precedence
    if app.show_password_overlay {
        overlays::draw_password_overlay(f, app, theme);
    }
    if app.show_share_overlay {
        overlays::draw_share_overlay(f, app, theme);
    }
    if app.show_hidden_overlay {
        overlays::draw_hidden_overlay(f, app, theme);
    }
    if app.show_profiles_overlay {
        overlays::draw_profiles_overlay(f, app, theme);
    }
    if app.show_help {
        overlays::draw_help_overlay(f, app, theme);
    }
    if app.show_markdown.is_some() {
        overlays::draw_markdown_overlay(f, app, theme);
    }

    // Render mouse text selection highlight overlay (drawn directly on buffer cells)
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
                let _ = crate::win32::copy_text_to_clipboard(&selected_text);
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
