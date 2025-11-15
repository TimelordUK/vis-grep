use regex::Regex;
use once_cell::sync::Lazy;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LogLevel {
    Trace,
    Debug,
    Info,
    Warn,
    Error,
    Fatal,
    Unknown,
}

impl LogLevel {
    /// Get severity score for ordering (higher = more severe)
    pub fn severity(&self) -> u8 {
        match self {
            LogLevel::Trace => 0,
            LogLevel::Debug => 1,
            LogLevel::Info => 2,
            LogLevel::Warn => 3,
            LogLevel::Error => 4,
            LogLevel::Fatal => 5,
            LogLevel::Unknown => 0,
        }
    }
}

pub struct LogLevelDetector {
    patterns: Vec<LevelPattern>,
}

struct LevelPattern {
    regex: Regex,
    level: LogLevel,
}

// Common log level patterns
static DEFAULT_PATTERNS: Lazy<Vec<(&str, LogLevel)>> = Lazy::new(|| {
    vec![
        // Bracketed formats: [LEVEL]
        (r"\[TRACE\]", LogLevel::Trace),
        (r"\[DEBUG\]", LogLevel::Debug),
        (r"\[INFO\]", LogLevel::Info),
        (r"\[WARN(?:ING)?\]", LogLevel::Warn),
        (r"\[ERROR\]", LogLevel::Error),
        (r"\[FATAL\]", LogLevel::Fatal),
        (r"\[CRITICAL\]", LogLevel::Fatal),

        // Colon-separated: LEVEL:
        (r"(?i)\bTRACE:", LogLevel::Trace),
        (r"(?i)\bDEBUG:", LogLevel::Debug),
        (r"(?i)\bINFO:", LogLevel::Info),
        (r"(?i)\bWARN(?:ING)?:", LogLevel::Warn),
        (r"(?i)\bERROR:", LogLevel::Error),
        (r"(?i)\bFATAL:", LogLevel::Fatal),
        (r"(?i)\bCRITICAL:", LogLevel::Fatal),

        // Angular brackets: <level>
        (r"<trace>", LogLevel::Trace),
        (r"<debug>", LogLevel::Debug),
        (r"<info>", LogLevel::Info),
        (r"<warn(?:ing)?>", LogLevel::Warn),
        (r"<error>", LogLevel::Error),
        (r"<fatal>", LogLevel::Fatal),

        // Short forms: INF, WRN, ERR
        (r"\bTRC\b", LogLevel::Trace),
        (r"\bDBG\b", LogLevel::Debug),
        (r"\bINF\b", LogLevel::Info),
        (r"\bWRN\b", LogLevel::Warn),
        (r"\bERR\b", LogLevel::Error),
        (r"\bFTL\b", LogLevel::Fatal),

        // Syslog style: level as word at start
        (r"^trace\s", LogLevel::Trace),
        (r"^debug\s", LogLevel::Debug),
        (r"^info\s", LogLevel::Info),
        (r"^warn(?:ing)?\s", LogLevel::Warn),
        (r"^error\s", LogLevel::Error),
        (r"^fatal\s", LogLevel::Fatal),
    ]
});

impl LogLevelDetector {
    pub fn new() -> Self {
        let patterns = DEFAULT_PATTERNS
            .iter()
            .filter_map(|(pattern, level)| {
                Regex::new(pattern).ok().map(|regex| LevelPattern {
                    regex,
                    level: *level,
                })
            })
            .collect();

        Self { patterns }
    }

    /// Detect log level from a line of text
    pub fn detect(&self, line: &str) -> LogLevel {
        for pattern in &self.patterns {
            if pattern.regex.is_match(line) {
                return pattern.level;
            }
        }
        LogLevel::Unknown
    }

    /// Detect log level and return the matched text range for highlighting
    pub fn detect_with_range(&self, line: &str) -> (LogLevel, Option<(usize, usize)>) {
        for pattern in &self.patterns {
            if let Some(m) = pattern.regex.find(line) {
                return (pattern.level, Some((m.start(), m.end())));
            }
        }
        (LogLevel::Unknown, None)
    }
}

impl Default for LogLevelDetector {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bracketed_levels() {
        let detector = LogLevelDetector::new();

        assert_eq!(detector.detect("[INFO] Starting application"), LogLevel::Info);
        assert_eq!(detector.detect("[ERROR] Connection failed"), LogLevel::Error);
        assert_eq!(detector.detect("[WARN] Low memory"), LogLevel::Warn);
        assert_eq!(detector.detect("[DEBUG] Processing"), LogLevel::Debug);
    }

    #[test]
    fn test_colon_separated() {
        let detector = LogLevelDetector::new();

        assert_eq!(detector.detect("INFO: Server started"), LogLevel::Info);
        assert_eq!(detector.detect("ERROR: Failed to connect"), LogLevel::Error);
        assert_eq!(detector.detect("WARN: Deprecated"), LogLevel::Warn);
    }

    #[test]
    fn test_short_forms() {
        let detector = LogLevelDetector::new();

        assert_eq!(detector.detect("INF Application ready"), LogLevel::Info);
        assert_eq!(detector.detect("ERR Network timeout"), LogLevel::Error);
        assert_eq!(detector.detect("WRN Cache miss"), LogLevel::Warn);
    }

    #[test]
    fn test_unknown() {
        let detector = LogLevelDetector::new();

        assert_eq!(detector.detect("Random log message"), LogLevel::Unknown);
    }
}
