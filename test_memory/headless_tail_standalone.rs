use std::collections::{HashMap, VecDeque};
use std::fs::File;
use std::io::{BufRead, BufReader, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant, SystemTime};

// Simple log level detection
#[derive(Debug, Clone, Copy)]
enum LogLevel {
    Fatal,
    Error,
    Warn,
    Info,
    Debug,
    Trace,
}

struct LogLevelDetector {
    patterns: Vec<(&'static str, LogLevel)>,
}

impl LogLevelDetector {
    fn new() -> Self {
        Self {
            patterns: vec![
                ("FATAL", LogLevel::Fatal),
                ("ERROR", LogLevel::Error),
                ("WARN", LogLevel::Warn),
                ("WARNING", LogLevel::Warn),
                ("INFO", LogLevel::Info),
                ("DEBUG", LogLevel::Debug),
                ("TRACE", LogLevel::Trace),
            ],
        }
    }

    fn detect(&self, line: &str) -> Option<LogLevel> {
        let upper = line.to_uppercase();
        for (pattern, level) in &self.patterns {
            if upper.contains(pattern) {
                return Some(*level);
            }
        }
        None
    }
}

// Simple string interning cache
struct StringCache {
    cache: Arc<Mutex<HashMap<String, Arc<str>>>>,
    max_entries: usize,
}

impl StringCache {
    fn new(max_entries: usize) -> Self {
        Self {
            cache: Arc::new(Mutex::new(HashMap::new())),
            max_entries,
        }
    }

    fn intern(&self, s: &str) -> Arc<str> {
        let mut cache = self.cache.lock().unwrap();
        
        if let Some(arc) = cache.get(s) {
            return arc.clone();
        }
        
        // If cache is full, clear it (simple strategy)
        if cache.len() >= self.max_entries {
            cache.clear();
        }
        
        let arc: Arc<str> = Arc::from(s);
        cache.insert(s.to_string(), arc.clone());
        arc
    }

    fn size(&self) -> usize {
        self.cache.lock().unwrap().len()
    }
}

// File watcher
struct FileWatcher {
    path: PathBuf,
    file: Option<File>,
    position: u64,
    last_size: u64,
}

impl FileWatcher {
    fn new(path: PathBuf) -> Self {
        Self {
            path,
            file: None,
            position: 0,
            last_size: 0,
        }
    }

    fn read_new_lines(&mut self) -> Vec<String> {
        let mut new_lines = Vec::new();

        // Open file if not already open
        if self.file.is_none() {
            match File::open(&self.path) {
                Ok(file) => {
                    self.file = Some(file);
                    // Start from beginning for testing
                    self.position = 0;
                    if let Ok(metadata) = std::fs::metadata(&self.path) {
                        self.last_size = metadata.len();
                    }
                }
                Err(_) => return new_lines,
            }
        }

        let file = self.file.as_mut().unwrap();

        // Check current file size
        let current_size = match std::fs::metadata(&self.path) {
            Ok(metadata) => metadata.len(),
            Err(_) => return new_lines,
        };

        // If file shrunk, it was probably rotated
        if current_size < self.last_size {
            self.position = 0;
        }

        // Seek to last position
        if file.seek(SeekFrom::Start(self.position)).is_err() {
            return new_lines;
        }

        // Read new lines
        let mut reader = BufReader::new(file);
        let mut line = String::new();
        
        while reader.read_line(&mut line).unwrap_or(0) > 0 {
            if !line.is_empty() {
                // Remove trailing newline
                if line.ends_with('\n') {
                    line.pop();
                    if line.ends_with('\r') {
                        line.pop();
                    }
                }
                new_lines.push(line.clone());
            }
            line.clear();
        }

        // Update position
        self.position = current_size;
        self.last_size = current_size;

        new_lines
    }
}

// Log line structure
struct LogLine {
    file_name: Arc<str>,
    content: Arc<str>,
    line_number: usize,
    timestamp: f64,
    level: Option<LogLevel>,
}

// Memory tracking
struct MemoryTracker;

impl MemoryTracker {
    fn current_memory_mb() -> f64 {
        #[cfg(target_os = "linux")]
        {
            if let Ok(contents) = std::fs::read_to_string("/proc/self/status") {
                for line in contents.lines() {
                    if line.starts_with("VmRSS:") {
                        if let Some(kb_str) = line.split_whitespace().nth(1) {
                            if let Ok(kb) = kb_str.parse::<f64>() {
                                return kb / 1024.0;
                            }
                        }
                    }
                }
            }
        }
        0.0
    }
}

// Main test state
struct HeadlessTailState {
    output_buffer: VecDeque<LogLine>,
    max_buffer_lines: usize,
    lines_processed: usize,
    start_time: Instant,
    last_memory_report: Instant,
    peak_memory: f64,
    initial_memory: f64,
    string_cache: StringCache,
    log_detector: LogLevelDetector,
}

impl HeadlessTailState {
    fn new(max_buffer_lines: usize) -> Self {
        let initial_memory = MemoryTracker::current_memory_mb();
        Self {
            output_buffer: VecDeque::new(),
            max_buffer_lines,
            lines_processed: 0,
            start_time: Instant::now(),
            last_memory_report: Instant::now(),
            peak_memory: initial_memory,
            initial_memory,
            string_cache: StringCache::new(10_000),
            log_detector: LogLevelDetector::new(),
        }
    }

    fn add_line(&mut self, file_name: &str, line_content: String) {
        // Intern strings
        let interned_content = self.string_cache.intern(&line_content);
        let interned_file = self.string_cache.intern(file_name);
        
        // Detect log level
        let level = self.log_detector.detect(&line_content);
        
        let log_line = LogLine {
            file_name: interned_file,
            content: interned_content,
            line_number: self.lines_processed,
            timestamp: SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap()
                .as_secs_f64(),
            level,
        };
        
        self.output_buffer.push_back(log_line);
        self.lines_processed += 1;

        // Maintain buffer size
        while self.output_buffer.len() > self.max_buffer_lines {
            self.output_buffer.pop_front();
        }

        // Aggressive capacity management
        if self.lines_processed % 100 == 0 {
            let desired_capacity = self.output_buffer.len() + 100;
            if self.output_buffer.capacity() > desired_capacity * 2 {
                self.output_buffer.shrink_to(desired_capacity);
            }
        }

        // Report memory usage every second
        if self.last_memory_report.elapsed() > Duration::from_secs(1) {
            self.report_memory();
        }
    }

    fn report_memory(&mut self) {
        let current_memory = MemoryTracker::current_memory_mb();
        self.peak_memory = self.peak_memory.max(current_memory);
        
        println!(
            "[{:6.1}s] Lines: {:8}, Buffer: {:6}, Memory: {:7.1}MB (delta: {:+7.1}MB, peak: {:7.1}MB), Cache: {:5}",
            self.start_time.elapsed().as_secs_f64(),
            self.lines_processed,
            self.output_buffer.len(),
            current_memory,
            current_memory - self.initial_memory,
            self.peak_memory,
            self.string_cache.size()
        );
        
        self.last_memory_report = Instant::now();
    }

    fn final_report(&self) {
        println!("\n=== Test Complete ===");
        println!("Total lines processed: {}", self.lines_processed);
        println!("Final buffer size: {}", self.output_buffer.len());
        println!("Initial memory: {:.1}MB", self.initial_memory);
        println!("Final memory: {:.1}MB", MemoryTracker::current_memory_mb());
        println!("Peak memory: {:.1}MB", self.peak_memory);
        println!("Memory growth: {:.1}MB", self.peak_memory - self.initial_memory);
        println!("String cache entries: {}", self.string_cache.size());
        
        if self.lines_processed > 0 {
            println!("Average memory per line: {:.3}KB", 
                (self.peak_memory - self.initial_memory) * 1024.0 / self.lines_processed as f64);
        }
    }
}

fn main() {
    println!("=== Standalone Headless Tail Mode Memory Test ===");
    let initial_memory = MemoryTracker::current_memory_mb();
    println!("Starting memory: {:.1}MB", initial_memory);
    
    // Parse command line arguments
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: {} <file1> [file2 ...] [--buffer-size N] [--duration S]", args[0]);
        eprintln!("  --buffer-size N   Set max buffer lines (default: 10000)");
        eprintln!("  --duration S      Test duration in seconds (default: 60)");
        std::process::exit(1);
    }
    
    let mut files = Vec::new();
    let mut max_buffer_lines = 10_000;
    let mut test_duration_secs = 60;
    let mut i = 1;
    
    while i < args.len() {
        match args[i].as_str() {
            "--buffer-size" => {
                if i + 1 < args.len() {
                    max_buffer_lines = args[i + 1].parse().unwrap_or(10_000);
                    i += 1;
                }
            }
            "--duration" => {
                if i + 1 < args.len() {
                    test_duration_secs = args[i + 1].parse().unwrap_or(60);
                    i += 1;
                }
            }
            file => files.push(PathBuf::from(file)),
        }
        i += 1;
    }
    
    if files.is_empty() {
        eprintln!("No files specified!");
        std::process::exit(1);
    }
    
    println!("Watching {} files", files.len());
    println!("Max buffer lines: {}", max_buffer_lines);
    println!("Test duration: {} seconds", test_duration_secs);
    
    // Create file watchers
    let mut watchers: Vec<FileWatcher> = files.iter()
        .map(|path| {
            println!("Watching file: {}", path.display());
            FileWatcher::new(path.clone())
        })
        .collect();
    
    // Create state
    let mut state = HeadlessTailState::new(max_buffer_lines);
    
    // Main loop
    let test_duration = Duration::from_secs(test_duration_secs);
    let start = Instant::now();
    
    println!("\nStarting test for {} seconds...\n", test_duration.as_secs());
    println!("Time       Lines    Buffer   Memory     Delta      Peak     Cache");
    println!("---------- -------- ------- ---------- ---------- ---------- ------");
    
    while start.elapsed() < test_duration {
        // Poll all files
        for (i, watcher) in watchers.iter_mut().enumerate() {
            let new_lines = watcher.read_new_lines();
            let file_name = files[i].to_string_lossy();
            
            for line in new_lines {
                state.add_line(&file_name, line);
            }
        }
        
        // Sleep briefly
        std::thread::sleep(Duration::from_millis(50));
    }
    
    // Final report
    state.final_report();
    
    // Cleanup
    println!("\nCleaning up...");
    drop(state);
    drop(watchers);
    
    std::thread::sleep(Duration::from_millis(100));
    println!("Memory after cleanup: {:.1}MB", MemoryTracker::current_memory_mb());
}