use eframe::egui::Color32;
use serde::{Deserialize, Serialize};
use super::LogLevel;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LogColorPreset {
    Vibrant,
    Subtle,
    Monochrome,
}

impl Default for LogColorPreset {
    fn default() -> Self {
        Self::Vibrant
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogColorScheme {
    pub trace: String,
    pub debug: String,
    pub info: String,
    pub warn: String,
    pub error: String,
    pub fatal: String,
    pub unknown: String,
}

impl LogColorScheme {
    /// Vibrant color scheme (default) - high contrast, colorful
    pub fn vibrant() -> Self {
        Self {
            trace: "#6B7280".to_string(),      // Dim gray
            debug: "#60A5FA".to_string(),      // Light blue
            info: "#D1D5DB".to_string(),       // Light gray (default text)
            warn: "#FBBF24".to_string(),       // Yellow/Orange
            error: "#EF4444".to_string(),      // Red
            fatal: "#DC2626".to_string(),      // Bright red
            unknown: "#9CA3AF".to_string(),    // Medium gray
        }
    }

    /// Subtle color scheme - muted colors, less distracting
    pub fn subtle() -> Self {
        Self {
            trace: "#6B7280".to_string(),      // Dim gray
            debug: "#93C5FD".to_string(),      // Softer blue
            info: "#D1D5DB".to_string(),       // Light gray (default text)
            warn: "#FCD34D".to_string(),       // Softer yellow
            error: "#F87171".to_string(),      // Softer red
            fatal: "#EF4444".to_string(),      // Medium red
            unknown: "#9CA3AF".to_string(),    // Medium gray
        }
    }

    /// Monochrome scheme - shades of gray with red for errors only
    pub fn monochrome() -> Self {
        Self {
            trace: "#4B5563".to_string(),      // Dark gray
            debug: "#6B7280".to_string(),      // Medium-dark gray
            info: "#9CA3AF".to_string(),       // Medium gray
            warn: "#D1D5DB".to_string(),       // Light gray (brighter than info)
            error: "#F87171".to_string(),      // Softer red (from Subtle theme)
            fatal: "#EF4444".to_string(),      // Medium red (toned down from bright)
            unknown: "#9CA3AF".to_string(),    // Medium gray
        }
    }

    /// Create from preset
    pub fn from_preset(preset: LogColorPreset) -> Self {
        match preset {
            LogColorPreset::Vibrant => Self::vibrant(),
            LogColorPreset::Subtle => Self::subtle(),
            LogColorPreset::Monochrome => Self::monochrome(),
        }
    }
}

impl Default for LogColorScheme {
    fn default() -> Self {
        Self::vibrant()
    }
}

impl LogColorScheme {
    /// Get color for a specific log level
    pub fn get_color(&self, level: LogLevel) -> Color32 {
        let hex = match level {
            LogLevel::Trace => &self.trace,
            LogLevel::Debug => &self.debug,
            LogLevel::Info => &self.info,
            LogLevel::Warn => &self.warn,
            LogLevel::Error => &self.error,
            LogLevel::Fatal => &self.fatal,
            LogLevel::Unknown => &self.unknown,
        };

        Self::parse_hex_color(hex).unwrap_or(Color32::WHITE)
    }

    /// Parse hex color string (#RRGGBB or #RRGGBBAA)
    fn parse_hex_color(hex: &str) -> Option<Color32> {
        let hex = hex.trim_start_matches('#');

        if hex.len() == 6 {
            // #RRGGBB
            let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
            let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
            let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
            Some(Color32::from_rgb(r, g, b))
        } else if hex.len() == 8 {
            // #RRGGBBAA
            let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
            let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
            let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
            let a = u8::from_str_radix(&hex[6..8], 16).ok()?;
            Some(Color32::from_rgba_unmultiplied(r, g, b, a))
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_hex_color() {
        assert_eq!(
            LogColorScheme::parse_hex_color("#FF0000"),
            Some(Color32::from_rgb(255, 0, 0))
        );

        assert_eq!(
            LogColorScheme::parse_hex_color("#00FF00"),
            Some(Color32::from_rgb(0, 255, 0))
        );

        assert_eq!(
            LogColorScheme::parse_hex_color("#0000FF80"),
            Some(Color32::from_rgba_unmultiplied(0, 0, 255, 128))
        );
    }

    #[test]
    fn test_get_color() {
        let scheme = LogColorScheme::default();

        // Should return colors without panicking
        scheme.get_color(LogLevel::Error);
        scheme.get_color(LogLevel::Warn);
        scheme.get_color(LogLevel::Info);
    }
}
