/// Represents a sliding window view into a file
///
/// This encapsulates the complexity of managing:
/// - Buffer-relative line numbers (what the user sees: 1-1000)
/// - Absolute file line numbers (actual position in file: 1-2343)
/// - Conversions between the two
/// - Scroll positions
#[derive(Debug, Clone)]
pub struct BufferWindow {
    /// The actual line content in the buffer
    pub lines: Vec<String>,

    /// Absolute line number where this buffer starts in the file (0-indexed)
    pub start_line: usize,

    /// Total number of lines in the source file
    pub total_file_lines: usize,
}

impl BufferWindow {
    /// Create a new buffer window
    pub fn new(lines: Vec<String>, start_line: usize, total_file_lines: usize) -> Self {
        Self {
            lines,
            start_line,
            total_file_lines,
        }
    }

    /// Create an empty buffer
    pub fn empty() -> Self {
        Self {
            lines: Vec::new(),
            start_line: 0,
            total_file_lines: 0,
        }
    }

    /// Get the number of lines in the buffer
    pub fn len(&self) -> usize {
        self.lines.len()
    }

    /// Check if buffer is empty
    pub fn is_empty(&self) -> bool {
        self.lines.is_empty()
    }

    /// Get the absolute line number where the buffer ends (exclusive)
    pub fn end_line(&self) -> usize {
        self.start_line + self.lines.len()
    }

    /// Convert a buffer-relative line (0-indexed) to absolute file line (0-indexed)
    /// Returns None if the relative line is out of bounds
    pub fn relative_to_absolute(&self, relative_line: usize) -> Option<usize> {
        if relative_line < self.lines.len() {
            Some(self.start_line + relative_line)
        } else {
            None
        }
    }

    /// Convert an absolute file line (0-indexed) to buffer-relative line (0-indexed)
    /// Returns None if the absolute line is not in the current buffer window
    pub fn absolute_to_relative(&self, absolute_line: usize) -> Option<usize> {
        if absolute_line >= self.start_line && absolute_line < self.end_line() {
            Some(absolute_line - self.start_line)
        } else {
            None
        }
    }

    /// Check if an absolute line number is within this buffer window
    pub fn contains_absolute_line(&self, absolute_line: usize) -> bool {
        absolute_line >= self.start_line && absolute_line < self.end_line()
    }

    /// Get a line by buffer-relative index (0-indexed)
    pub fn get_line(&self, relative_line: usize) -> Option<&String> {
        self.lines.get(relative_line)
    }

    /// Get buffer info as a formatted string for debugging
    pub fn info(&self) -> String {
        format!(
            "Buffer: {} lines (file lines {}-{} of {})",
            self.lines.len(),
            self.start_line + 1,
            self.end_line(),
            self.total_file_lines
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_conversions() {
        let lines = vec!["line1".to_string(), "line2".to_string(), "line3".to_string()];
        let buffer = BufferWindow::new(lines, 100, 200);

        // Relative to absolute
        assert_eq!(buffer.relative_to_absolute(0), Some(100));
        assert_eq!(buffer.relative_to_absolute(1), Some(101));
        assert_eq!(buffer.relative_to_absolute(2), Some(102));
        assert_eq!(buffer.relative_to_absolute(3), None);

        // Absolute to relative
        assert_eq!(buffer.absolute_to_relative(100), Some(0));
        assert_eq!(buffer.absolute_to_relative(101), Some(1));
        assert_eq!(buffer.absolute_to_relative(102), Some(2));
        assert_eq!(buffer.absolute_to_relative(99), None);
        assert_eq!(buffer.absolute_to_relative(103), None);

        // Contains
        assert!(buffer.contains_absolute_line(100));
        assert!(buffer.contains_absolute_line(102));
        assert!(!buffer.contains_absolute_line(99));
        assert!(!buffer.contains_absolute_line(103));
    }

    #[test]
    fn test_bounds() {
        let buffer = BufferWindow::new(vec!["a".to_string()], 50, 100);

        assert_eq!(buffer.len(), 1);
        assert_eq!(buffer.start_line, 50);
        assert_eq!(buffer.end_line(), 51);
        assert_eq!(buffer.total_file_lines, 100);
    }
}
