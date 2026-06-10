use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    widgets::Widget,
};
use library::ui::colors::AccentColors;

/// A custom progress gauge styled with the system DWM accent color.
/// Supports a `focused` flag to make it first-class for tab/focus-based UIs.
/// When `focused`, the bar uses the accent color; when inactive it falls back
/// to dim_color so the whole panel can visually de-emphasize together with
/// borders and other accent widgets.
#[derive(Debug, Clone)]
pub struct AccentGauge {
    pub progress: f64,
    pub label: String,
    pub accent_color: Color,
    pub dim_color: Color,
    pub use_unicode: bool,
    pub focused: bool,
}

impl AccentGauge {
    pub fn new(
        progress: f64,
        label: &str,
        accent_color: Color,
        dim_color: Color,
        use_unicode: bool,
        focused: bool,
    ) -> Self {
        Self {
            progress: progress.clamp(0.0, 1.0),
            label: label.to_string(),
            accent_color,
            dim_color,
            use_unicode,
            focused,
        }
    }

    /// First-class constructor using the bundled `AccentColors`.
    #[allow(dead_code)]
    pub fn new_with_colors(
        progress: f64,
        label: &str,
        colors: &AccentColors,
        use_unicode: bool,
        focused: bool,
    ) -> Self {
        Self::new(progress, label, colors.accent, colors.dim, use_unicode, focused)
    }
}

impl Widget for AccentGauge {
    fn render(self, area: Rect, buf: &mut ratatui::prelude::Buffer) {
        if area.height < 1 || area.width < 4 {
            return;
        }

        // Render starting bracket
        buf[(area.x, area.y)]
            .set_char('[')
            .set_fg(self.dim_color);

        // Render ending bracket
        buf[(area.x + area.width - 1, area.y)]
            .set_char(']')
            .set_fg(self.dim_color);

        // Render gauge fill/empty
        let bar_width = area.width - 2;
        let filled_cells = (self.progress * bar_width as f64).round() as u16;
        let fill_symbol = if self.use_unicode { '█' } else { '#' };
        let empty_symbol = if self.use_unicode { '░' } else { '-' };

        let fill_color = if self.focused { self.accent_color } else { self.dim_color };
        for i in 0..bar_width {
            let cx = area.x + 1 + i;
            let cell = &mut buf[(cx, area.y)];
            if i < filled_cells {
                cell.set_char(fill_symbol).set_fg(fill_color);
            } else {
                cell.set_char(empty_symbol).set_fg(self.dim_color);
            }
        }

        // Overlay centered text
        let pct_text = format!(" {:.0}% ", self.progress * 100.0);
        let label_text = if !self.label.is_empty() {
            format!(" {} -{}", self.label, pct_text)
        } else {
            pct_text
        };

        let label_len = label_text.chars().count() as u16;
        let text_style = if self.focused {
            Style::default().add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(self.dim_color)
        };
        if label_len < area.width {
            let start_x = (area.width - label_len) / 2;
            for (i, c) in label_text.chars().enumerate() {
                let cx = area.x + start_x + i as u16;
                if cx >= area.x && cx < area.x + area.width {
                    let cell = &mut buf[(cx, area.y)];
                    cell.set_char(c);
                    cell.set_style(text_style);
                }
            }
        }
    }
}
