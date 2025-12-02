use std::collections::VecDeque;
use std::fs::File;
use std::io::{BufRead, BufReader, Seek, SeekFrom};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};

// We'll implement a minimal file watching system for headless mode
// to avoid GUI dependencies

// Simple LogLine struct for testing
struct LogLine {
    file_name: Arc<str>,
    content: Arc<str>,
    line_number: usize,
    timestamp: f64,
    level: Option<LogLevel>,
}

// Minimal LogLevel enum for headless testing
#[derive(Debug, Clone, Copy, PartialEq)]
enum LogLevel {
    Error,
    Warn,
    Info,
    Debug,
    Trace,
    Unknown,
}

// Simple log level detector
struct LogLevelDetector;

impl LogLevelDetector {
    fn new() -> Self {
        Self
    }
    
    fn detect(&self, line: &str) -> Option<LogLevel> {
        let line_upper = line.to_uppercase();
        if line_upper.contains("ERROR") || line_upper.contains("ERR") {
            Some(LogLevel::Error)
        } else if line_upper.contains("WARN") || line_upper.contains("WARNING") {
            Some(LogLevel::Warn)
        } else if line_upper.contains("INFO") {
            Some(LogLevel::Info)
        } else if line_upper.contains("DEBUG") {
            Some(LogLevel::Debug)
        } else if line_upper.contains("TRACE") {
            Some(LogLevel::Trace)
        } else {
            Some(LogLevel::Unknown)
        }
    }
}

// Simple string cache for headless testing
mod string_cache {
    use std::collections::HashMap;
    use std::sync::{Arc, Mutex};
    use once_cell::sync::Lazy;
    
    static CACHE: Lazy<Mutex<HashMap<String, Arc<str>>>> = Lazy::new(|| {
        Mutex::new(HashMap::new())
    });
    
    pub fn intern_string(s: &str) -> Arc<str> {
        let mut cache = CACHE.lock().unwrap();
        if let Some(cached) = cache.get(s) {
            Arc::clone(cached)
        } else {
            let arc = Arc::from(s);
            cache.insert(s.to_string(), Arc::clone(&arc));
            arc
        }
    }
    
    pub fn cache_size() -> usize {
        CACHE.lock().unwrap().len()
    }
    
    pub fn clear_cache() {
        CACHE.lock().unwrap().clear();
    }
}

// Minimal file watcher for headless testing
struct FileWatcher {
    path: PathBuf,
    last_position: u64,
    last_size: u64,
}

impl FileWatcher {
    fn new(path: PathBuf) -> std::io::Result<Self> {
        let metadata = std::fs::metadata(&path)?;
        Ok(Self {
            path,
            last_position: metadata.len(),
            last_size: metadata.len(),
        })
    }
    
    fn poll(&mut self) -> std::io::Result<Vec<String>> {
        let metadata = std::fs::metadata(&self.path)?;
        let current_size = metadata.len();
        
        if current_size < self.last_size {
            // File was truncated
            self.last_position = 0;
            self.last_size = current_size;
        }
        
        if current_size == self.last_position {
            // No new data
            return Ok(Vec::new());
        }
        
        // Read new lines
        let mut file = File::open(&self.path)?;
        file.seek(SeekFrom::Start(self.last_position))?;
        
        let reader = BufReader::new(file);
        let mut lines = Vec::new();
        
        for line in reader.lines() {
            lines.push(line?);
        }
        
        self.last_position = current_size;
        self.last_size = current_size;
        
        Ok(lines)
    }
}

struct AllocationTracker;

impl AllocationTracker {
    fn current_memory_mb() -> f64 {
        // Try to get actual RSS memory usage
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

struct HeadlessTailState {
    output_buffer: VecDeque<LogLine>,
    max_buffer_lines: usize,
    lines_processed: usize,
    start_time: Instant,
    last_memory_report: Instant,
    peak_memory: f64,
    initial_memory: f64,
}

impl HeadlessTailState {
    fn new(max_buffer_lines: usize) -> Self {
        let initial_memory = AllocationTracker::current_memory_mb();
        Self {
            output_buffer: VecDeque::new(),
            max_buffer_lines,
            lines_processed: 0,
            start_time: Instant::now(),
            last_memory_report: Instant::now(),
            peak_memory: initial_memory,
            initial_memory,
        }
    }

    fn add_line(&mut self, line: LogLine) {
        self.output_buffer.push_back(line);
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
            let current_memory = AllocationTracker::current_memory_mb();
            self.peak_memory = self.peak_memory.max(current_memory);
            
            println!(
                "[{:6.1}s] Lines: {:8}, Buffer: {:6}, Memory: {:7.1}MB (delta: {:+7.1}MB, peak: {:7.1}MB), Cache: {:5}",
                self.start_time.elapsed().as_secs_f64(),
                self.lines_processed,
                self.output_buffer.len(),
                current_memory,
                current_memory - self.initial_memory,
                self.peak_memory,
                string_cache::cache_size()
            );
            
            self.last_memory_report = Instant::now();
        }
    }
}

fn main() {
    env_logger::init();
    
    println!("=== Headless Tail Mode Memory Test ===");
    let initial_memory = AllocationTracker::current_memory_mb();
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
    let mut watchers = Vec::new();
    for file_path in &files {
        println!("Watching file: {}", file_path.display());
        match FileWatcher::new(file_path.clone()) {
            Ok(watcher) => watchers.push((file_path.clone(), watcher)),
            Err(e) => {
                eprintln!("Error opening {}: {}", file_path.display(), e);
                std::process::exit(1);
            }
        }
    }
    
    // Create headless state
    let mut state = HeadlessTailState::new(max_buffer_lines);
    let log_detector = LogLevelDetector::new();
    
    // Main loop
    let test_duration = Duration::from_secs(test_duration_secs);
    let start = Instant::now();
    
    println!("\nStarting test for {} seconds...\n", test_duration.as_secs());
    println!("Time       Lines    Buffer   Memory     Delta      Peak     Cache");
    println!("---------- -------- ------- ---------- ---------- ---------- ------");
    
    while start.elapsed() < test_duration {
        // Poll all watchers
        for (path, watcher) in &mut watchers {
            match watcher.poll() {
                Ok(lines) => {
                    let file_name = path.file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or("unknown");
                    let file_name_arc = string_cache::intern_string(file_name);
                    
                    for line_content in lines {
                        // Intern the string to simulate real usage
                        let interned_content = string_cache::intern_string(&line_content);
                        
                        // Detect log level
                        let level = log_detector.detect(&line_content);
                        
                        let log_line = LogLine {
                            file_name: Arc::clone(&file_name_arc),
                            content: interned_content,
                            line_number: state.lines_processed,
                            timestamp: std::time::SystemTime::now()
                                .duration_since(std::time::UNIX_EPOCH)
                                .unwrap()
                                .as_secs_f64(),
                            level,
                        };
                        state.add_line(log_line);
                    }
                }
                Err(e) => {
                    eprintln!("Error reading {}: {}", path.display(), e);
                }
            }
        }
        
        // Sleep briefly to avoid busy-waiting
        std::thread::sleep(Duration::from_millis(10));
    }
    
    // Final report
    println!("\n=== Test Complete ===");
    println!("Total lines processed: {}", state.lines_processed);
    println!("Final buffer size: {}", state.output_buffer.len());
    println!("Initial memory: {:.1}MB", state.initial_memory);
    println!("Final memory: {:.1}MB", AllocationTracker::current_memory_mb());
    println!("Peak memory: {:.1}MB", state.peak_memory);
    println!("Memory growth: {:.1}MB", state.peak_memory - state.initial_memory);
    println!("String cache entries: {}", string_cache::cache_size());
    println!("Average memory per line: {:.3}KB", 
        (state.peak_memory - state.initial_memory) * 1024.0 / state.lines_processed as f64);
    
    // Force cleanup
    println!("\nCleaning up...");
    drop(state);
    drop(watchers);
    string_cache::clear_cache();
    
    std::thread::sleep(Duration::from_millis(100));
    println!("Memory after cleanup: {:.1}MB", AllocationTracker::current_memory_mb());
}