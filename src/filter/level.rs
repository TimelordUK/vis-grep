use crate::log_parser::{LogLevel, LogLevelDetector};
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct LogLevelFilter {
    pub active: bool,
    pub minimum_level: LogLevel,
    pub show_unknown: bool,
    pub apply_to_preview: bool,
    pub level_counts: HashMap<LogLevel, usize>,
}

impl LogLevelFilter {
    pub fn new() -> Self {
        Self {
            active: false,
            minimum_level: LogLevel::Info,
            show_unknown: true,
            apply_to_preview: false,
            level_counts: HashMap::new(),
        }
    }

    /// Check if a line should be shown based on current filter settings
    pub fn should_show_line(&self, line: &str, detector: &LogLevelDetector) -> bool {
        if !self.active {
            return true;  // Filter disabled, show everything
        }

        let detected_level = detector.detect(line);

        match detected_level {
            LogLevel::Unknown => self.show_unknown,
            _ => detected_level.severity() >= self.minimum_level.severity()
        }
    }

    /// Cycle through all filter modes: ALL -> INFO+ -> WARN+ -> ERROR -> ALL
    pub fn cycle_mode(&mut self) {
        if !self.active {
            // ALL -> INFO+
            self.active = true;
            self.minimum_level = LogLevel::Info;
        } else {
            match self.minimum_level {
                LogLevel::Info => {
                    // INFO+ -> WARN+
                    self.minimum_level = LogLevel::Warn;
                }
                LogLevel::Warn => {
                    // WARN+ -> ERROR
                    self.minimum_level = LogLevel::Error;
                }
                LogLevel::Error => {
                    // ERROR -> ALL
                    self.active = false;
                }
                _ => {
                    // Fallback to ALL
                    self.active = false;
                }
            }
        }
    }

    /// Cycle backwards through all filter modes: ALL -> ERROR -> WARN+ -> INFO+ -> ALL
    pub fn cycle_mode_backwards(&mut self) {
        if !self.active {
            // ALL -> ERROR
            self.active = true;
            self.minimum_level = LogLevel::Error;
        } else {
            match self.minimum_level {
                LogLevel::Error => {
                    // ERROR -> WARN+
                    self.minimum_level = LogLevel::Warn;
                }
                LogLevel::Warn => {
                    // WARN+ -> INFO+
                    self.minimum_level = LogLevel::Info;
                }
                LogLevel::Info => {
                    // INFO+ -> ALL
                    self.active = false;
                }
                _ => {
                    // Fallback to ALL
                    self.active = false;
                }
            }
        }
    }

    /// Cycle through filter levels: INFO -> WARN -> ERROR -> INFO
    pub fn cycle_level(&mut self) {
        self.minimum_level = match self.minimum_level {
            LogLevel::Info => LogLevel::Warn,
            LogLevel::Warn => LogLevel::Error,
            LogLevel::Error => LogLevel::Info,
            _ => LogLevel::Info,
        };
    }

    /// Cycle backwards through filter levels
    pub fn cycle_level_backwards(&mut self) {
        self.minimum_level = match self.minimum_level {
            LogLevel::Info => LogLevel::Error,
            LogLevel::Error => LogLevel::Warn,
            LogLevel::Warn => LogLevel::Info,
            _ => LogLevel::Info,
        };
    }

    /// Update level counts from a line
    pub fn update_counts(&mut self, line: &str, detector: &LogLevelDetector) {
        let level = detector.detect(line);
        *self.level_counts.entry(level).or_insert(0) += 1;
    }

    /// Clear level counts
    pub fn clear_counts(&mut self) {
        self.level_counts.clear();
    }

    /// Get a display string for the current filter mode
    pub fn display_mode(&self) -> &'static str {
        if !self.active {
            return "ALL";
        }

        match self.minimum_level {
            LogLevel::Info => "INFO+",
            LogLevel::Warn => "WARN+",
            LogLevel::Error => "ERROR",
            _ => "ALL",
        }
    }
}

impl Default for LogLevelFilter {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_filter_inactive() {
        let filter = LogLevelFilter::new();
        let detector = LogLevelDetector::new(vec![]);

        // When inactive, all lines should be shown
        assert!(filter.should_show_line("[ERROR] test", &detector));
        assert!(filter.should_show_line("[INFO] test", &detector));
        assert!(filter.should_show_line("random text", &detector));
    }

    #[test]
    fn test_filter_error_level() {
        let mut filter = LogLevelFilter::new();
        filter.active = true;
        filter.minimum_level = LogLevel::Error;

        let detector = LogLevelDetector::new(vec![]);

        // Should show errors and fatal
        assert!(filter.should_show_line("[ERROR] test", &detector));
        assert!(filter.should_show_line("[FATAL] test", &detector));

        // Should hide info, warn, debug
        assert!(!filter.should_show_line("[INFO] test", &detector));
        assert!(!filter.should_show_line("[WARN] test", &detector));
        assert!(!filter.should_show_line("[DEBUG] test", &detector));
    }

    #[test]
    fn test_cycle_level() {
        let mut filter = LogLevelFilter::new();
        filter.minimum_level = LogLevel::Info;

        filter.cycle_level();
        assert_eq!(filter.minimum_level, LogLevel::Warn);

        filter.cycle_level();
        assert_eq!(filter.minimum_level, LogLevel::Error);

        filter.cycle_level();
        assert_eq!(filter.minimum_level, LogLevel::Info);
    }

    #[test]
    fn test_display_mode() {
        let mut filter = LogLevelFilter::new();

        assert_eq!(filter.display_mode(), "ALL");

        filter.active = true;
        filter.minimum_level = LogLevel::Info;
        assert_eq!(filter.display_mode(), "INFO+");

        filter.minimum_level = LogLevel::Warn;
        assert_eq!(filter.display_mode(), "WARN+");

        filter.minimum_level = LogLevel::Error;
        assert_eq!(filter.display_mode(), "ERROR");
    }
}
