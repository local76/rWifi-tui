//! Mouse input event handler.
//!
//! **Taxonomy Classification**: Controller (Mouse Controller).

use crate::app::AppState;
use crate::win32;

pub fn handle_mouse(app: &mut AppState, mouse_event: crossterm::event::MouseEvent) {
    let (term_w, term_h) = crossterm::terminal::size().unwrap_or((100, 35));

    // Calculate layout regions matching draw_ui
    let chunks = ratatui::layout::Layout::default()
        .direction(ratatui::layout::Direction::Vertical)
        .constraints([
            ratatui::layout::Constraint::Length(3), // Header
            ratatui::layout::Constraint::Min(5),    // Body
            ratatui::layout::Constraint::Length(3), // Footer
        ])
        .split(ratatui::layout::Rect::new(0, 0, term_w, term_h));

    let body_chunks = ratatui::layout::Layout::default()
        .direction(ratatui::layout::Direction::Horizontal)
        .constraints([
            ratatui::layout::Constraint::Percentage(60), // Left: Station List
            ratatui::layout::Constraint::Percentage(40), // Right: Info Panel
        ])
        .split(chunks[1]);

    match mouse_event.kind {
        crossterm::event::MouseEventKind::Down(crossterm::event::MouseButton::Left) => {
            let mut clicked_btn = false;
            
            // Check Quit button
            if let Some((btn_y, btn_start, btn_end)) = app.quit_btn_bounds
                && mouse_event.row == btn_y && mouse_event.column >= btn_start && mouse_event.column < btn_end {
                    app.should_quit = true;
                    clicked_btn = true;
                }
            
            // Check Help button
            if !clicked_btn
                && let Some((btn_y, btn_start, btn_end)) = app.help_btn_bounds
                    && mouse_event.row == btn_y && mouse_event.column >= btn_start && mouse_event.column < btn_end {
                        app.show_help = !app.show_help;
                        app.set_status(if app.show_help {
                            "Help overlay active. Press ESC/q to close.".to_string()
                        } else {
                            "Help overlay closed.".to_string()
                        }, false);
                        clicked_btn = true;
                    }

            // Check Click inside Network List
            if !clicked_btn && !app.show_help && app.show_markdown.is_none() && !app.show_password_overlay && !app.show_hidden_overlay && !app.show_share_overlay && !app.show_profiles_overlay {
                let list_area = body_chunks[0];
                let is_inside_list = mouse_event.column > list_area.x 
                    && mouse_event.column < list_area.x + list_area.width - 1
                    && mouse_event.row > list_area.y 
                    && mouse_event.row < list_area.y + list_area.height - 1;

                if is_inside_list {
                    let clicked_row = (mouse_event.row - list_area.y - 1) as usize;
                    let filtered_len = if app.search_active {
                        app.networks
                            .iter()
                            .filter(|n| n.ssid.to_lowercase().contains(&app.search_box.text.to_lowercase()))
                            .count()
                    } else {
                        app.networks.len()
                    };

                    if clicked_row < filtered_len {
                        app.selected_network_idx = clicked_row;
                        app.focus = crate::app::FocusedSection::NetworkList;
                    }
                    clicked_btn = true;
                }
            }
            
            if !clicked_btn {
                if mouse_event.row <= 2 {
                    if let Some(cursor_pos) = win32::query_cursor_pos()
                        && let Some(rect) = win32::get_window_rect() {
                            app.drag_active = true;
                            app.drag_start_cursor = Some(cursor_pos);
                            app.drag_start_window = Some((rect.left, rect.top));
                        }
                } else {
                    app.selection_start = Some((mouse_event.column, mouse_event.row));
                    app.selection_end = Some((mouse_event.column, mouse_event.row));
                    app.selection_pending_copy = false;
                }
            }
        }
        crossterm::event::MouseEventKind::Drag(crossterm::event::MouseButton::Left) => {
            if app.drag_active {
                if let (Some(start_cursor), Some(start_window)) = (app.drag_start_cursor, app.drag_start_window)
                    && let Some(curr_cursor) = win32::query_cursor_pos() {
                        let dx = curr_cursor.0 - start_cursor.0;
                        let dy = curr_cursor.1 - start_cursor.1;
                        win32::set_window_pos(start_window.0 + dx, start_window.1 + dy);
                    }
            } else if app.selection_start.is_some() {
                app.selection_end = Some((mouse_event.column, mouse_event.row));
            }
        }
        crossterm::event::MouseEventKind::Up(crossterm::event::MouseButton::Left) => {
            if app.drag_active {
                app.drag_active = false;
                app.drag_start_cursor = None;
                app.drag_start_window = None;
            } else if let (Some(start), Some(end)) = (app.selection_start, app.selection_end) {
                let dx = (start.0 as i32 - end.0 as i32).abs();
                let dy = (start.1 as i32 - end.1 as i32).abs();
                
                // Set copy only if drag is greater than 1 horizontal cell or 0 vertical cells (micro-jitter guard)
                if dx > 1 || dy > 0 {
                    app.selection_pending_copy = true;
                } else {
                    app.selection_start = None;
                    app.selection_end = None;
                }
            }
        }
        crossterm::event::MouseEventKind::ScrollUp => {
            if app.show_markdown.is_some() {
                app.markdown_scroll = app.markdown_scroll.saturating_sub(3);
            }
        }
        crossterm::event::MouseEventKind::ScrollDown
            if app.show_markdown.is_some() => {
                let inner_h = ((term_h * 80) / 100).saturating_sub(2) as usize;
                let max_scroll = app.markdown_lines.len().saturating_sub(inner_h);
                if app.markdown_scroll < max_scroll {
                    app.markdown_scroll = (app.markdown_scroll + 3).min(max_scroll);
                }
            }
        _ => {}
    }
}
