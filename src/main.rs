use arboard::Clipboard;
use clap::{Parser, Subcommand};
use eframe::egui;
use log::{debug, info, warn};
use std::collections::HashMap;
use std::path::PathBuf;
use std::time::Instant;
use std::sync::Arc;

mod config;
mod input_handler;
mod preview;
mod search;
mod grep_mode;
mod tail_mode;
mod splitter;
mod tail_layout;
mod theme;
mod filter;
mod log_parser;
mod widgets;
mod bookmark_manager;
mod buffer_window;
mod file_watch_manager;
mod file_tail_reader;
mod memory_monitor;
mod string_cache;
mod bounded_cache;
mod idle_monitor;

use config::Config;
use input_handler::{InputHandler, NavigationCommand};
use preview::FilePreview;
use search::{SearchEngine, SearchResult};
use splitter::{Splitter, SplitterAxis};
use tail_layout::TailLayout;
use theme::Theme;
use bookmark_manager::BookmarkManager;
use buffer_window::BufferWindow;
use file_watch_manager::{FileWatchManager, FileSubscriber, FileUpdate};
use memory_monitor::MemoryMonitor;
use idle_monitor::IdleMonitor;

// ============================================================================
// Command-Line Arguments
// ============================================================================

/// VisGrep - Fast visual search and log monitoring tool
#[derive(Parser, Debug)]
#[command(name = "vis-grep")]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,

    /// Start in tail mode (same as 'tail' subcommand)
    #[arg(short = 'f', long = "follow")]
    follow: bool,

    /// Load a tail layout file
    #[arg(long = "tail-layout", short = 'l', value_name = "FILE")]
    tail_layout: Option<PathBuf>,

    /// Files to tail/follow (when using -f flag)
    #[arg(value_name = "FILES")]
    files: Vec<PathBuf>,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Tail/follow mode - monitor files like 'tail -f'
    Tail {
        /// Files to monitor
        #[arg(required = true, value_name = "FILES")]
        files: Vec<PathBuf>,
    },
}

/// Startup configuration for the app
struct StartupConfig {
    mode: AppMode,
    tail_files: Vec<PathBuf>,
    tail_layout: Option<PathBuf>,
}

impl Default for StartupConfig {
    fn default() -> Self {
        Self {
            mode: AppMode::Grep,
            tail_files: Vec::new(),
            tail_layout: None,
        }
    }
}

// ============================================================================
// Application Mode Types
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AppMode {
    Grep,
    Tail,
    Test, // Minimal test mode to debug splitter
}

// ============================================================================
// Grep Mode State
// ============================================================================

struct GrepState {
    search_path: String,
    file_pattern: String,
    search_query: String,
    case_sensitive: bool,
    use_regex: bool,
    recursive: bool,
    file_age_hours: Option<u64>,

    search_engine: SearchEngine,
    results: Vec<SearchResult>,
    selected_result: Option<usize>,

    searching: bool,
    results_filter: String,
    collapsing_state: HashMap<usize, bool>,
    last_search_time: Instant,
    pending_search: bool,

    // FIX message highlighting pattern
    fix_highlight_pattern: String,
    
    // Font settings
    font_size: f32,
}

impl GrepState {
    fn new(config: &Config) -> Self {
        Self {
            search_path: VisGrepApp::expand_tilde(
                std::env::current_dir()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .as_ref(),
            ),
            file_pattern: String::from("*.log"),
            search_query: String::new(),
            case_sensitive: false,
            use_regex: true,
            recursive: true,
            file_age_hours: None,

            search_engine: SearchEngine::new(),
            results: Vec::new(),
            selected_result: None,

            searching: false,
            results_filter: String::new(),
            collapsing_state: HashMap::new(),
            last_search_time: Instant::now(),
            pending_search: false,

            fix_highlight_pattern: String::new(),
            font_size: config.ui.font_size,
        }
    }
}

// ============================================================================
// Tail Mode State
// ============================================================================

use std::collections::VecDeque;
use std::fs::File;
use std::io::{BufRead, BufReader, Seek, SeekFrom};

#[derive(Debug, Clone, Copy, PartialEq)]
enum ThrottleState {
    Normal,
    Throttled { skip_ratio: f32 },
    Paused { reason: ThrottleReason },
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum ThrottleReason {
    TooFast,
    UserPaused,
    BufferFull,
}

struct TailedFile {
    // Identity
    path: PathBuf,
    display_name: String,
    display_name_arc: Arc<str>,

    // File monitoring
    last_size: u64,
    last_position: u64,

    // Activity tracking
    is_active: bool,
    last_activity: Instant,
    lines_since_last_read: usize,

    // Throttling
    paused: bool,
    throttle_state: ThrottleState,

    // Statistics
    total_lines_read: usize,
    total_bytes_read: u64,

    // Log level tracking for recent activity
    level_counts_since_last_read: HashMap<log_parser::LogLevel, usize>,

    // Group membership
    group_id: Option<String>,
    
    // Error tracking for recovery
    consecutive_errors: u32,
    last_error_time: Option<Instant>,
    
    // Cached display strings to avoid repeated allocations
    cached_path_string: String,
    cached_size_kb_string: String,
}

impl TailedFile {
    fn new(path: PathBuf) -> std::io::Result<Self> {
        // Resolve to absolute path
        let absolute_path = if path.is_absolute() {
            path
        } else {
            std::env::current_dir()?.join(&path)
        };
        
        let display_name = absolute_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown")
            .to_string();

        // Get initial file size without keeping handle open
        let metadata = std::fs::metadata(&absolute_path)?;
        let size = metadata.len();

        let display_name_arc = Arc::from(display_name.as_str());
        let cached_path_string = absolute_path.to_string_lossy().to_string();
        let cached_size_kb_string = format!("{:.1} KB", size as f64 / 1024.0);
        
        Ok(Self {
            path: absolute_path,
            display_name,
            display_name_arc,
            last_size: size,
            last_position: size, // Start at end (like tail -f)
            is_active: false,
            last_activity: Instant::now(),
            lines_since_last_read: 0,
            paused: false,
            throttle_state: ThrottleState::Normal,
            total_lines_read: 0,
            total_bytes_read: 0,
            level_counts_since_last_read: HashMap::new(),
            group_id: None,
            consecutive_errors: 0,
            last_error_time: None,
            cached_path_string,
            cached_size_kb_string,
        })
    }

    fn check_for_updates(&mut self) -> std::io::Result<Vec<String>> {
        // Skip if we're in error backoff period
        if let Some(last_error) = self.last_error_time {
            let backoff_duration = std::time::Duration::from_secs(
                std::cmp::min(60, 2u64.pow(self.consecutive_errors.min(6)))
            );
            if last_error.elapsed() < backoff_duration {
                return Ok(vec![]);
            }
        }
        
        match self.check_for_updates_impl() {
            Ok(lines) => {
                // Reset error tracking on success
                if self.consecutive_errors > 0 {
                    info!("File {} recovered after {} errors", self.display_name, self.consecutive_errors);
                    self.consecutive_errors = 0;
                    self.last_error_time = None;
                }
                Ok(lines)
            }
            Err(e) => {
                self.consecutive_errors += 1;
                self.last_error_time = Some(Instant::now());
                
                if self.consecutive_errors == 1 || self.consecutive_errors % 10 == 0 {
                    info!("Error reading {} (attempt {}): {}", self.display_name, self.consecutive_errors, e);
                }
                
                // Return error only on first attempt, otherwise silently skip
                if self.consecutive_errors == 1 {
                    Err(e)
                } else {
                    Ok(vec![])
                }
            }
        }
    }
    
    fn check_for_updates_impl(&mut self) -> std::io::Result<Vec<String>> {
        // For network shares, always try to read from last position
        // to avoid stale metadata issues
        let mut file = File::open(&self.path)?;
        
        // First, seek to our last known position to see if there's new data
        file.seek(SeekFrom::Start(self.last_position))?;
        
        // Try to read some data
        let reader = BufReader::new(file);
        let new_lines: Vec<String> = reader.lines().filter_map(|l| l.ok()).collect();
        
        if !new_lines.is_empty() {
            // We got new data, update our position
            let lines_read = new_lines.len();
            self.total_lines_read += lines_read;
            
            // Update position based on what we actually read
            // Note: This is approximate but avoids needing to re-seek
            let approx_bytes: usize = new_lines.iter().map(|l| l.len() + 1).sum();
            self.last_position += approx_bytes as u64;
            self.total_bytes_read += approx_bytes as u64;
            
            // Update activity tracking
            self.lines_since_last_read += lines_read;
            self.last_activity = Instant::now();
            self.is_active = true;
            
            Ok(new_lines)
        } else {
            // No new data, but check if file was truncated
            // Only do this occasionally to reduce network operations
            if self.last_activity.elapsed() > std::time::Duration::from_secs(10) {
                if let Ok(metadata) = std::fs::metadata(&self.path) {
                    let current_size = metadata.len();
                    if current_size < self.last_size {
                        info!("File rotation detected for {}: size decreased from {} to {}", 
                            self.display_name, self.last_size, current_size);
                        self.last_position = 0;
                        self.last_size = current_size;
                        self.cached_size_kb_string = format!("{:.1} KB", current_size as f64 / 1024.0);
                        return Ok(vec!["[FILE TRUNCATED/ROTATED]".to_string()]);
                    }
                    self.last_size = current_size;
                    self.cached_size_kb_string = format!("{:.1} KB", current_size as f64 / 1024.0);
                }
            }
            Ok(vec![])
        }
    }
}

struct LogLine {
    timestamp: Instant,
    source_file: Arc<str>,
    line_number: usize,
    content: Arc<str>, // Changed from String to Arc<str> for memory efficiency
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum PreviewMode {
    Following, // Auto-scroll to bottom, show last N lines
    Paused,    // Manual navigation
}

struct TailState {
    // Files being monitored
    files: Vec<TailedFile>,
    selected_file_index: Option<usize>,

    // Output buffer (circular)
    output_buffer: VecDeque<LogLine>,
    max_buffer_lines: usize,

    // Global controls
    paused_all: bool,
    auto_scroll: bool,

    // Filtering
    filter_pattern: String,
    preview_filter: filter::PreviewFilter,
    tree_filter: filter::TreeFilter,
    log_level_filter: filter::LogLevelFilter,

    // Polling
    last_poll_time: Instant,
    poll_interval_ms: u64,

    // Statistics
    total_lines_received: usize,
    lines_dropped: usize,

    // Performance tuning
    max_lines_per_poll: usize,

    // Preview pane
    preview_selected_file: Option<usize>,
    preview_mode: PreviewMode,
    preview_scroll_offset: f32,
    preview_follow_lines: usize,
    preview_buffer: BufferWindow,  // Encapsulates preview_content and buffer tracking
    preview_needs_reload: bool,

    // Text viewer state (encapsulates preview display, navigation, filtering, goto)
    text_viewer_state: widgets::TextViewerState,

    // Font settings
    font_size: f32,

    // Tree layout
    layout: Option<TailLayout>,

    // UI state
    control_panel_height: f32,
    max_filename_width: f32,  // Cached maximum filename width for alignment

    // Bookmark management for tail mode
    bookmark_manager: BookmarkManager,

    // Centralized file watching
    file_watch_manager: FileWatchManager,

    // Error tracking
    file_errors: Vec<String>,
    
    // Rendering statistics for memory monitoring
    last_output_rendered_lines: Option<usize>,
    last_output_visible_lines: Option<usize>,
}

impl TailState {
    fn new(config: &Config) -> Self {
        Self {
            files: Vec::new(),
            selected_file_index: None,
            output_buffer: VecDeque::new(),
            max_buffer_lines: config.ui.max_buffer_lines,
            paused_all: false,
            auto_scroll: true,
            filter_pattern: String::new(),
            preview_filter: filter::PreviewFilter::new(),
            tree_filter: filter::TreeFilter::new(),
            log_level_filter: filter::LogLevelFilter::new(),
            last_poll_time: Instant::now(),
            poll_interval_ms: config.ui.poll_interval_ms,
            total_lines_received: 0,
            lines_dropped: 0,
            max_lines_per_poll: 100,
            preview_selected_file: None,
            preview_mode: PreviewMode::Following,  // Default to Following (last N lines)
            preview_scroll_offset: 0.0,
            preview_follow_lines: 1000,
            preview_buffer: BufferWindow::empty(),
            preview_needs_reload: false,
            text_viewer_state: widgets::TextViewerState::new(config.ui.font_size),
            font_size: config.ui.font_size,
            layout: None,
            control_panel_height: 250.0,
            max_filename_width: 200.0,  // Initial default, will be recalculated
            bookmark_manager: BookmarkManager::new(),
            file_watch_manager: FileWatchManager::new(),
            file_errors: Vec::new(),
            last_output_rendered_lines: None,
            last_output_visible_lines: None,
        }
    }

    fn add_file(&mut self, path: PathBuf) -> Result<(), String> {
        self.add_file_with_group(path, None)
    }
    
    fn add_file_with_group(&mut self, path: PathBuf, group_id: Option<String>) -> Result<(), String> {
        match TailedFile::new(path.clone()) {
            Ok(mut file) => {
                info!("Started tailing: {}", file.display_name);
                file.group_id = group_id;

                // Subscribe to file updates via FileWatchManager
                // IMPORTANT: Use the absolute path from TailedFile, not the original path
                let file_idx = self.files.len();
                match self.file_watch_manager.subscribe(
                    file.path.clone(),  // Use absolute path from TailedFile
                    FileSubscriber::Tail { mode_id: file_idx }
                ) {
                    Ok(_) => {
                        info!("📂 Subscribed to file updates for: {}", file.display_name);
                    }
                    Err(e) => {
                        warn!("📂 Failed to subscribe to file watch for {}: {}", file.display_name, e);
                    }
                }

                self.files.push(file);
                Ok(())
            }
            Err(e) => {
                let msg = format!("Failed to tail file '{}': {}", path.display(), e);
                info!("{}", msg);
                self.file_errors.push(msg.clone());
                Err(msg)
            }
        }
    }
    
    fn load_layout(&mut self, layout_path: &PathBuf) -> Result<(), String> {
        // Load the layout file
        let mut layout = TailLayout::from_yaml_file(layout_path)?;
        
        // Apply layout settings
        if let Some(poll_ms) = layout.settings.poll_interval_ms {
            self.poll_interval_ms = poll_ms;
        }
        if let Some(buffer_lines) = layout.settings.max_buffer_lines {
            self.max_buffer_lines = buffer_lines;
        }
        
        // Add all files from the layout
        let file_paths = layout.get_all_file_paths();
        for (path, custom_name, group_id, paused) in file_paths {
            match TailedFile::new(path.clone()) {
                Ok(mut file) => {
                    if let Some(name) = custom_name {
                        file.display_name = name;
                    }
                    file.group_id = Some(group_id.clone());
                    file.paused = paused;  // Apply paused setting from YAML

                    // Store the index before pushing
                    let file_idx = self.files.len();

                    // Subscribe to FileWatchManager using absolute path
                    match self.file_watch_manager.subscribe(
                        file.path.clone(),  // Use absolute path from TailedFile
                        FileSubscriber::Tail { mode_id: file_idx }
                    ) {
                        Ok(_) => {
                            info!("📂 Subscribed to file updates for: {}", file.display_name);
                        }
                        Err(e) => {
                            warn!("📂 Failed to subscribe to file watch for {}: {}", file.display_name, e);
                        }
                    }

                    self.files.push(file);

                    // Update the layout to link to this file
                    layout.link_file_to_index(&path, &group_id, file_idx);
                }
                Err(e) => {
                    let msg = format!("Failed to tail file '{}': {}", path.display(), e);
                    info!("{}", msg);
                    self.file_errors.push(msg);
                }
            }
        }
        
        self.layout = Some(layout);
        Ok(())
    }
}

// ============================================================================
// Main Application State
// ============================================================================

struct VisGrepApp {
    // Current mode
    mode: AppMode,

    // Mode-specific state
    grep_state: GrepState,
    tail_state: TailState,

    // Shared state (used across modes)
    preview: FilePreview,
    preview_scroll_offset: f32,
    should_scroll_to_match: bool,
    scroll_to_selected_result: bool,

    input_handler: InputHandler,
    marks: HashMap<char, usize>,

    config: Config,
    theme: Theme,

    // Log level detection
    log_detector: log_parser::LogLevelDetector,
    
    // Memory monitoring
    memory_monitor: MemoryMonitor,
    
    // Debug mode for showing memory stats
    debug_mode: bool,
    
    // Idle monitor for auto-shutdown
    idle_monitor: IdleMonitor,
}

impl Default for VisGrepApp {
    fn default() -> Self {
        Self::new(StartupConfig::default())
    }
}

impl VisGrepApp {
    fn new(startup_config: StartupConfig) -> Self {
        // Load config first so we can use it for initialization
        let config = Config::load();
        let theme = config.theme;
        
        // Initialize idle monitor from config
        let idle_monitor = IdleMonitor::new(
            config.ui.auto_shutdown_minutes.unwrap_or(0),
            config.ui.auto_shutdown_minutes.is_some()
        );

        let mut tail_state = TailState::new(&config);

        // Load layout file if provided
        if let Some(layout_path) = &startup_config.tail_layout {
            if let Err(e) = tail_state.load_layout(layout_path) {
                eprintln!("Failed to load layout file: {}", e);
            }
        }

        // Add individual files from startup config
        for file_path in startup_config.tail_files {
            if let Err(e) = tail_state.add_file(file_path) {
                eprintln!("{}", e);
            }
        }

        Self {
            mode: startup_config.mode,

            grep_state: GrepState::new(&config),
            tail_state,

            preview: FilePreview::new(),
            preview_scroll_offset: 0.0,
            should_scroll_to_match: false,
            scroll_to_selected_result: false,

            input_handler: InputHandler::new(),
            marks: HashMap::new(),

            config,
            theme,

            log_detector: log_parser::LogLevelDetector::new(),
            memory_monitor: MemoryMonitor::new(),
            debug_mode: false,
            idle_monitor,
        }
    }

    /// Expand ~ to home directory
    fn expand_tilde(path: &str) -> String {
        if let Some(stripped) = path.strip_prefix("~/") {
            if let Some(home) = std::env::var_os("HOME") {
                return format!("{}/{}", home.to_string_lossy(), stripped);
            }
        }
        path.to_string()
    }

    fn perform_search(&mut self) {
        // Expand tilde in search path
        let expanded_path = Self::expand_tilde(&self.grep_state.search_path);

        info!(
            "Starting search: path='{}', pattern='{}', query='{}', file_age={:?}hrs",
            &expanded_path,
            &self.grep_state.file_pattern,
            &self.grep_state.search_query,
            &self.grep_state.file_age_hours
        );
        self.grep_state.searching = true;
        self.grep_state.pending_search = false;
        let start = Instant::now();
        self.grep_state.results = self.grep_state.search_engine.search(
            &expanded_path,
            &self.grep_state.file_pattern,
            &self.grep_state.search_query,
            self.grep_state.case_sensitive,
            self.grep_state.use_regex,
            self.grep_state.recursive,
            self.grep_state.file_age_hours,
        );
        let duration = start.elapsed();
        info!(
            "Search completed in {:.2}s: found {} matches in {} files",
            duration.as_secs_f64(),
            self.grep_state
                .results
                .iter()
                .map(|r| r.matches.len())
                .sum::<usize>(),
            self.grep_state.results.len()
        );
        self.grep_state.searching = false;
        self.grep_state.selected_result = None;
        self.grep_state.last_search_time = Instant::now();

        // Initialize all headers as expanded for new search
        self.grep_state.collapsing_state.clear();
        for i in 0..self.grep_state.results.len() {
            self.grep_state.collapsing_state.insert(i, true);
        }
    }

    fn poll_tail_files(&mut self) {
        if self.tail_state.paused_all {
            debug!("📂 poll_tail_files: Skipping - paused_all is true");
            return;
        }

        let now = Instant::now();
        let elapsed = now.duration_since(self.tail_state.last_poll_time);

        // Poll at configured interval
        if elapsed < std::time::Duration::from_millis(self.tail_state.poll_interval_ms) {
            debug!("📂 poll_tail_files: Skipping - not enough time elapsed ({:?} < {}ms)",
                   elapsed, self.tail_state.poll_interval_ms);
            return;
        }

        debug!("📂 poll_tail_files: Starting poll cycle");

        self.tail_state.last_poll_time = now;

        // Check for file updates via FileWatchManager
        let updates = self.tail_state.file_watch_manager.check_for_updates();

        if !updates.is_empty() {
            info!("📂 poll_tail_files: Received {} file updates from FileWatchManager", updates.len());
        }

        // Collect activity changes to apply after the loop
        let mut activity_changes: Vec<(String, bool)> = Vec::new();

        // Process updates from FileWatchManager
        for (file_path, (update, subscribers)) in updates {
            info!("📂 poll_tail_files: Processing update for {:?} with {} subscribers",
                  file_path.file_name().unwrap_or_default(), subscribers.len());
            // Process each subscriber
            for subscriber in subscribers {
                if let FileSubscriber::Tail { mode_id: file_idx } = subscriber {
                    // Get the file from our files list
                    if let Some(file) = self.tail_state.files.get_mut(file_idx) {
                        // Skip if file is paused
                        if file.paused {
                            continue;
                        }

                        match update {
                            FileUpdate::NewLines { ref lines, start_line: _ } => {
                                let was_active = file.is_active;
                                if !lines.is_empty() {
                                    info!("📂 poll_tail_files: File {} received {} new lines",
                                          file.display_name, lines.len());
                                    file.is_active = true;
                                    file.last_activity = now;
                                    file.lines_since_last_read = lines.len();

                                    // Update total lines read
                                    file.total_lines_read += lines.len();

                                    // Clear and recalculate level counts for recent activity
                                    file.level_counts_since_last_read.clear();

                                    // Store activity change to propagate later
                                    if !was_active {
                                        if let Some(group_id) = &file.group_id {
                                            activity_changes.push((group_id.clone(), true));
                                        }
                                    }

                                    // Add lines to output buffer and track log levels
                                    for line in lines {
                                        // Detect and count log level for this line
                                        let level = self.log_detector.detect(line);
                                        *file.level_counts_since_last_read.entry(level).or_insert(0) += 1;

                                        let log_line = LogLine {
                                            timestamp: now,
                                            source_file: Arc::clone(&file.display_name_arc),
                                            line_number: file.total_lines_read,
                                            content: string_cache::intern_string(line),
                                        };

                                        self.tail_state.output_buffer.push_back(log_line);
                                        self.tail_state.total_lines_received += 1;

                                        // Trim buffer if over capacity
                                        if self.tail_state.output_buffer.len()
                                            > self.tail_state.max_buffer_lines
                                        {
                                            self.tail_state.output_buffer.pop_front();
                                            self.tail_state.lines_dropped += 1;
                                        }
                                    }

                                    // If preview is in Following mode and showing this file, reload it
                                    if self.tail_state.preview_mode == PreviewMode::Following {
                                        if let Some(preview_idx) = self.tail_state.preview_selected_file {
                                            if file_idx == preview_idx {
                                                self.tail_state.preview_needs_reload = true;
                                            }
                                        }
                                    }
                                }
                            }
                            FileUpdate::Truncated => {
                                info!("📂 File truncated: {:?}", file_path);
                                // Reset file state
                                file.last_position = 0;
                                file.last_size = 0;
                                file.cached_size_kb_string = format!("0.0 KB");
                                file.total_lines_read = 0;
                            }
                            FileUpdate::Deleted => {
                                info!("📂 File deleted: {:?}", file_path);
                                file.is_active = false;
                            }
                            FileUpdate::Error(ref err) => {
                                info!("📂 Error reading {}: {}", file.display_name, err);
                            }
                        }
                    }
                }
            }
        }

        // Check for idle files (mark inactive after 2 seconds)
        for file in self.tail_state.files.iter_mut() {
            if file.is_active && now.duration_since(file.last_activity) > std::time::Duration::from_secs(2) {
                file.is_active = false;
                file.lines_since_last_read = 0;
                file.level_counts_since_last_read.clear();

                // Store activity change to propagate later
                if let Some(group_id) = &file.group_id {
                    activity_changes.push((group_id.clone(), false));
                }
            }
        }

        // Apply activity changes after the loop
        for (group_id, active) in activity_changes {
            self.propagate_activity_to_group(&group_id, active);
        }

        // Reload preview if needed (auto update from file polling)
        if self.tail_state.preview_needs_reload {
            self.reload_tail_preview_impl(true);  // auto_update = true
        }
        
        // Aggressively shrink buffer capacity to free memory
        // Check every 100 lines instead of 1000 for more responsive memory management
        if self.tail_state.total_lines_received % 100 == 0 {
            let current_capacity = self.tail_state.output_buffer.capacity();
            let current_len = self.tail_state.output_buffer.len();
            
            // Be more aggressive: shrink if capacity is more than 1.5x actual usage
            // This helps prevent memory bloat during bursts of activity
            let target_capacity = (current_len as f32 * 1.2) as usize + 100;
            
            if current_capacity > target_capacity && current_capacity > current_len + 100 {
                debug!("Shrinking buffer capacity from {} to {} (current len: {})", 
                    current_capacity, target_capacity, current_len);
                self.tail_state.output_buffer.shrink_to(target_capacity);
                
                // Also trigger string cache cleanup periodically
                if self.tail_state.total_lines_received % 10000 == 0 {
                    let (size, hits, misses) = string_cache::cache_stats();
                    let hit_rate = if hits + misses > 0 {
                        (hits as f32 / (hits + misses) as f32) * 100.0
                    } else {
                        0.0
                    };
                    info!("String cache stats: {} entries, {:.1}% hit rate", size, hit_rate);
                }
            }
        }
    }
    
    fn propagate_activity_to_group(&mut self, group_id: &str, active: bool) {
        if let Some(layout) = &mut self.tail_state.layout {
            layout.update_group_activity(group_id, active);
        }
    }

    fn reload_tail_preview(&mut self) {
        self.reload_tail_preview_impl(false)
    }

    fn reload_tail_preview_impl(&mut self, auto_update: bool) {
        if let Some(file_idx) = self.tail_state.preview_selected_file {
            if file_idx < self.tail_state.files.len() {
                let file = &self.tail_state.files[file_idx];
                let file_path = file.path.clone();

                match self.read_file_for_preview(&file_path) {
                    Ok(buffer) => {
                        self.tail_state.preview_buffer = buffer;
                        self.tail_state.preview_needs_reload = false;

                        // Update filter matches if filter is active
                        if self.tail_state.preview_filter.active {
                            filter::preview::update_filter_matches(
                                &mut self.tail_state.preview_filter,
                                &self.tail_state.preview_buffer.lines
                            );
                        }

                        // Only prune bookmarks during automatic updates in Following mode
                        // Don't prune when manually switching files or navigating to bookmarks
                        if auto_update && self.tail_state.preview_mode == PreviewMode::Following {
                            // Get all marks for this file
                            let file_marks = self.tail_state.bookmark_manager.get_file_marks(&file_path);

                            // Check which ones are outside the current buffer
                            let mut marks_to_remove = Vec::new();
                            for (mark_char, bookmark) in file_marks {
                                if !self.tail_state.preview_buffer.contains_absolute_line(bookmark.absolute_line_number) {
                                    marks_to_remove.push(mark_char);
                                    info!("⚠️  Mark '{}' at {}:{} is outside current buffer ({}..{}), will prune",
                                        mark_char,
                                        bookmark.file_path.display(),
                                        bookmark.absolute_line_number,
                                        self.tail_state.preview_buffer.start_line,
                                        self.tail_state.preview_buffer.end_line()
                                    );
                                }
                            }

                            // Prune marks that slid out of the buffer
                            for mark_char in marks_to_remove {
                                if let Some(removed_bookmark) = self.tail_state.bookmark_manager.remove_mark(mark_char) {
                                    info!("🗑️  Pruned stale mark '{}' from {}:{} (slid outside Following buffer range {}..{})",
                                        mark_char,
                                        removed_bookmark.file_path.display(),
                                        removed_bookmark.absolute_line_number,
                                        self.tail_state.preview_buffer.start_line,
                                        self.tail_state.preview_buffer.end_line()
                                    );
                                }
                            }
                        } else if !auto_update {
                            info!("ℹ️  Manual reload - not pruning bookmarks");
                        }
                    }
                    Err(e) => {
                        info!("Error loading preview for {}: {}", file.display_name, e);
                        self.tail_state.preview_buffer = BufferWindow::new(
                            vec![format!("Error: {}", e)],
                            0,
                            1
                        );
                    }
                }
            }
        }
    }

    fn read_file_for_preview(&self, path: &PathBuf) -> std::io::Result<BufferWindow> {
        use crate::file_tail_reader::FileTailReader;

        // Use efficient tail reading that seeks from end of file
        let (lines, start_line, total_lines) = 
            FileTailReader::read_last_lines(path, self.tail_state.preview_follow_lines)?;

        Ok(BufferWindow::new(
            lines,
            start_line,
            total_lines
        ))
    }
}

impl eframe::App for VisGrepApp {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        // Apply theme
        self.theme.apply(ctx);
        
        // Check memory usage periodically
        self.memory_monitor.check();
        
        // Track user activity for idle monitoring
        let has_activity = ctx.input(|i| {
            // Check for meaningful interaction events
            for event in &i.events {
                match event {
                    // Mouse/pointer events
                    egui::Event::PointerButton { .. } |
                    egui::Event::PointerMoved(_) |
                    // Keyboard events  
                    egui::Event::Key { .. } |
                    egui::Event::Text(_) |
                    // Touch events
                    egui::Event::Touch { .. } => {
                        return true;
                    }
                    // Ignore these events for activity tracking
                    egui::Event::Copy |
                    egui::Event::Cut |
                    egui::Event::Paste(_) => {}
                    
                    // Handle any other events we might not know about
                    _ => {}
                }
            }
            
            false
        });
        
        if has_activity {
            self.idle_monitor.record_activity();
        }
        
        // Check if we should auto-shutdown
        if self.idle_monitor.should_shutdown() {
            info!("Auto-shutdown triggered due to inactivity");
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            return;
        }

        // For tail mode, handle TextViewer navigation FIRST
        // This updates cursor_line before we process bookmark commands
        if self.mode == AppMode::Tail {
            self.handle_tail_mode_navigation(ctx);
        }

        // Process keyboard input and handle navigation commands
        if let Some(command) = self.input_handler.process_input(ctx) {
            self.handle_navigation_command(command);
        }

        // Top header panel (non-resizable)
        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            self.render_header(ui);
            ui.separator();
            self.render_mode_tabs(ui);
            ui.separator();
        });

        // Bottom status bar
        egui::TopBottomPanel::bottom("bottom_panel").show(ctx, |ui| {
            self.render_status_bar(ui);
        });

        // Mode-specific top panels
        match self.mode {
            AppMode::Grep => {
                egui::TopBottomPanel::top("grep_controls")
                    .resizable(true)
                    .default_height(200.0)
                    .height_range(150.0..=400.0)
                    .show(ctx, |ui| {
                        self.render_grep_mode_ui(ui);
                    });
            },
            _ => {},
        }

        // 2. Second: SidePanels
        // Get available width to calculate better ranges
        let available_width = ctx.available_rect().width();
        let min_panel_width = 200.0; // Minimum for any panel
        let max_left_panel_width = available_width - min_panel_width; // Leave room for right panel
        
        match self.mode {
            AppMode::Grep => {
                egui::SidePanel::left("grep_left_panel")
                    .resizable(true)
                    .default_width((available_width * 0.4).clamp(300.0, 800.0))
                    .width_range(min_panel_width..=max_left_panel_width)
                    .show(ctx, |ui| {
                        egui::ScrollArea::horizontal()
                            .id_salt("grep_left_scroll_h")
                            .show(ui, |ui| {
                                self.render_grep_left_panel(ui);
                            });
                    });
            },
            AppMode::Tail => {
                // No side panels - we'll use custom splitters in CentralPanel
            },
            AppMode::Test => {
                // No side panels in test mode
            },
        }

        // 3. Last: CentralPanel
        egui::CentralPanel::default().show(ctx, |ui| {
            match self.mode {
                AppMode::Grep => {
                    let available_rect = ui.available_rect_before_wrap();
                    if available_rect.width() < 50.0 || available_rect.height() < 50.0 {
                        ui.colored_label(
                            egui::Color32::RED,
                            format!("Warning: Panel too small: {:.0}x{:.0}", 
                                    available_rect.width(), 
                                    available_rect.height())
                        );
                    } else {
                        self.render_grep_right_panel(ui);
                    }
                },
                AppMode::Tail => {
                    // Use custom vertical splitter (horizontal divider line)
                    Splitter::new("tail_vertical_split", SplitterAxis::Vertical)
                        .min_size(150.0)
                        .default_pos(0.3) // 30% top for controls, 70% bottom for content
                        .show(ui, |ui_top, ui_bottom| {
                            // Top: Controls and file list
                            self.render_tail_mode_controls(ui_top);
                            
                            // Bottom: Horizontal splitter for output (left) and preview (right)
                            Splitter::new("tail_horizontal_split", SplitterAxis::Horizontal)
                                .min_size(200.0)
                                .default_pos(0.5) // 50/50 split
                                .show(ui_bottom, |ui_left, ui_right| {
                                    // Left: Combined output
                                    self.render_tail_output(ui_left);
                                    
                                    // Right: File preview
                                    self.render_tail_preview(ui_right);
                                });
                        });
                },
                AppMode::Test => {
                    Splitter::new("test_split", SplitterAxis::Vertical)
                        .min_size(100.0)
                        .default_pos(0.3)
                        .show(ui, |ui_top, ui_bottom| {
                            ui_top.heading("Top Panel (Commands & Files)");
                            ui_top.label("This is the top 30%");
                            ui_top.label("Drag the horizontal line below to resize");
                            
                            ui_bottom.heading("Bottom Panel (Output)");
                            ui_bottom.label("This is the bottom 70%");
                            ui_bottom.label("The custom splitter works!");
                        });
                },
            }
        });
        
        // Debug mode toggle (Ctrl+Shift+D)
        ctx.input(|i| {
            if i.key_pressed(egui::Key::D) && i.modifiers.ctrl && i.modifiers.shift {
                self.debug_mode = !self.debug_mode;
                info!("Debug mode: {}", if self.debug_mode { "ON" } else { "OFF" });
            }
        });
        
        // Render debug overlay if enabled
        if self.debug_mode {
            egui::Area::new("debug_overlay_area".into())
                .interactable(false)
                .order(egui::Order::Foreground)
                .show(ctx, |ui| {
                    self.memory_monitor.render_debug_overlay(ui);
                });
        }

        // Mode-specific background tasks
        match self.mode {
            AppMode::Grep => self.handle_grep_mode_background_tasks(),
            AppMode::Tail => {
                // Poll files for updates
                self.poll_tail_files();
                // Note: tail mode navigation handled earlier in update()
            },
            AppMode::Test => {
                // No background tasks for test mode
            },
        }

        // Request repaints for different scenarios
        if self.mode == AppMode::Tail && !self.tail_state.paused_all {
            // Normal tail mode updates
            ctx.request_repaint_after(std::time::Duration::from_millis(self.tail_state.poll_interval_ms));
        } else if self.idle_monitor.time_until_shutdown().is_some() {
            // Update idle monitor countdown every second when idle monitoring is active
            ctx.request_repaint_after(std::time::Duration::from_secs(1));
        }
    }

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        // egui will automatically save persisted memory when this is called
        log::info!("Application exiting, persisted data will be saved");
    }
}

impl VisGrepApp {
    fn select_match(&mut self, result_id: usize, file_path: &std::path::Path, line_number: usize) {
        self.grep_state.selected_result = Some(result_id);
        self.preview.load_file(file_path, line_number);

        // Calculate scroll offset to center the target line in viewport
        if let Some(target_line_idx) = self.preview.target_line_in_preview {
            let line_height = 14.0; // egui code editor default line height
            let lines_above_target = 10;
            let scroll_to_line = target_line_idx.saturating_sub(lines_above_target);
            self.preview_scroll_offset = scroll_to_line as f32 * line_height;
            self.should_scroll_to_match = true; // Flag that we want to scroll
            info!("Match selected: file line {}, preview line index {}, scroll to line {} (show {} lines above), offset {}px",
                  line_number, target_line_idx, scroll_to_line, lines_above_target, self.preview_scroll_offset);
        }
    }

    fn select_match_with_keyboard(
        &mut self,
        result_id: usize,
        file_path: &std::path::Path,
        line_number: usize,
    ) {
        self.select_match(result_id, file_path, line_number);
        self.scroll_to_selected_result = true; // Flag to scroll results panel
    }

    fn select_next_match(&mut self) {
        if self.grep_state.results.is_empty() {
            return;
        }

        let current_id = self.grep_state.selected_result.unwrap_or(0);
        let current_file_idx = current_id / 10000;
        let current_match_idx = current_id % 10000;

        // Try next match in current file
        if current_file_idx < self.grep_state.results.len()
            && current_match_idx + 1 < self.grep_state.results[current_file_idx].matches.len()
        {
            let next_id = current_file_idx * 10000 + current_match_idx + 1;
            let file_path = self.grep_state.results[current_file_idx].file_path.clone();
            let line_number = self.grep_state.results[current_file_idx].matches
                [current_match_idx + 1]
                .line_number;
            self.select_match_with_keyboard(next_id, &file_path, line_number);
            return;
        }

        // Move to first match in next file
        for file_idx in (current_file_idx + 1)..self.grep_state.results.len() {
            if !self.grep_state.results[file_idx].matches.is_empty() {
                let next_id = file_idx * 10000;
                let file_path = self.grep_state.results[file_idx].file_path.clone();
                let line_number = self.grep_state.results[file_idx].matches[0].line_number;
                self.select_match_with_keyboard(next_id, &file_path, line_number);
                return;
            }
        }

        // Wrap to first match
        if !self.grep_state.results.is_empty() && !self.grep_state.results[0].matches.is_empty() {
            let file_path = self.grep_state.results[0].file_path.clone();
            let line_number = self.grep_state.results[0].matches[0].line_number;
            self.select_match_with_keyboard(0, &file_path, line_number);
        }
    }

    fn handle_navigation_command(&mut self, command: NavigationCommand) {
        match command {
            NavigationCommand::NextMatch => self.select_next_match(),
            NavigationCommand::PreviousMatch => self.select_previous_match(),
            NavigationCommand::FirstMatch => self.select_first_match(),
            NavigationCommand::LastMatch => self.select_last_match(),
            NavigationCommand::NextMatchWithCount(count) => {
                for _ in 0..count {
                    self.select_next_match();
                }
            }
            NavigationCommand::PreviousMatchWithCount(count) => {
                for _ in 0..count {
                    self.select_previous_match();
                }
            }
            NavigationCommand::FirstMatchInCurrentFile => self.select_first_match_in_current_file(),
            NavigationCommand::LastMatchInCurrentFile => self.select_last_match_in_current_file(),
            NavigationCommand::NextFile => self.select_next_file(),
            NavigationCommand::PreviousFile => self.select_previous_file(),
            NavigationCommand::NextFileWithCount(count) => {
                for _ in 0..count {
                    self.select_next_file();
                }
            }
            NavigationCommand::PreviousFileWithCount(count) => {
                for _ in 0..count {
                    self.select_previous_file();
                }
            }
            NavigationCommand::YankMatchedLine => self.yank_matched_line(),
            NavigationCommand::OpenInExplorer => self.open_in_explorer(),
            NavigationCommand::SetMark(ch) => self.set_mark(ch),
            NavigationCommand::GotoMark(ch) => self.goto_mark(ch),
        }
    }

    fn set_mark(&mut self, ch: char) {
        match self.mode {
            AppMode::Grep => self.set_grep_mark(ch),
            AppMode::Tail => self.set_tail_mark(ch),
            _ => {}
        }
    }

    fn goto_mark(&mut self, ch: char) {
        match self.mode {
            AppMode::Grep => self.goto_grep_mark(ch),
            AppMode::Tail => self.goto_tail_mark(ch),
            _ => {}
        }
    }

    fn set_grep_mark(&mut self, ch: char) {
        if let Some(result_id) = self.grep_state.selected_result {
            self.marks.insert(ch, result_id);
            info!("Set mark '{}' at result {}", ch, result_id);
        } else {
            info!("No result selected to mark");
        }
    }

    fn goto_grep_mark(&mut self, ch: char) {
        if let Some(&result_id) = self.marks.get(&ch) {
            let file_idx = result_id / 10000;
            let match_idx = result_id % 10000;

            if file_idx < self.grep_state.results.len()
                && match_idx < self.grep_state.results[file_idx].matches.len()
            {
                let file_path = self.grep_state.results[file_idx].file_path.clone();
                let line_number = self.grep_state.results[file_idx].matches[match_idx].line_number;
                self.select_match_with_keyboard(result_id, &file_path, line_number);
                info!("Jumped to mark '{}'", ch);
            } else {
                info!("Mark '{}' points to invalid result", ch);
            }
        } else {
            info!("Mark '{}' not set", ch);
        }
    }

    fn set_tail_mark(&mut self, ch: char) {
        // Only set marks when viewing a file in tail mode
        if let Some(file_idx) = self.tail_state.preview_selected_file {
            if file_idx < self.tail_state.files.len() {
                let file_path = self.tail_state.files[file_idx].path.clone();

                // Get current line - prefer last_navigated_line (from :goto or previous marks)
                // Otherwise use cursor_line (updated by TextViewer)
                let relative_line = self.tail_state.text_viewer_state.last_navigated_line
                    .or(Some(self.tail_state.text_viewer_state.cursor_line))
                    .unwrap_or_else(|| {
                        // Fallback: estimate from scroll offset
                        let line_height = self.tail_state.text_viewer_state.font_size + 4.0;
                        (self.tail_state.text_viewer_state.scroll_offset / line_height) as usize
                    });

                // Convert to absolute line number using BufferWindow
                let absolute_line = self.tail_state.preview_buffer
                    .relative_to_absolute(relative_line)
                    .unwrap_or(relative_line);

                // Get the line content
                let line_content = self.tail_state.preview_buffer
                    .get_line(relative_line)
                    .cloned()
                    .unwrap_or_default();

                self.tail_state.bookmark_manager.set_mark(
                    ch,
                    file_path.clone(),
                    absolute_line,
                    line_content.clone()
                );

                info!("📌 SET TAIL BOOKMARK '{}' → File: {}, Relative Line: {}, Absolute Line: {}, Buffer Start: {}, Content: '{}'",
                    ch,
                    file_path.display(),
                    relative_line,
                    absolute_line,
                    self.tail_state.preview_buffer.start_line,
                    line_content.chars().take(50).collect::<String>()
                );

                // Show all current bookmarks
                let all_marks = self.tail_state.bookmark_manager.get_all_marks();
                info!("📚 ACTIVE BOOKMARKS ({}): {:?}",
                    all_marks.len(),
                    all_marks.iter().map(|(c, b)| format!("'{}' → {}:{}", c, b.file_path.display(), b.absolute_line_number)).collect::<Vec<_>>()
                );
            }
        } else {
            info!("No file selected in tail mode to mark");
        }
    }

    fn goto_tail_mark(&mut self, ch: char) {
        info!("🔍 GOTO TAIL BOOKMARK '{}' - Checking bookmark manager...", ch);

        // Show all current bookmarks for debugging
        let all_marks = self.tail_state.bookmark_manager.get_all_marks();
        info!("📚 AVAILABLE BOOKMARKS: {:?}",
            all_marks.iter().map(|(c, b)| format!("'{}' → {}:{}", c, b.file_path.display(), b.absolute_line_number)).collect::<Vec<_>>()
        );

        // Clone bookmark data to avoid borrow conflicts
        if let Some(bookmark) = self.tail_state.bookmark_manager.get_mark(ch).cloned() {
            info!("✅ Found bookmark '{}' at {}:{}", ch, bookmark.file_path.display(), bookmark.absolute_line_number);

            // Find the file index for this bookmark
            let file_idx = self.tail_state.files.iter()
                .position(|f| f.path == bookmark.file_path);

            if let Some(idx) = file_idx {
                info!("📂 Switching to file index {} ({})", idx, bookmark.file_path.display());

                // Switch to the bookmarked file
                let previous_file = self.tail_state.preview_selected_file;
                self.tail_state.preview_selected_file = Some(idx);
                self.tail_state.preview_needs_reload = true;
                self.tail_state.preview_mode = PreviewMode::Paused;

                // Force reload NOW if we're switching files
                if previous_file != Some(idx) {
                    info!("🔄 File changed (from {:?} to {}), forcing reload", previous_file, idx);
                    self.reload_tail_preview();
                }

                // Try to convert absolute line to relative position using BufferWindow
                if let Some(relative_line) = self.tail_state.preview_buffer
                    .absolute_to_relative(bookmark.absolute_line_number) {
                    info!("✅ Bookmark is in buffer range! Absolute: {}, Buffer Start: {}, Relative: {}",
                        bookmark.absolute_line_number,
                        self.tail_state.preview_buffer.start_line,
                        relative_line
                    );

                    // Navigate to the bookmarked line
                    self.tail_state.text_viewer_state.cursor_line = relative_line;
                    self.tail_state.text_viewer_state.goto_line_target = Some(relative_line);
                    self.tail_state.text_viewer_state.last_navigated_line = Some(relative_line);
                    self.tail_state.text_viewer_state.view_mode = widgets::ViewMode::Paused;

                    // CRITICAL: Calculate scroll position directly based on line number
                    // This avoids issues with scroll_to_rect and stale layout state
                    let line_height = self.tail_state.text_viewer_state.font_size + 4.0;
                    let target_scroll = (relative_line as f32 * line_height).max(0.0);
                    self.tail_state.text_viewer_state.scroll_offset = target_scroll;
                    info!("📍 Set scroll_offset to {} (line {} * line_height {})", target_scroll, relative_line, line_height);

                    info!("🎯 Jumped to tail mark '{}' at {}:{} (relative line {} in buffer)",
                        ch,
                        bookmark.file_path.display(),
                        bookmark.absolute_line_number,
                        relative_line
                    );
                } else {
                    info!("⚠️  Mark '{}' at absolute line {} is outside current buffer (buffer: {}..{})",
                        ch,
                        bookmark.absolute_line_number,
                        self.tail_state.preview_buffer.start_line,
                        self.tail_state.preview_buffer.end_line()
                    );
                }
            } else {
                info!("❌ Mark '{}' file not found in current tail session: {}",
                    ch,
                    bookmark.file_path.display()
                );
            }
        } else {
            info!("❌ Tail mark '{}' not set", ch);
        }
    }

    fn open_in_editor(&self) {
        if self.grep_state.results.is_empty() {
            info!("No results to open");
            return;
        }
        
        let current_file_idx = self.grep_state.selected_result.unwrap_or(0) / 10000;
        if current_file_idx >= self.grep_state.results.len() {
            info!("Invalid file index");
            return;
        }
        let file_path = &self.grep_state.results[current_file_idx].file_path;
        self.open_file_in_editor(file_path);
    }
    
    fn open_in_explorer(&self) {
        if self.grep_state.results.is_empty() {
            info!("No results to open");
            return;
        }

        let current_id = self.grep_state.selected_result.unwrap_or(0);
        let current_file_idx = current_id / 10000;

        if current_file_idx >= self.grep_state.results.len() {
            info!("Invalid file index");
            return;
        }

        let file_path = &self.grep_state.results[current_file_idx].file_path;
        Self::open_path_in_explorer(file_path);
    }
    
    fn open_in_terminal(&self) {
        if self.grep_state.results.is_empty() {
            info!("No results to open");
            return;
        }

        let current_id = self.grep_state.selected_result.unwrap_or(0);
        let current_file_idx = current_id / 10000;

        if current_file_idx >= self.grep_state.results.len() {
            info!("Invalid file index");
            return;
        }

        let file_path = &self.grep_state.results[current_file_idx].file_path;
        self.open_file_in_terminal(file_path);
    }
    
    /// Open a file in the configured editor
    fn open_file_in_editor(&self, file_path: &std::path::Path) {
        // Try config first, then environment variables
        let editor_config = if let Some(ref editor) = self.config.editor {
            Some((editor.command.clone(), editor.args.clone()))
        } else {
            // Check common environment variables
            let editor_var = std::env::var("VISUAL")
                .or_else(|_| std::env::var("EDITOR"))
                .ok();
            
            editor_var.map(|cmd| {
                // Split command and args (simple parsing)
                let parts: Vec<String> = cmd.split_whitespace().map(|s| s.to_string()).collect();
                if parts.is_empty() {
                    (cmd, vec![])
                } else {
                    (parts[0].clone(), parts[1..].to_vec())
                }
            })
        };
        
        if let Some((command, args)) = editor_config {
            info!("Opening file in editor: {} {:?} {:?}", command, args, file_path);
            
            let mut cmd = std::process::Command::new(&command);
            for arg in &args {
                cmd.arg(arg);
            }
            cmd.arg(file_path);
            
            match cmd.spawn() {
                Ok(_) => {
                    info!("Opened file in editor: {:?}", file_path);
                }
                Err(e) => {
                    info!("Failed to open editor: {}", e);
                    // Fall back to trying common editors
                    self.try_fallback_editors(file_path);
                }
            }
        } else {
            // No editor configured, try common ones
            self.try_fallback_editors(file_path);
        }
    }
    
    /// Try common editors as fallback
    fn try_fallback_editors(&self, file_path: &std::path::Path) {
        #[cfg(target_os = "windows")]
        let editors = vec!["notepad++.exe", "notepad.exe"];
        
        #[cfg(not(target_os = "windows"))]
        let editors = vec!["code", "vim", "nano", "gedit", "kate"];
        
        for editor in editors {
            if std::process::Command::new(editor)
                .arg(file_path)
                .spawn()
                .is_ok()
            {
                info!("Opened file with {}: {:?}", editor, file_path);
                return;
            }
        }
        
        info!("Could not find any editor to open file");
    }
    
    /// Open a file in terminal with a pager (less, more, etc.)
    fn open_file_in_terminal(&self, file_path: &std::path::Path) {
        info!("Opening file in terminal with pager: {:?}", file_path);
        
        if let Some(terminal_config) = &self.config.terminal {
            let command = &terminal_config.command;
            let mut args = terminal_config.args.clone();
            
            // Build the pager command with auto-close on exit
            let pager_cmd = if terminal_config.pager_args.is_empty() {
                // Add "; exit" to close terminal when pager exits
                format!("{} \"{}\"; exit", terminal_config.pager, file_path.display())
            } else {
                format!("{} {} \"{}\"; exit", 
                    terminal_config.pager, 
                    terminal_config.pager_args.join(" "),
                    file_path.display())
            };
            
            info!("Terminal config found - Command: {}, Args: {:?}, Pager command: {}", 
                command, args, pager_cmd);
            
            // Special handling for different terminals
            #[cfg(target_os = "macos")]
            if command == "Terminal" {
                // macOS Terminal.app requires special handling via osascript
                let script = format!(
                    r#"tell application "Terminal"
                        set newTab to do script "{}"
                        activate
                        delay 0.5
                        repeat
                            delay 0.5
                            if not busy of newTab then exit repeat
                        end repeat
                        close window 1
                    end tell"#,
                    pager_cmd
                );
                
                match std::process::Command::new("osascript")
                    .arg("-e")
                    .arg(&script)
                    .spawn()
                {
                    Ok(_) => {
                        info!("Opened file in Terminal.app with {}", terminal_config.pager);
                        return;
                    }
                    Err(e) => {
                        info!("Failed to open Terminal.app: {}", e);
                    }
                }
            }
            
            // Special handling for Windows Terminal - launch in new tab
            #[cfg(target_os = "windows")]
            if command == "wt" || command == "wt.exe" {
                // For Windows Terminal, we need to construct the command differently
                // wt new-tab PowerShell.exe -NoExit -Command "less 'file.log'"
                let mut wt_args = vec!["new-tab".to_string()];
                
                // Add the shell and its args
                if args.len() >= 2 && args[0] == "-NoExit" && args[1] == "-Command" {
                    // This is PowerShell syntax
                    wt_args.push("pwsh.exe".to_string());
                    wt_args.extend(args.clone());
                    wt_args.push(pager_cmd.clone());
                } else {
                    // Generic case - add all args and pager command
                    wt_args.extend(args.clone());
                    wt_args.push(pager_cmd.clone());
                }
                
                info!("Executing Windows Terminal: {} with args: {:?}", command, wt_args);
                
                match std::process::Command::new(command)
                    .args(&wt_args)
                    .spawn()
                {
                    Ok(_) => {
                        info!("Successfully opened file in new Windows Terminal tab with {}", terminal_config.pager);
                        return;
                    }
                    Err(e) => {
                        info!("Failed to open Windows Terminal: {}", e);
                        // Continue to try standard execution
                    }
                }
            }
            
            // Special handling for launching via Windows Terminal when using pwsh directly
            #[cfg(target_os = "windows")]
            if command == "pwsh" || command == "pwsh.exe" || command == "powershell" || command == "powershell.exe" {
                // Try to launch via Windows Terminal in a new tab
                if args.len() >= 2 {
                    let wt_args = vec![
                        "new-tab".to_string(),
                        command.to_string(),
                        args[0].clone(),  // -NoExit
                        args[1].clone(),  // -Command
                        pager_cmd.clone()
                    ];
                    
                    info!("Trying to launch via Windows Terminal: wt with args: {:?}", wt_args);
                    
                    if std::process::Command::new("wt")
                        .args(&wt_args)
                        .spawn()
                        .is_ok()
                    {
                        info!("Successfully opened file in new Windows Terminal tab with {}", terminal_config.pager);
                        return;
                    }
                    
                    info!("Windows Terminal not available, falling back to direct execution");
                }
            }
            
            // Standard terminal command execution (fallback)
            args.push(pager_cmd);
            
            info!("Executing command: {} with args: {:?}", command, args);
            
            match std::process::Command::new(command)
                .args(&args)
                .spawn()
            {
                Ok(_) => {
                    info!("Successfully opened file in {} with {}", command, terminal_config.pager);
                }
                Err(e) => {
                    info!("Failed to open terminal '{}': {}", command, e);
                    info!("Falling back to default terminals...");
                    self.try_fallback_terminals(file_path);
                }
            }
        } else {
            // No terminal configured, try fallbacks
            info!("No terminal config found in config.yaml, using fallback terminals");
            self.try_fallback_terminals(file_path);
        }
    }
    
    /// Try common terminals as fallback
    fn try_fallback_terminals(&self, file_path: &std::path::Path) {
        info!("Trying fallback terminals for file: {:?}", file_path);
        
        #[cfg(target_os = "windows")]
        {
            // Try Windows Terminal, then PowerShell, then CMD
            let file_str = file_path.display().to_string();
            
            // Windows Terminal
            info!("Trying Windows Terminal with: wt -d . cmd /k more \"{}\"", file_str);
            if std::process::Command::new("wt")
                .args(&["-d", ".", "cmd", "/k", &format!("more \"{}\"", file_str)])
                .spawn()
                .is_ok()
            {
                info!("Opened file in Windows Terminal");
                return;
            }
            
            // PowerShell Core
            if std::process::Command::new("pwsh")
                .args(&["-NoExit", "-Command", &format!("Get-Content \"{}\" | more", file_str)])
                .spawn()
                .is_ok()
            {
                info!("Opened file in PowerShell Core");
                return;
            }
            
            // CMD
            if std::process::Command::new("cmd")
                .args(&["/k", &format!("more \"{}\"", file_str)])
                .spawn()
                .is_ok()
            {
                info!("Opened file in CMD");
                return;
            }
        }
        
        #[cfg(not(target_os = "windows"))]
        {
            let pager_cmd = format!("less -R \"{}\"; exit", file_path.display());
            
            // Try different terminals with their specific syntax
            let terminals = [
                ("gnome-terminal", "--", "sh", "-c"),
                ("konsole", "-e", "sh", "-c"),
                ("xfce4-terminal", "-x", "sh", "-c"),
                ("xterm", "-e", "sh", "-c"),
            ];
            
            for (terminal, arg1, arg2, arg3) in terminals {
                let mut cmd = std::process::Command::new(terminal);
                cmd.arg(arg1).arg(arg2).arg(arg3).arg(&pager_cmd);
                
                if cmd.spawn().is_ok() {
                    info!("Opened file in {} with less", terminal);
                    return;
                }
            }
        }
        
        info!("Could not find any terminal to open file");
    }
    
    /// Open a file path in the system file explorer (reusable static method)
    fn open_path_in_explorer(file_path: &std::path::Path) {
        // Print to console for debugging
        println!("Opening file in explorer: {:?}", file_path);
        
        if let Some(parent) = file_path.parent() {
            println!("Parent directory: {:?}", parent);
        }
        
        info!("Opening file in explorer: {:?}", file_path);
        
        #[cfg(target_os = "windows")]
        {
            // On Windows, use 'explorer /select,' to open Explorer and select the file
            // Windows Explorer requires backslashes in paths
            let path_str = file_path.to_string_lossy().replace('/', "\\");
            println!("Windows Explorer command: explorer /select,\"{}\"", path_str);
            info!("Windows Explorer command: explorer /select,\"{}\"", path_str);
            
            // Note: No space after /select, and path should be quoted
            if let Err(e) = std::process::Command::new("explorer")
                .arg(format!("/select,{}", path_str))
                .spawn()
            {
                println!("Failed to open explorer: {}", e);
                info!("Failed to open explorer: {}", e);
            } else {
                println!("Opened file in Explorer: {:?}", file_path);
                info!("Opened file in Explorer: {:?}", file_path);
            }
        }

        #[cfg(target_os = "macos")]
        {
            // On macOS, use 'open -R' to reveal in Finder
            if let Err(e) = std::process::Command::new("open")
                .args(&["-R", &file_path.to_string_lossy()])
                .spawn()
            {
                info!("Failed to open Finder: {}", e);
            } else {
                info!("Opened file in Finder: {:?}", file_path);
            }
        }

        #[cfg(target_os = "linux")]
        {
            // On Linux, try various file managers
            let file_managers = [
                ("nautilus", vec!["--select"]),
                ("dolphin", vec!["--select"]),
                ("nemo", vec![]),
                ("thunar", vec![]),
                ("xdg-open", vec![]),
            ];

            let parent_dir = file_path.parent().unwrap_or(file_path.as_ref());
            let mut opened = false;

            for (manager, args) in &file_managers {
                let mut cmd = std::process::Command::new(manager);
                for arg in args {
                    cmd.arg(arg);
                }
                cmd.arg(file_path.to_string_lossy().to_string());

                if cmd.spawn().is_ok() {
                    info!("Opened file with {}: {:?}", manager, file_path);
                    opened = true;
                    break;
                }
            }

            if !opened {
                // Fallback: just open the parent directory
                if let Err(e) = std::process::Command::new("xdg-open")
                    .arg(parent_dir.to_string_lossy().to_string())
                    .spawn()
                {
                    info!("Failed to open file manager: {}", e);
                } else {
                    info!("Opened parent directory: {:?}", parent_dir);
                }
            }
        }
    }

    fn yank_matched_line(&mut self) {
        if let Some(matched_line) = &self.preview.matched_line_text {
            match Clipboard::new() {
                Ok(mut clipboard) => match clipboard.set_text(matched_line.clone()) {
                    Ok(_) => info!(
                        "Yanked matched line ({} chars) to clipboard",
                        matched_line.len()
                    ),
                    Err(e) => info!("Failed to yank matched line to clipboard: {}", e),
                },
                Err(e) => info!("Failed to access clipboard: {}", e),
            }
        } else {
            info!("No matched line to yank");
        }
    }

    fn select_first_match(&mut self) {
        if self.grep_state.results.is_empty() {
            return;
        }

        // Find first file with matches
        for file_idx in 0..self.grep_state.results.len() {
            if !self.grep_state.results[file_idx].matches.is_empty() {
                let result_id = file_idx * 10000;
                let file_path = self.grep_state.results[file_idx].file_path.clone();
                let line_number = self.grep_state.results[file_idx].matches[0].line_number;
                self.select_match_with_keyboard(result_id, &file_path, line_number);
                return;
            }
        }
    }

    fn select_last_match(&mut self) {
        if self.grep_state.results.is_empty() {
            return;
        }

        // Find last file with matches, and last match in that file
        for file_idx in (0..self.grep_state.results.len()).rev() {
            if !self.grep_state.results[file_idx].matches.is_empty() {
                let last_match_idx = self.grep_state.results[file_idx].matches.len() - 1;
                let result_id = file_idx * 10000 + last_match_idx;
                let file_path = self.grep_state.results[file_idx].file_path.clone();
                let line_number =
                    self.grep_state.results[file_idx].matches[last_match_idx].line_number;
                self.select_match_with_keyboard(result_id, &file_path, line_number);
                return;
            }
        }
    }

    fn select_first_match_in_current_file(&mut self) {
        if self.grep_state.results.is_empty() {
            return;
        }

        let current_id = self.grep_state.selected_result.unwrap_or(0);
        let current_file_idx = current_id / 10000;

        if current_file_idx < self.grep_state.results.len()
            && !self.grep_state.results[current_file_idx].matches.is_empty()
        {
            let result_id = current_file_idx * 10000;
            let file_path = self.grep_state.results[current_file_idx].file_path.clone();
            let line_number = self.grep_state.results[current_file_idx].matches[0].line_number;
            self.select_match_with_keyboard(result_id, &file_path, line_number);
        }
    }

    fn select_last_match_in_current_file(&mut self) {
        if self.grep_state.results.is_empty() {
            return;
        }

        let current_id = self.grep_state.selected_result.unwrap_or(0);
        let current_file_idx = current_id / 10000;

        if current_file_idx < self.grep_state.results.len()
            && !self.grep_state.results[current_file_idx].matches.is_empty()
        {
            let last_match_idx = self.grep_state.results[current_file_idx].matches.len() - 1;
            let result_id = current_file_idx * 10000 + last_match_idx;
            let file_path = self.grep_state.results[current_file_idx].file_path.clone();
            let line_number =
                self.grep_state.results[current_file_idx].matches[last_match_idx].line_number;
            self.select_match_with_keyboard(result_id, &file_path, line_number);
        }
    }

    fn select_next_file(&mut self) {
        if self.grep_state.results.is_empty() {
            return;
        }

        let current_id = self.grep_state.selected_result.unwrap_or(0);
        let current_file_idx = current_id / 10000;

        // Move to first match in next file
        for file_idx in (current_file_idx + 1)..self.grep_state.results.len() {
            if !self.grep_state.results[file_idx].matches.is_empty() {
                let next_id = file_idx * 10000;
                let file_path = self.grep_state.results[file_idx].file_path.clone();
                let line_number = self.grep_state.results[file_idx].matches[0].line_number;
                self.select_match_with_keyboard(next_id, &file_path, line_number);
                return;
            }
        }

        // Wrap to first file
        for file_idx in 0..self.grep_state.results.len() {
            if !self.grep_state.results[file_idx].matches.is_empty() {
                let next_id = file_idx * 10000;
                let file_path = self.grep_state.results[file_idx].file_path.clone();
                let line_number = self.grep_state.results[file_idx].matches[0].line_number;
                self.select_match_with_keyboard(next_id, &file_path, line_number);
                return;
            }
        }
    }

    fn select_previous_file(&mut self) {
        if self.grep_state.results.is_empty() {
            return;
        }

        let current_id = self.grep_state.selected_result.unwrap_or(0);
        let current_file_idx = current_id / 10000;

        // Move to first match in previous file
        for file_idx in (0..current_file_idx).rev() {
            if !self.grep_state.results[file_idx].matches.is_empty() {
                let prev_id = file_idx * 10000;
                let file_path = self.grep_state.results[file_idx].file_path.clone();
                let line_number = self.grep_state.results[file_idx].matches[0].line_number;
                self.select_match_with_keyboard(prev_id, &file_path, line_number);
                return;
            }
        }

        // Wrap to last file
        for file_idx in (0..self.grep_state.results.len()).rev() {
            if !self.grep_state.results[file_idx].matches.is_empty() {
                let prev_id = file_idx * 10000;
                let file_path = self.grep_state.results[file_idx].file_path.clone();
                let line_number = self.grep_state.results[file_idx].matches[0].line_number;
                self.select_match_with_keyboard(prev_id, &file_path, line_number);
                return;
            }
        }
    }

    fn select_previous_match(&mut self) {
        if self.grep_state.results.is_empty() {
            return;
        }

        let current_id = self.grep_state.selected_result.unwrap_or(0);
        let current_file_idx = current_id / 10000;
        let current_match_idx = current_id % 10000;

        // Try previous match in current file
        if current_match_idx > 0 {
            let prev_id = current_file_idx * 10000 + current_match_idx - 1;
            let file_path = self.grep_state.results[current_file_idx].file_path.clone();
            let line_number = self.grep_state.results[current_file_idx].matches
                [current_match_idx - 1]
                .line_number;
            self.select_match_with_keyboard(prev_id, &file_path, line_number);
            return;
        }

        // Move to last match in previous file
        for file_idx in (0..current_file_idx).rev() {
            if !self.grep_state.results[file_idx].matches.is_empty() {
                let last_match_idx = self.grep_state.results[file_idx].matches.len() - 1;
                let prev_id = file_idx * 10000 + last_match_idx;
                let file_path = self.grep_state.results[file_idx].file_path.clone();
                let line_number =
                    self.grep_state.results[file_idx].matches[last_match_idx].line_number;
                self.select_match_with_keyboard(prev_id, &file_path, line_number);
                return;
            }
        }

        // Wrap to last match in last file
        for file_idx in (0..self.grep_state.results.len()).rev() {
            if !self.grep_state.results[file_idx].matches.is_empty() {
                let last_match_idx = self.grep_state.results[file_idx].matches.len() - 1;
                let last_id = file_idx * 10000 + last_match_idx;
                let file_path = self.grep_state.results[file_idx].file_path.clone();
                let line_number =
                    self.grep_state.results[file_idx].matches[last_match_idx].line_number;
                self.select_match_with_keyboard(last_id, &file_path, line_number);
                return;
            }
        }
    }

    fn render_results(&mut self, ui: &mut egui::Ui) {
        let filter = self.grep_state.results_filter.to_lowercase();
        let mut clicked_match: Option<(usize, std::path::PathBuf, usize)> = None;
        let should_scroll = self.scroll_to_selected_result;
        self.scroll_to_selected_result = false; // Reset flag

        for (file_idx, result) in self.grep_state.results.iter().enumerate() {
            let file_name = result
                .file_path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("unknown");

            // Apply filename filter
            if !filter.is_empty() && !file_name.to_lowercase().contains(&filter) {
                continue;
            }

            // Get current open state, default to true if not set
            let is_open = *self
                .grep_state
                .collapsing_state
                .get(&file_idx)
                .unwrap_or(&true);

            let header_id = ui.make_persistent_id(format!("header_{}", file_idx));

            // Load the state from egui's storage (respects user clicks)
            let mut state = egui::collapsing_header::CollapsingState::load_with_default_open(
                ui.ctx(),
                header_id,
                is_open,
            );

            // Only force the state if our tracked state differs from egui's state
            // This allows user clicks to work, but also allows Expand/Collapse All buttons to work
            if state.is_open() != is_open {
                state.set_open(is_open);
                state.store(ui.ctx());
            }

            state
                .show_header(ui, |ui| {
                    ui.label(format!("{} ({} matches)", file_name, result.matches.len()));
                })
                .body(|ui| {
                    for (match_idx, m) in result.matches.iter().enumerate() {
                        let result_id = file_idx * 10000 + match_idx;
                        let is_selected = self.grep_state.selected_result == Some(result_id);

                        let label = format!("  Line {}: {}", m.line_number, m.line_text.trim());

                        let response = ui.selectable_label(is_selected, label);

                        if response.clicked() {
                            clicked_match =
                                Some((result_id, result.file_path.clone(), m.line_number));
                        }

                        // Scroll to this item if it's selected and we should scroll
                        if is_selected && should_scroll {
                            response.scroll_to_me(Some(egui::Align::Center));
                        }
                    }
                });

            // Re-load state to get updated open/close status after user interaction
            let updated_state = egui::collapsing_header::CollapsingState::load_with_default_open(
                ui.ctx(),
                header_id,
                is_open,
            );
            self.grep_state
                .collapsing_state
                .insert(file_idx, updated_state.is_open());
        }

        // Handle match selection after iteration is complete
        if let Some((result_id, file_path, line_number)) = clicked_match {
            self.select_match(result_id, &file_path, line_number);
        }
    }

    fn render_preview(&mut self, ui: &mut egui::Ui) {
        if let Some(preview_text) = &self.preview.content {
            // Check if we should try syntax highlighting based on selected result
            let should_highlight = if let Some(selected_id) = self.grep_state.selected_result {
                let file_idx = selected_id / 10000;
                self.grep_state
                    .results
                    .get(file_idx)
                    .map(|r| self.should_highlight_file(&r.file_path))
                    .unwrap_or(false)
            } else {
                false
            };

            if should_highlight {
                // Use egui_extras syntax highlighting
                let mut layouter = |ui: &egui::Ui, string: &str, wrap_width: f32| {
                    let mut layout_job = egui_extras::syntax_highlighting::highlight(
                        ui.ctx(),
                        ui.style().as_ref(),
                        &egui_extras::syntax_highlighting::CodeTheme::from_memory(
                            ui.ctx(),
                            ui.style().as_ref(),
                        ),
                        string,
                        "rs", // Default to rust, we can make this smarter later
                    );
                    layout_job.wrap.max_width = wrap_width;
                    ui.fonts(|f| f.layout_job(layout_job))
                };

                ui.add(
                    egui::TextEdit::multiline(&mut preview_text.as_str())
                        .code_editor()
                        .desired_width(f32::INFINITY)
                        .desired_rows(100)
                        .layouter(&mut layouter),
                );
            } else {
                // Plain text for non-code files
                // Always use custom rendering to highlight matched line
                self.render_preview_with_highlights(ui, preview_text);
            }
        } else {
            ui.label("Select a result to preview");
        }
    }

    fn render_matched_line_focus(&self, ui: &mut egui::Ui) {
        use egui::{Color32, RichText};

        if let Some(matched_line) = &self.preview.matched_line_text {
            ui.style_mut().override_text_style = Some(egui::TextStyle::Monospace);
            
            // Apply custom font size
            let font_id = egui::FontId::new(self.grep_state.font_size, egui::FontFamily::Monospace);
            ui.style_mut().text_styles.insert(egui::TextStyle::Monospace, font_id);

            let highlight_color = Color32::from_rgb(255, 200, 100); // Orange/yellow
            let highlight_bg = Color32::from_rgb(80, 60, 40); // Brown background

            // Use highlight pattern if specified, otherwise use search query
            let pattern_to_use = if !self.grep_state.fix_highlight_pattern.is_empty() {
                &self.grep_state.fix_highlight_pattern
            } else {
                &self.grep_state.search_query
            };

            let has_pattern = !pattern_to_use.is_empty();

            if has_pattern && matched_line.contains(pattern_to_use) {
                // Render with highlighted pattern
                ui.horizontal_wrapped(|ui| {
                    ui.spacing_mut().item_spacing.x = 0.0;

                    let parts: Vec<&str> = matched_line.split(pattern_to_use).collect();

                    for (i, part) in parts.iter().enumerate() {
                        if !part.is_empty() {
                            ui.label(*part);
                        }

                        // Add highlighted pattern between parts (except after last part)
                        if i < parts.len() - 1 {
                            ui.label(
                                RichText::new(pattern_to_use)
                                    .color(highlight_color)
                                    .background_color(highlight_bg)
                                    .strong(),
                            );
                        }
                    }
                });
            } else {
                // Just show the line normally
                ui.label(matched_line);
            }
        } else {
            ui.label(
                RichText::new("Select a match to see the line here")
                    .italics()
                    .color(Color32::GRAY),
            );
        }
    }

    fn render_preview_with_highlights(&self, ui: &mut egui::Ui, text: &str) {
        use egui::Color32;

        egui::ScrollArea::neither()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                ui.style_mut().override_text_style = Some(egui::TextStyle::Monospace);
                
                // Apply custom font size
                let font_id = egui::FontId::new(self.grep_state.font_size, egui::FontFamily::Monospace);
                ui.style_mut().text_styles.insert(egui::TextStyle::Monospace, font_id);

                let match_line_bg = Color32::from_rgb(60, 60, 80); // Subtle blue-gray for matched line

                for line in text.lines() {
                    let is_match_line = line.starts_with(">>>");

                    // Apply background color for matched line
                    if is_match_line {
                        let frame = egui::Frame::none()
                            .fill(match_line_bg)
                            .inner_margin(egui::Margin::symmetric(4.0, 2.0));

                        frame.show(ui, |ui| {
                            ui.label(line);
                        });
                    } else {
                        // Regular line
                        ui.label(line);
                    }
                }
            });
    }

    fn should_highlight_file(&self, path: &std::path::Path) -> bool {
        if let Some(ext) = path.extension().and_then(|s| s.to_str()) {
            matches!(
                ext,
                "rs" | "toml"
                    | "js"
                    | "ts"
                    | "tsx"
                    | "jsx"
                    | "py"
                    | "java"
                    | "c"
                    | "cpp"
                    | "h"
                    | "hpp"
                    | "go"
                    | "rb"
                    | "php"
                    | "cs"
                    | "swift"
                    | "kt"
                    | "scala"
                    | "sh"
                    | "bash"
                    | "json"
                    | "xml"
                    | "html"
                    | "css"
                    | "md"
                    | "yaml"
                    | "yml"
                    | "sql"
            )
        } else {
            false
        }
    }

    // ============================================================================
    // UI Rendering Functions - Extracted from update()
    // ============================================================================

    /// Render the header with title and status indicators
    fn render_header(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.heading("VisGrep");

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                // Theme toggle button
                if ui.button(format!("Theme: {}", self.theme.name())).clicked() {
                    self.theme.cycle();
                    self.config.theme = self.theme;
                    // Save config with new theme
                    if let Err(e) = self.config.save() {
                        log::error!("Failed to save config: {}", e);
                    }
                }
                
                ui.separator();
                
                // Show pending input state (e.g., "3" or "g")
                let status = self.input_handler.get_status();
                if !status.is_empty() {
                    ui.label(format!("Command: {}", status));
                }

                // Show active marks
                if !self.marks.is_empty() {
                    let marks_str: String = self.marks.keys().collect();
                    ui.label(format!("Marks: {}", marks_str));
                }
            });
        });
    }

    /// Render mode selector tabs
    fn render_mode_tabs(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.selectable_value(&mut self.mode, AppMode::Grep, "🔍 Grep Mode");
            ui.selectable_value(&mut self.mode, AppMode::Tail, "📄 Tail Mode");
            ui.selectable_value(&mut self.mode, AppMode::Test, "🔧 Test Mode");
        });
    }




    /// Render the highlight pattern field
    fn render_highlight_pattern_field(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.label("Highlight pattern in Matched Line (e.g., 150= or fn):");
            let response = ui.add(
                egui::TextEdit::singleline(&mut self.grep_state.fix_highlight_pattern)
                    .desired_width(150.0)
                    .hint_text("uses search query if empty"),
            );

            // Show active indicator
            let active_pattern = if !self.grep_state.fix_highlight_pattern.is_empty() {
                &self.grep_state.fix_highlight_pattern
            } else {
                &self.grep_state.search_query
            };

            if !active_pattern.is_empty() {
                ui.label(
                    egui::RichText::new(format!("✓ Active: '{}'", active_pattern))
                        .color(egui::Color32::from_rgb(100, 255, 100)),
                );
            }

            if ui.small_button("Clear").clicked() {
                self.grep_state.fix_highlight_pattern.clear();
            }

            // Log when pattern changes
            if response.changed() {
                info!(
                    "Highlight pattern changed to: '{}'",
                    self.grep_state.fix_highlight_pattern
                );
            }
        });
    }

    /// Render the search path field with folder presets
    fn render_search_path_field(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.label("Search Path:");
            ui.add(
                egui::TextEdit::singleline(&mut self.grep_state.search_path).desired_width(350.0),
            );

            // Preset folders dropdown
            egui::ComboBox::from_id_salt("folder_presets")
                .selected_text("📁")
                .width(40.0)
                .show_ui(ui, |ui| {
                    for preset in &self.config.folder_presets {
                        if ui.selectable_label(false, &preset.name).clicked() {
                            self.grep_state.search_path = Self::expand_tilde(&preset.path);
                            info!(
                                "Selected preset: {} -> {}",
                                preset.name, self.grep_state.search_path
                            );
                        }
                    }
                });

            if ui.button("Current Dir").clicked() {
                if let Ok(cwd) = std::env::current_dir() {
                    self.grep_state.search_path = cwd.display().to_string();
                }
            }

            if ui.button("Browse...").clicked() {
                match rfd::FileDialog::new().pick_folder() {
                    Some(path) => {
                        self.grep_state.search_path = path.display().to_string();
                        info!("Selected folder: {}", self.grep_state.search_path);
                    }
                    None => {
                        info!("Browse dialog cancelled or unavailable");
                    }
                }
            }

            ui.label("File Pattern:");
            ui.add(
                egui::TextEdit::singleline(&mut self.grep_state.file_pattern).desired_width(150.0),
            );
            if ui.small_button("Clear").clicked() {
                self.grep_state.file_pattern.clear();
            }
        });
    }

    /// Render the search query field with patterns dropdown
    fn render_search_query_field(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.label("Search Query:");
            let response = ui.add(
                egui::TextEdit::singleline(&mut self.grep_state.search_query).desired_width(300.0),
            );

            // Saved patterns dropdown
            if !self.config.saved_patterns.is_empty() {
                self.render_patterns_dropdown(ui);
            }

            // Debounced auto-search: trigger search 500ms after typing stops
            if response.changed() {
                self.grep_state.pending_search = true;
                self.grep_state.last_search_time = Instant::now();
            }

            if response.lost_focus()
                && ui.input(|i| i.key_pressed(egui::Key::Enter))
                && !self.grep_state.search_query.is_empty()
            {
                self.perform_search();
            }

            ui.checkbox(&mut self.grep_state.case_sensitive, "Case Sensitive");
            ui.checkbox(&mut self.grep_state.use_regex, "Regex");
            ui.checkbox(&mut self.grep_state.recursive, "Recursive");

            if ui.button("Search").clicked() && !self.grep_state.search_query.is_empty() {
                self.perform_search();
            }
        });
    }

    /// Render the saved patterns dropdown
    fn render_patterns_dropdown(&mut self, ui: &mut egui::Ui) {
        egui::ComboBox::from_id_salt("saved_patterns")
            .selected_text("📝")
            .width(40.0)
            .show_ui(ui, |ui| {
                // Group by category if available
                let mut by_category: std::collections::HashMap<String, Vec<&config::SavedPattern>> =
                    std::collections::HashMap::new();

                for pattern in &self.config.saved_patterns {
                    let cat = if pattern.category.is_empty() {
                        "Other".to_string()
                    } else {
                        pattern.category.clone()
                    };
                    by_category.entry(cat).or_default().push(pattern);
                }

                let mut categories: Vec<_> = by_category.keys().collect();
                categories.sort();

                for category in categories {
                    if let Some(patterns) = by_category.get(category) {
                        if by_category.len() > 1 {
                            ui.label(egui::RichText::new(category).strong());
                            ui.separator();
                        }

                        for pattern in patterns {
                            let label = if pattern.description.is_empty() {
                                pattern.name.clone()
                            } else {
                                pattern.name.to_string()
                            };

                            let mut button = ui.selectable_label(false, label);

                            if !pattern.description.is_empty() {
                                button = button.on_hover_text(&pattern.description);
                            }

                            if button.clicked() {
                                self.grep_state.search_query = pattern.pattern.clone();
                                info!("Loaded pattern: {} -> {}", pattern.name, pattern.pattern);
                            }
                        }

                        if by_category.len() > 1 {
                            ui.separator();
                        }
                    }
                }
            });
    }

    /// Render file age filter controls
    fn render_file_age_filter(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.label("File Age:");
            let mut enabled = self.grep_state.file_age_hours.is_some();
            ui.checkbox(&mut enabled, "Filter by age");

            if enabled {
                let mut hours = self.grep_state.file_age_hours.unwrap_or(24);
                ui.add(egui::DragValue::new(&mut hours).speed(1.0).range(1..=8760));
                ui.label("hours");
                self.grep_state.file_age_hours = Some(hours);
            } else {
                self.grep_state.file_age_hours = None;
            }

            if ui.small_button("?").clicked() {
                info!("File Age Filter: Only search files modified within the specified hours");
            }
        });
    }

    /// Render status bar showing search stats
    fn render_status_bar(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            match self.mode {
                AppMode::Grep => {
                    let total_matches: usize = self
                        .grep_state
                        .results
                        .iter()
                        .map(|r| r.matches.len())
                        .sum();
                    let file_count = self.grep_state.results.len();

                    ui.label(format!(
                        "Found {} matches in {} files",
                        total_matches, file_count
                    ));

                    if self.grep_state.searching {
                        ui.spinner();
                        ui.label("Searching...");
                    }
                },
                AppMode::Tail => {
                    // Tail mode status - show file and buffer info
                    let buffer_pct = if self.tail_state.max_buffer_lines > 0 {
                        (self.tail_state.output_buffer.len() as f32
                            / self.tail_state.max_buffer_lines as f32)
                            * 100.0
                    } else {
                        0.0
                    };

                    let active_count = self.tail_state.files.iter().filter(|f| f.is_active).count();

                    ui.label(format!(
                        "Files: {}  Active: {}  Lines: {} / {}  Buffer: {:.1}%  Update: {}ms",
                        self.tail_state.files.len(),
                        active_count,
                        self.tail_state.output_buffer.len(),
                        self.tail_state.max_buffer_lines,
                        buffer_pct,
                        self.tail_state.poll_interval_ms
                    ));

                    if self.tail_state.lines_dropped > 0 {
                        ui.colored_label(
                            egui::Color32::YELLOW,
                            format!("  ⚠ Dropped: {}", self.tail_state.lines_dropped),
                        );
                    }
                },
                AppMode::Test => {
                    ui.label("Test Mode - Splitter working!");
                },
            }
            
            // Add memory info and idle status to the right side of status bar
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(self.memory_monitor.status_string());
                
                let idle_status = self.idle_monitor.status_string();
                if !idle_status.is_empty() {
                    ui.separator();
                    ui.label(idle_status);
                }
            });
        });
    }
}

fn main() -> eframe::Result<()> {
    // Force X11 backend on Linux for WSL compatibility
    #[cfg(target_os = "linux")]
    std::env::set_var("WINIT_UNIX_BACKEND", "x11");

    // Initialize logging
    // Respect RUST_LOG environment variable, default to warn if not set
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("warn"))
        .init();

    // Parse command-line arguments
    let cli = Cli::parse();
    
    // Print config path for debugging
    if let Some(config_path) = Config::config_path() {
        info!("Config file location: {:?}", config_path);
        if !config_path.exists() {
            info!("Config file does not exist. Creating example config...");
            if let Err(e) = Config::create_example() {
                warn!("Failed to create example config: {}", e);
            } else {
                info!("Created example config at {:?}", config_path);
            }
        }
    }

    // Determine startup configuration
    let startup_config = match cli.command {
        Some(Commands::Tail { files }) => {
            info!("Starting in Tail mode with files: {:?}", files);
            StartupConfig {
                mode: AppMode::Tail,
                tail_files: files,
                tail_layout: cli.tail_layout,
            }
        }
        None => {
            if cli.follow || !cli.files.is_empty() || cli.tail_layout.is_some() {
                // -f flag, files provided, or layout specified
                if let Some(ref layout) = cli.tail_layout {
                    info!("Starting in Tail mode with layout file: {:?}", layout);
                } else {
                    info!(
                        "Starting in Tail mode (via -f flag) with files: {:?}",
                        cli.files
                    );
                }
                StartupConfig {
                    mode: AppMode::Tail,
                    tail_files: cli.files,
                    tail_layout: cli.tail_layout,
                }
            } else {
                // Default: Grep mode
                info!("Starting in Grep mode (default)");
                StartupConfig::default()
            }
        }
    };

    info!("VisGrep starting in {:?} mode...", startup_config.mode);

    // Set up persistence path for window size and splitter positions
    let persistence_path = if let Some(config_path) = Config::config_path() {
        let path = config_path.parent()
            .map(|p| p.join("app_state.ron"));
        if let Some(ref p) = path {
            info!("Persistence path: {:?}", p);
        }
        path
    } else {
        None
    };

    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1400.0, 900.0])
            .with_title("VisGrep - Fast Search & Tail Tool"),
        persistence_path,
        ..Default::default()
    };

    eframe::run_native(
        "VisGrep",
        native_options,
        Box::new(move |cc| {
            // Set dark theme
            let mut visuals = egui::Visuals::dark();
            // Ensure good contrast for panels
            visuals.window_shadow = egui::epaint::Shadow::NONE;
            cc.egui_ctx.set_visuals(visuals);
            Ok(Box::new(VisGrepApp::new(startup_config)))
        }),
    )
}

// ============================================================================
// Helper Functions
// ============================================================================

// Helper function for color coding files
fn get_color_for_file(filename: &str) -> egui::Color32 {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let mut hasher = DefaultHasher::new();
    filename.hash(&mut hasher);
    let hash = hasher.finish();

    // Generate distinguishable colors
    let hue = (hash % 12) as f32 * 30.0; // 12 colors around the wheel
    let (r, g, b) = hsl_to_rgb(hue, 0.7, 0.6);
    egui::Color32::from_rgb(r, g, b)
}

// Convert HSL to RGB
fn hsl_to_rgb(h: f32, s: f32, l: f32) -> (u8, u8, u8) {
    let c = (1.0 - (2.0 * l - 1.0).abs()) * s;
    let x = c * (1.0 - ((h / 60.0) % 2.0 - 1.0).abs());
    let m = l - c / 2.0;

    let (r, g, b) = if h < 60.0 {
        (c, x, 0.0)
    } else if h < 120.0 {
        (x, c, 0.0)
    } else if h < 180.0 {
        (0.0, c, x)
    } else if h < 240.0 {
        (0.0, x, c)
    } else if h < 300.0 {
        (x, 0.0, c)
    } else {
        (c, 0.0, x)
    };

    (
        ((r + m) * 255.0) as u8,
        ((g + m) * 255.0) as u8,
        ((b + m) * 255.0) as u8,
    )
}
