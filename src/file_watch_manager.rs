use std::collections::HashMap;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::time::{Instant, SystemTime};
use log::{debug, info, warn, error};

/// Centralized file watching and update management
///
/// This manages file watching for all modes (Tail, Less, Events) to avoid:
/// - Duplicated polling logic
/// - Multiple reads of the same file
/// - Inconsistent state tracking
pub struct FileWatchManager {
    /// Watched files with their state
    files: HashMap<PathBuf, WatchedFile>,

    /// Subscribers per file - maps file path to list of subscribers
    subscribers: HashMap<PathBuf, Vec<FileSubscriber>>,
}

/// Represents a file being watched
pub struct WatchedFile {
    pub path: PathBuf,
    pub display_name: String,

    // File state
    pub last_position: u64,
    pub last_size: u64,
    pub last_modified: SystemTime,

    // Error handling
    pub consecutive_errors: u32,
    pub last_error_time: Option<Instant>,

    // Cached metadata
    pub total_lines: usize,
    pub total_bytes: u64,
}

/// Identifies a subscriber to file updates
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FileSubscriber {
    /// Tail mode instance
    Tail { mode_id: usize },
    /// Less mode instance
    Less { mode_id: usize },
    /// Events mode instance
    Events { mode_id: usize },
}

/// Types of updates that can occur to a watched file
#[derive(Debug, Clone)]
pub enum FileUpdate {
    /// New lines were added to the file
    NewLines {
        lines: Vec<String>,
        start_line: usize,
    },
    /// File was truncated (became smaller)
    Truncated,
    /// File was deleted
    Deleted,
    /// Error occurred while checking file
    Error(String),
}

impl FileWatchManager {
    /// Create a new file watch manager
    pub fn new() -> Self {
        info!("📂 FileWatchManager: Initializing");
        Self {
            files: HashMap::new(),
            subscribers: HashMap::new(),
        }
    }

    /// Subscribe to updates for a file
    ///
    /// If the file is not already being watched, it will be added to the watch list.
    /// Returns true if this is a new subscription, false if already subscribed.
    pub fn subscribe(&mut self, path: PathBuf, subscriber: FileSubscriber) -> Result<bool, std::io::Error> {
        debug!("📂 FileWatchManager: Subscribe request - Path: {:?}, Subscriber: {:?}", path, subscriber);

        // Add to subscribers list
        let subs = self.subscribers.entry(path.clone()).or_insert_with(Vec::new);
        if subs.contains(&subscriber) {
            debug!("📂 FileWatchManager: Already subscribed - {:?}", subscriber);
            return Ok(false);
        }
        subs.push(subscriber);

        // If this is the first subscriber, start watching the file
        if !self.files.contains_key(&path) {
            info!("📂 FileWatchManager: Starting to watch file - {:?}", path);
            self.start_watching(path)?;
        }

        Ok(true)
    }

    /// Unsubscribe from updates for a file
    ///
    /// If this was the last subscriber, stop watching the file.
    pub fn unsubscribe(&mut self, path: &Path, subscriber: FileSubscriber) {
        debug!("📂 FileWatchManager: Unsubscribe request - Path: {:?}, Subscriber: {:?}", path, subscriber);

        if let Some(subs) = self.subscribers.get_mut(path) {
            subs.retain(|s| s != &subscriber);

            // If no more subscribers, stop watching
            if subs.is_empty() {
                info!("📂 FileWatchManager: No more subscribers, stopping watch - {:?}", path);
                self.subscribers.remove(path);
                self.files.remove(path);
            }
        }
    }

    /// Start watching a file
    fn start_watching(&mut self, path: PathBuf) -> Result<(), std::io::Error> {
        let display_name = path.file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("?")
            .to_string();

        let metadata = std::fs::metadata(&path)?;
        let size = metadata.len();
        let modified = metadata.modified()?;

        // Don't count lines initially - we'll track incrementally
        // This avoids the expensive operation of reading large files
        let total_lines = 0;

        let watched = WatchedFile {
            path: path.clone(),
            display_name,
            last_position: size,
            last_size: size,
            last_modified: modified,
            consecutive_errors: 0,
            last_error_time: None,
            total_lines,
            total_bytes: size,
        };

        info!("📂 FileWatchManager: Started watching - {:?}, Size: {}",
              path, size);

        self.files.insert(path, watched);
        Ok(())
    }

    /// Check all watched files for updates
    ///
    /// Returns a map of file paths to their updates and subscribers that should be notified
    pub fn check_for_updates(&mut self) -> HashMap<PathBuf, (FileUpdate, Vec<FileSubscriber>)> {
        let mut updates = HashMap::new();

        debug!("📂 FileWatchManager: Checking {} files for updates", self.files.len());

        for (path, file) in self.files.iter_mut() {
            // Check if we should skip due to error backoff
            if let Some(last_error) = file.last_error_time {
                let backoff_secs = std::cmp::min(60, 2u64.pow(file.consecutive_errors.min(6)));
                let backoff_duration = std::time::Duration::from_secs(backoff_secs);

                if last_error.elapsed() < backoff_duration {
                    continue;
                }
            }

            // Check for updates (inline to avoid borrowing issues)
            match Self::check_file_update_static(file) {
                Ok(Some(update)) => {
                    // Get subscribers for this file
                    let subs = self.subscribers.get(path).cloned().unwrap_or_default();

                    debug!("📂 FileWatchManager: Update detected - Path: {:?}, Subscribers: {}",
                           path, subs.len());

                    updates.insert(path.clone(), (update, subs));
                }
                Ok(None) => {
                    // No update, reset error count
                    file.consecutive_errors = 0;
                    file.last_error_time = None;
                }
                Err(e) => {
                    // Error occurred
                    warn!("📂 FileWatchManager: Error checking file - {:?}: {}", path, e);

                    file.consecutive_errors += 1;
                    file.last_error_time = Some(Instant::now());

                    let subs = self.subscribers.get(path).cloned().unwrap_or_default();
                    let update = FileUpdate::Error(e.to_string());
                    updates.insert(path.clone(), (update, subs));
                }
            }
        }

        updates
    }

    /// Check a single file for updates (static method to avoid borrow issues)
    fn check_file_update_static(file: &mut WatchedFile) -> Result<Option<FileUpdate>, std::io::Error> {
        // Check if file exists
        if !file.path.exists() {
            error!("📂 FileWatchManager: File deleted - {:?}", file.path);
            return Ok(Some(FileUpdate::Deleted));
        }

        // Get current metadata
        let metadata = std::fs::metadata(&file.path)?;
        let current_size = metadata.len();
        let current_modified = metadata.modified()?;

        // Check if file was modified (either time changed or size increased)
        if current_modified <= file.last_modified && current_size <= file.last_size {
            debug!("📂 FileWatchManager: No changes in {:?} (size: {}, modified: {:?})",
                   file.path.file_name().unwrap_or_default(), current_size, current_modified);
            return Ok(None);
        }

        debug!("📂 FileWatchManager: Detected change in {:?} - Old: size={}, modified={:?} -> New: size={}, modified={:?}",
               file.path.file_name().unwrap_or_default(),
               file.last_size, file.last_modified,
               current_size, current_modified);

        // Check if file was truncated
        if current_size < file.last_size {
            info!("📂 FileWatchManager: File truncated - {:?}, Old size: {}, New size: {}",
                  file.path, file.last_size, current_size);

            file.last_size = current_size;
            file.last_position = 0;
            file.last_modified = current_modified;
            file.total_bytes = current_size;

            // Reset line count - we'll count incrementally as we read
            file.total_lines = 0;

            return Ok(Some(FileUpdate::Truncated));
        }

        // Read new lines
        let new_lines = Self::read_new_lines_static(&file.path, file.last_position)?;

        if new_lines.is_empty() {
            file.last_modified = current_modified;
            return Ok(None);
        }

        let start_line = file.total_lines;
        file.total_lines += new_lines.len();
        file.last_position = current_size;
        file.last_size = current_size;
        file.last_modified = current_modified;
        file.total_bytes = current_size;

        info!("📂 FileWatchManager: New lines - {:?}, Count: {}, Total lines now: {}",
              file.path, new_lines.len(), file.total_lines);

        Ok(Some(FileUpdate::NewLines {
            lines: new_lines,
            start_line,
        }))
    }

    /// Read new lines from a file starting at a given position (static helper)
    fn read_new_lines_static(path: &Path, from_position: u64) -> Result<Vec<String>, std::io::Error> {
        // Use our efficient reader with a limit to prevent memory issues
        use crate::file_tail_reader::FileTailReader;
        
        // Limit to 10,000 lines per poll to avoid memory bloat
        const MAX_LINES_PER_POLL: usize = 10_000;
        
        let (lines, truncated) = FileTailReader::read_from_position_limited(
            path, 
            from_position, 
            MAX_LINES_PER_POLL
        )?;
        
        if truncated {
            warn!("📂 FileWatchManager: Truncated read at {} lines for {:?} (file growing very fast)",
                  MAX_LINES_PER_POLL, path);
        }
        
        Ok(lines)
    }

    /// Count total lines in a file (static helper)
    /// NOTE: This is expensive for large files and should be avoided
    #[allow(dead_code)]
    fn count_lines_static(path: &Path) -> Result<usize, std::io::Error> {
        let file = File::open(path)?;
        let reader = BufReader::new(file);
        Ok(reader.lines().count())
    }

    /// Get information about a watched file
    pub fn get_file_info(&self, path: &Path) -> Option<&WatchedFile> {
        self.files.get(path)
    }

    /// Get all subscribers for a file
    pub fn get_subscribers(&self, path: &Path) -> Vec<FileSubscriber> {
        self.subscribers.get(path).cloned().unwrap_or_default()
    }

    /// Get all watched files
    pub fn watched_files(&self) -> impl Iterator<Item = (&PathBuf, &WatchedFile)> {
        self.files.iter()
    }

    /// Get total number of watched files
    pub fn watch_count(&self) -> usize {
        self.files.len()
    }
}

impl Default for FileWatchManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_subscribe_unsubscribe() {
        let mut manager = FileWatchManager::new();
        let mut temp_file = NamedTempFile::new().unwrap();
        writeln!(temp_file, "test line").unwrap();
        let path = temp_file.path().to_path_buf();

        let sub1 = FileSubscriber::Tail { mode_id: 0 };
        let sub2 = FileSubscriber::Events { mode_id: 1 };

        // Subscribe first subscriber
        assert!(manager.subscribe(path.clone(), sub1).is_ok());
        assert_eq!(manager.watch_count(), 1);

        // Subscribe second subscriber to same file
        assert!(manager.subscribe(path.clone(), sub2).is_ok());
        assert_eq!(manager.watch_count(), 1); // Still watching 1 file

        // Unsubscribe first, file should still be watched
        manager.unsubscribe(&path, sub1);
        assert_eq!(manager.watch_count(), 1);

        // Unsubscribe second, file should be unwatched
        manager.unsubscribe(&path, sub2);
        assert_eq!(manager.watch_count(), 0);
    }

    #[test]
    fn test_detect_new_lines() {
        let mut manager = FileWatchManager::new();
        let mut temp_file = NamedTempFile::new().unwrap();
        writeln!(temp_file, "line 1").unwrap();
        temp_file.flush().unwrap();
        let path = temp_file.path().to_path_buf();

        // Subscribe
        manager.subscribe(path.clone(), FileSubscriber::Tail { mode_id: 0 }).unwrap();

        // Initial check should show no updates
        let updates = manager.check_for_updates();
        assert!(updates.is_empty());

        // Add new line
        writeln!(temp_file, "line 2").unwrap();
        temp_file.flush().unwrap();

        // Should detect update
        let updates = manager.check_for_updates();
        assert_eq!(updates.len(), 1);

        if let Some((update, subs)) = updates.get(&path) {
            assert_eq!(subs.len(), 1);
            match update {
                FileUpdate::NewLines { lines, start_line } => {
                    assert_eq!(lines.len(), 1);
                    assert_eq!(lines[0], "line 2");
                    assert_eq!(*start_line, 1);
                }
                _ => panic!("Expected NewLines update"),
            }
        }
    }
}
