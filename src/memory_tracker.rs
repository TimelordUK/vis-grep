use std::time::Instant;
use std::collections::VecDeque;
use log::info;

#[derive(Debug, Clone)]
pub struct MemorySnapshot {
    pub timestamp: Instant,
    pub rust_allocated: usize,
    pub os_memory: Option<u64>, // RSS in bytes
    pub output_buffer_len: usize,
    pub output_buffer_capacity: usize,
    pub file_count: usize,
    pub active_files: usize,
    pub frame_count: u64,
    pub egui_textures: usize,
    pub egui_fonts_pixels: usize,
}

pub struct MemoryTracker {
    snapshots: VecDeque<MemorySnapshot>,
    max_snapshots: usize,
    last_report_time: Instant,
    report_interval_secs: u64,
    frame_count: u64,
}

impl MemoryTracker {
    pub fn new() -> Self {
        Self {
            snapshots: VecDeque::new(),
            max_snapshots: 300, // Keep 5 minutes at 1 snapshot per second
            last_report_time: Instant::now(),
            report_interval_secs: 10,
            frame_count: 0,
        }
    }

    pub fn add_snapshot(&mut self, snapshot: MemorySnapshot) {
        self.frame_count = snapshot.frame_count;
        
        if self.snapshots.len() >= self.max_snapshots {
            self.snapshots.pop_front();
        }
        
        self.snapshots.push_back(snapshot);
        
        // Report periodically
        if self.last_report_time.elapsed().as_secs() >= self.report_interval_secs {
            self.report_memory_stats();
            self.last_report_time = Instant::now();
        }
    }
    
    fn report_memory_stats(&self) {
        if self.snapshots.len() < 2 {
            return;
        }
        
        let latest = self.snapshots.back().unwrap();
        let oldest_recent = self.snapshots.get(self.snapshots.len().saturating_sub(60)).unwrap_or(&self.snapshots[0]);
        
        let rust_delta = latest.rust_allocated as i64 - oldest_recent.rust_allocated as i64;
        let os_delta = match (latest.os_memory, oldest_recent.os_memory) {
            (Some(new), Some(old)) => Some(new as i64 - old as i64),
            _ => None,
        };
        
        info!("📊 Memory Report (last 60s):");
        info!("  Rust Heap: {} MB (Δ {:+.2} MB)", 
              latest.rust_allocated / 1024 / 1024,
              rust_delta as f64 / 1024.0 / 1024.0);
        
        if let Some(os_mem) = latest.os_memory {
            info!("  OS Memory (RSS): {} MB (Δ {:+.2} MB)",
                  os_mem / 1024 / 1024,
                  os_delta.unwrap_or(0) as f64 / 1024.0 / 1024.0);
        }
        
        info!("  Output Buffer: {} lines (capacity: {})",
              latest.output_buffer_len,
              latest.output_buffer_capacity);
        
        info!("  Files: {} total, {} active",
              latest.file_count,
              latest.active_files);
              
        info!("  egui Textures: {}", latest.egui_textures);
        info!("  egui Font Pixels: {} KB", latest.egui_fonts_pixels / 1024);
        info!("  Frame #: {}", latest.frame_count);
        
        // Check for concerning growth
        if let Some(os_delta) = os_delta {
            if os_delta > 10 * 1024 * 1024 { // More than 10MB growth
                info!("  ⚠️  Significant OS memory growth detected!");
            }
        }
    }
    
    pub fn get_snapshots(&self) -> &VecDeque<MemorySnapshot> {
        &self.snapshots
    }
}

// Platform-specific function to get RSS
#[cfg(target_os = "linux")]
pub fn get_process_memory() -> Option<u64> {
    use std::fs;
    
    let status = fs::read_to_string("/proc/self/status").ok()?;
    for line in status.lines() {
        if line.starts_with("VmRSS:") {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 2 {
                if let Ok(kb) = parts[1].parse::<u64>() {
                    return Some(kb * 1024); // Convert KB to bytes
                }
            }
        }
    }
    None
}

#[cfg(not(target_os = "linux"))]
pub fn get_process_memory() -> Option<u64> {
    None // Not implemented for other platforms yet
}