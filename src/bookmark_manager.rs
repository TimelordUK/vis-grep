use std::collections::HashMap;
use std::ops::Range;
use std::path::PathBuf;
use std::time::SystemTime;

/// A bookmark representing a specific line in a file
#[derive(Debug, Clone, PartialEq)]
pub struct Bookmark {
    /// Path to the file containing this bookmark
    pub file_path: PathBuf,
    /// Absolute line number in the file (0-indexed)
    pub absolute_line_number: usize,
    /// Cached content of the line for display
    pub line_content: String,
    /// When the bookmark was created
    pub created_at: SystemTime,
}

/// Manages bookmarks across files with support for sliding window contexts
#[derive(Debug)]
pub struct BookmarkManager {
    marks: HashMap<char, Bookmark>,
}

impl BookmarkManager {
    pub fn new() -> Self {
        Self {
            marks: HashMap::new(),
        }
    }

    /// Set a bookmark at the given mark character
    pub fn set_mark(
        &mut self,
        mark_char: char,
        file_path: PathBuf,
        absolute_line: usize,
        content: String,
    ) {
        self.marks.insert(
            mark_char,
            Bookmark {
                file_path,
                absolute_line_number: absolute_line,
                line_content: content,
                created_at: SystemTime::now(),
            },
        );
    }

    /// Get a bookmark by mark character
    pub fn get_mark(&self, mark_char: char) -> Option<&Bookmark> {
        self.marks.get(&mark_char)
    }

    /// Check if a mark exists and is within the given line range
    pub fn is_mark_in_range(&self, mark_char: char, valid_range: &Range<usize>) -> bool {
        self.marks
            .get(&mark_char)
            .map(|bookmark| valid_range.contains(&bookmark.absolute_line_number))
            .unwrap_or(false)
    }

    /// Remove bookmarks that are outside the valid range
    /// Returns the removed bookmarks for notification purposes
    pub fn prune_stale_marks(&mut self, valid_range: Range<usize>) -> Vec<(char, Bookmark)> {
        let mut removed = Vec::new();

        self.marks.retain(|&mark_char, bookmark| {
            if valid_range.contains(&bookmark.absolute_line_number) {
                true // Keep it
            } else {
                removed.push((mark_char, bookmark.clone()));
                false // Remove it
            }
        });

        removed
    }

    /// Remove all bookmarks for a specific file
    pub fn clear_file_marks(&mut self, file_path: &PathBuf) -> Vec<(char, Bookmark)> {
        let mut removed = Vec::new();

        self.marks.retain(|&mark_char, bookmark| {
            if &bookmark.file_path == file_path {
                removed.push((mark_char, bookmark.clone()));
                false // Remove it
            } else {
                true // Keep it
            }
        });

        removed
    }

    /// Get all bookmarks for a specific file
    pub fn get_file_marks(&self, file_path: &PathBuf) -> Vec<(char, &Bookmark)> {
        self.marks
            .iter()
            .filter(|(_, bookmark)| &bookmark.file_path == file_path)
            .map(|(&mark_char, bookmark)| (mark_char, bookmark))
            .collect()
    }

    /// Convert an absolute line number to a relative position in a buffer
    /// Returns None if the line is outside the buffer range
    pub fn absolute_to_relative(
        absolute_line: usize,
        buffer_start: usize,
        buffer_size: usize,
    ) -> Option<usize> {
        if absolute_line >= buffer_start && absolute_line < buffer_start + buffer_size {
            Some(absolute_line - buffer_start)
        } else {
            None
        }
    }

    /// Convert a relative buffer position to an absolute line number
    pub fn relative_to_absolute(relative_line: usize, buffer_start: usize) -> usize {
        buffer_start + relative_line
    }

    /// Remove a specific mark
    pub fn remove_mark(&mut self, mark_char: char) -> Option<Bookmark> {
        self.marks.remove(&mark_char)
    }

    /// Get all marks
    pub fn get_all_marks(&self) -> Vec<(char, &Bookmark)> {
        self.marks.iter().map(|(&c, b)| (c, b)).collect()
    }

    /// Clear all marks
    pub fn clear_all(&mut self) {
        self.marks.clear();
    }
}

impl Default for BookmarkManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn test_file_path() -> PathBuf {
        PathBuf::from("/tmp/test.log")
    }

    fn other_file_path() -> PathBuf {
        PathBuf::from("/tmp/other.log")
    }

    #[test]
    fn test_set_and_get_mark() {
        let mut manager = BookmarkManager::new();

        manager.set_mark('a', test_file_path(), 100, "test line".to_string());

        let bookmark = manager.get_mark('a').unwrap();
        assert_eq!(bookmark.absolute_line_number, 100);
        assert_eq!(bookmark.line_content, "test line");
        assert_eq!(bookmark.file_path, test_file_path());
    }

    #[test]
    fn test_overwrite_mark() {
        let mut manager = BookmarkManager::new();

        manager.set_mark('a', test_file_path(), 100, "first".to_string());
        manager.set_mark('a', test_file_path(), 200, "second".to_string());

        let bookmark = manager.get_mark('a').unwrap();
        assert_eq!(bookmark.absolute_line_number, 200);
        assert_eq!(bookmark.line_content, "second");
    }

    #[test]
    fn test_is_mark_in_range() {
        let mut manager = BookmarkManager::new();

        manager.set_mark('a', test_file_path(), 500, "test".to_string());

        // Mark is in range
        assert!(manager.is_mark_in_range('a', &(400..600)));

        // Mark is at start of range
        assert!(manager.is_mark_in_range('a', &(500..600)));

        // Mark is just before end of range
        assert!(manager.is_mark_in_range('a', &(400..501)));

        // Mark is outside range (too low)
        assert!(!manager.is_mark_in_range('a', &(600..700)));

        // Mark is outside range (too high)
        assert!(!manager.is_mark_in_range('a', &(300..400)));

        // Non-existent mark
        assert!(!manager.is_mark_in_range('z', &(400..600)));
    }

    #[test]
    fn test_prune_stale_marks() {
        let mut manager = BookmarkManager::new();

        // Set marks at various positions
        manager.set_mark('a', test_file_path(), 100, "line 100".to_string());
        manager.set_mark('b', test_file_path(), 500, "line 500".to_string());
        manager.set_mark('c', test_file_path(), 900, "line 900".to_string());

        // Buffer contains lines 400-600
        let removed = manager.prune_stale_marks(400..600);

        // Marks 'a' and 'c' should be removed, 'b' should remain
        assert_eq!(removed.len(), 2);
        assert!(removed.iter().any(|(c, _)| *c == 'a'));
        assert!(removed.iter().any(|(c, _)| *c == 'c'));

        // Only 'b' should remain
        assert!(manager.get_mark('b').is_some());
        assert!(manager.get_mark('a').is_none());
        assert!(manager.get_mark('c').is_none());
    }

    #[test]
    fn test_sliding_window_scenario() {
        let mut manager = BookmarkManager::new();

        // Initial state: buffer shows lines 9000-10000, mark at 9500
        let _buffer_start = 9000;
        let buffer_size = 1000;

        manager.set_mark(
            'a',
            test_file_path(),
            9500,
            "important line".to_string(),
        );

        // Buffer slides to 9400-10400 (400 new lines added)
        let new_buffer_start = 9400;
        let removed = manager.prune_stale_marks(new_buffer_start..new_buffer_start + buffer_size);

        // Mark should still be valid
        assert_eq!(removed.len(), 0);
        assert!(manager.get_mark('a').is_some());

        // Convert to relative position in new buffer
        let relative = BookmarkManager::absolute_to_relative(9500, new_buffer_start, buffer_size);
        assert_eq!(relative, Some(100)); // 9500 - 9400 = 100

        // Buffer slides to 10000-11000 (mark scrolled out)
        let new_buffer_start = 10000;
        let removed = manager.prune_stale_marks(new_buffer_start..new_buffer_start + buffer_size);

        // Mark should be pruned
        assert_eq!(removed.len(), 1);
        assert_eq!(removed[0].0, 'a');
        assert!(manager.get_mark('a').is_none());
    }

    #[test]
    fn test_absolute_to_relative_conversion() {
        // Buffer starts at line 1000, size 100 (lines 1000-1099)
        let buffer_start = 1000;
        let buffer_size = 100;

        // Line at start of buffer
        assert_eq!(
            BookmarkManager::absolute_to_relative(1000, buffer_start, buffer_size),
            Some(0)
        );

        // Line in middle of buffer
        assert_eq!(
            BookmarkManager::absolute_to_relative(1050, buffer_start, buffer_size),
            Some(50)
        );

        // Line at end of buffer
        assert_eq!(
            BookmarkManager::absolute_to_relative(1099, buffer_start, buffer_size),
            Some(99)
        );

        // Line before buffer
        assert_eq!(
            BookmarkManager::absolute_to_relative(999, buffer_start, buffer_size),
            None
        );

        // Line after buffer
        assert_eq!(
            BookmarkManager::absolute_to_relative(1100, buffer_start, buffer_size),
            None
        );
    }

    #[test]
    fn test_relative_to_absolute_conversion() {
        let buffer_start = 5000;

        assert_eq!(BookmarkManager::relative_to_absolute(0, buffer_start), 5000);
        assert_eq!(
            BookmarkManager::relative_to_absolute(50, buffer_start),
            5050
        );
        assert_eq!(
            BookmarkManager::relative_to_absolute(999, buffer_start),
            5999
        );
    }

    #[test]
    fn test_multiple_files() {
        let mut manager = BookmarkManager::new();

        manager.set_mark('a', test_file_path(), 100, "file1 line".to_string());
        manager.set_mark('b', other_file_path(), 200, "file2 line".to_string());

        // Both marks exist
        assert!(manager.get_mark('a').is_some());
        assert!(manager.get_mark('b').is_some());

        // Get marks for specific file
        let file1_marks = manager.get_file_marks(&test_file_path());
        assert_eq!(file1_marks.len(), 1);
        assert_eq!(file1_marks[0].0, 'a');

        let file2_marks = manager.get_file_marks(&other_file_path());
        assert_eq!(file2_marks.len(), 1);
        assert_eq!(file2_marks[0].0, 'b');
    }

    #[test]
    fn test_clear_file_marks() {
        let mut manager = BookmarkManager::new();

        manager.set_mark('a', test_file_path(), 100, "file1 line1".to_string());
        manager.set_mark('b', test_file_path(), 200, "file1 line2".to_string());
        manager.set_mark('c', other_file_path(), 300, "file2 line".to_string());

        // Clear marks for test_file
        let removed = manager.clear_file_marks(&test_file_path());

        assert_eq!(removed.len(), 2);
        assert!(removed.iter().any(|(c, _)| *c == 'a'));
        assert!(removed.iter().any(|(c, _)| *c == 'b'));

        // Only mark 'c' should remain
        assert!(manager.get_mark('a').is_none());
        assert!(manager.get_mark('b').is_none());
        assert!(manager.get_mark('c').is_some());
    }

    #[test]
    fn test_remove_specific_mark() {
        let mut manager = BookmarkManager::new();

        manager.set_mark('a', test_file_path(), 100, "test".to_string());
        manager.set_mark('b', test_file_path(), 200, "test".to_string());

        let removed = manager.remove_mark('a');
        assert!(removed.is_some());
        assert_eq!(removed.unwrap().absolute_line_number, 100);

        assert!(manager.get_mark('a').is_none());
        assert!(manager.get_mark('b').is_some());

        // Removing non-existent mark
        let removed = manager.remove_mark('z');
        assert!(removed.is_none());
    }

    #[test]
    fn test_get_all_marks() {
        let mut manager = BookmarkManager::new();

        manager.set_mark('a', test_file_path(), 100, "test1".to_string());
        manager.set_mark('b', test_file_path(), 200, "test2".to_string());
        manager.set_mark('c', other_file_path(), 300, "test3".to_string());

        let all_marks = manager.get_all_marks();
        assert_eq!(all_marks.len(), 3);
    }

    #[test]
    fn test_clear_all() {
        let mut manager = BookmarkManager::new();

        manager.set_mark('a', test_file_path(), 100, "test1".to_string());
        manager.set_mark('b', test_file_path(), 200, "test2".to_string());

        manager.clear_all();

        assert_eq!(manager.get_all_marks().len(), 0);
        assert!(manager.get_mark('a').is_none());
        assert!(manager.get_mark('b').is_none());
    }

    #[test]
    fn test_edge_case_empty_range() {
        let mut manager = BookmarkManager::new();

        manager.set_mark('a', test_file_path(), 100, "test".to_string());

        // Empty range
        let removed = manager.prune_stale_marks(0..0);
        assert_eq!(removed.len(), 1);
    }

    #[test]
    fn test_edge_case_mark_at_range_boundary() {
        let mut manager = BookmarkManager::new();

        manager.set_mark('a', test_file_path(), 100, "test".to_string());
        manager.set_mark('b', test_file_path(), 199, "test".to_string());

        // Range is 100..200 (includes 100, excludes 200)
        let removed = manager.prune_stale_marks(100..200);

        // Mark at 100 should be kept, mark at 199 should be kept
        assert_eq!(removed.len(), 0);

        // Mark at 200 would be excluded
        manager.set_mark('c', test_file_path(), 200, "test".to_string());
        let removed = manager.prune_stale_marks(100..200);
        assert_eq!(removed.len(), 1);
        assert_eq!(removed[0].0, 'c');
    }
}
