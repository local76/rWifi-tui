//! Layout and QR generation rendering helpers.
//!
//! **Taxonomy Classification**: UI Rendering (UI Utilities).

use ratatui::layout::Rect;

#[allow(unused_imports)]
pub use library::interface::app::design::prelude::{centered_rect, wrap_text};

/// Fixed-size layout helper to center a popup.
pub fn centered_rect_fixed(width: u16, height: u16, r: Rect) -> Rect {
    let x = r.x + (r.width.saturating_sub(width)) / 2;
    let y = r.y + (r.height.saturating_sub(height)) / 2;
    Rect {
        x,
        y,
        width: width.min(r.width),
        height: height.min(r.height),
    }
}

/// Generates a payload-ready QR code matching WiFi credentials format:
/// `WIFI:T:WPA;S:SSID;P:PASSWORD;;`
pub fn generate_qr_code_lines(ssid: &str, password: &str, auth_type: &str) -> Vec<String> {
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
            let x = x_idx - border;
            let y1 = y_idx - border;
            let y2 = y_idx + 1 - border;
            
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

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::layout::Rect;

    #[test]
    fn test_centered_rect_fixed() {
        let parent = Rect::new(0, 0, 100, 100);
        let centered = centered_rect_fixed(20, 10, parent);
        assert_eq!(centered.x, 40);
        assert_eq!(centered.y, 45);
        assert_eq!(centered.width, 20);
        assert_eq!(centered.height, 10);

        // Clamps to parent
        let oversized = centered_rect_fixed(150, 150, parent);
        assert_eq!(oversized.width, 100);
        assert_eq!(oversized.height, 100);
    }

    #[test]
    fn test_generate_qr_code_lines() {
        let lines = generate_qr_code_lines("TestSSID", "TestPassword", "WPA2-Personal");
        assert!(!lines.is_empty());
        // Verify we got a line that isn't a failure message
        assert_ne!(lines[0], "Failed to generate QR code");

        let open_lines = generate_qr_code_lines("OpenSSID", "", "Open");
        assert!(!open_lines.is_empty());
        assert_ne!(open_lines[0], "Failed to generate QR code");
    }
}

