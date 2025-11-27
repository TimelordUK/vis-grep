use std::io::Write;
use std::path::PathBuf;
use std::thread;
use std::time::{Duration, Instant};
use vis_grep::file_watch_manager::{FileWatchManager, FileSubscriber, FileUpdate};

extern crate chrono;

fn format_memory_size(bytes: usize) -> String {
    if bytes < 1024 {
        format!("{} B", bytes)
    } else if bytes < 1024 * 1024 {
        format!("{:.2} KB", bytes as f64 / 1024.0)
    } else {
        format!("{:.2} MB", bytes as f64 / 1024.0 / 1024.0)
    }
}

fn get_memory_usage() -> usize {
    #[cfg(target_os = "linux")]
    {
        if let Ok(status) = std::fs::read_to_string("/proc/self/status") {
            for line in status.lines() {
                if line.starts_with("VmRSS:") {
                    let parts: Vec<&str> = line.split_whitespace().collect();
                    if parts.len() >= 2 {
                        if let Ok(kb) = parts[1].parse::<usize>() {
                            return kb * 1024; // Convert KB to bytes
                        }
                    }
                }
            }
        }
    }
    0
}

fn main() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info"))
        .format_timestamp(None)
        .init();

    println!("FileWatchManager Memory Test");
    println!("=============================");
    
    // Create test files
    let test_dir = PathBuf::from("test_memory_logs");
    std::fs::create_dir_all(&test_dir).unwrap();
    
    let file_count = 10;
    let mut test_files = Vec::new();
    
    for i in 0..file_count {
        let file_path = test_dir.join(format!("test_{}.log", i));
        let mut file = std::fs::File::create(&file_path).unwrap();
        writeln!(file, "Initial content for file {}", i).unwrap();
        test_files.push(file_path);
    }
    
    // Create FileWatchManager
    let mut manager = FileWatchManager::new();
    let start_memory = get_memory_usage();
    let start_time = Instant::now();
    
    println!("Initial memory: {}", format_memory_size(start_memory));
    
    // Subscribe to all files
    for (i, path) in test_files.iter().enumerate() {
        manager.subscribe(path.clone(), FileSubscriber::Tail { mode_id: i }).unwrap();
    }
    
    println!("Subscribed to {} files", file_count);
    
    // Spawn writer thread that continuously writes to files
    let test_files_clone = test_files.clone();
    thread::spawn(move || {
        let mut line_count = 0;
        loop {
            for path in &test_files_clone {
                let mut file = std::fs::OpenOptions::new()
                    .append(true)
                    .open(path)
                    .unwrap();
                    
                // Write lines with varying content to simulate realistic logs
                for _ in 0..10 {
                    writeln!(file, "[{}] This is a test log line with some content that might be typical in a real log file. Line number: {}", 
                        chrono::Local::now().format("%Y-%m-%d %H:%M:%S%.3f"),
                        line_count
                    ).unwrap();
                    line_count += 1;
                }
            }
            thread::sleep(Duration::from_millis(100));
        }
    });
    
    // Monitor memory usage
    let mut last_memory = start_memory;
    let mut update_count = 0;
    let mut total_lines_received = 0;
    
    println!("\nMonitoring memory usage... Press Ctrl+C to stop\n");
    println!("{:>10} | {:>12} | {:>12} | {:>10} | {:>15} | {:>10}", 
        "Time (s)", "Memory", "Delta", "Updates", "Total Lines", "Lines/MB");
    println!("{:-<80}", "");
    
    loop {
        // Check for updates
        let updates = manager.check_for_updates();
        
        for (_path, (update, _subs)) in updates {
            update_count += 1;
            if let FileUpdate::NewLines { lines, .. } = update {
                total_lines_received += lines.len();
            }
        }
        
        let current_memory = get_memory_usage();
        let elapsed = start_time.elapsed().as_secs();
        let memory_delta = if current_memory > last_memory {
            format!("+{}", format_memory_size(current_memory - last_memory))
        } else if current_memory < last_memory {
            format!("-{}", format_memory_size(last_memory - current_memory))
        } else {
            "0 B".to_string()
        };
        
        let memory_growth = current_memory.saturating_sub(start_memory);
        let lines_per_mb = if memory_growth > 1024 * 1024 {
            total_lines_received as f64 / (memory_growth as f64 / 1024.0 / 1024.0)
        } else {
            0.0
        };
        
        println!("{:>10} | {:>12} | {:>12} | {:>10} | {:>15} | {:>10.1}", 
            elapsed,
            format_memory_size(current_memory),
            memory_delta,
            update_count,
            total_lines_received,
            lines_per_mb
        );
        
        last_memory = current_memory;
        
        // Sleep before next check
        thread::sleep(Duration::from_secs(1));
    }
}