use std::time::{Duration, Instant};
use eframe::egui;

/// Memory usage information
#[derive(Debug, Clone)]
pub struct MemoryInfo {
    /// Physical memory used by process in bytes
    pub physical_memory: u64,
    /// Virtual memory used by process in bytes
    pub virtual_memory: u64,
    /// System total memory in bytes
    pub system_total: u64,
    /// System available memory in bytes  
    pub system_available: u64,
}

impl MemoryInfo {
    /// Get physical memory in MB
    pub fn physical_mb(&self) -> f64 {
        self.physical_memory as f64 / (1024.0 * 1024.0)
    }
    
    /// Get virtual memory in MB
    pub fn virtual_mb(&self) -> f64 {
        self.virtual_memory as f64 / (1024.0 * 1024.0)
    }
    
    /// Get system memory usage percentage
    pub fn system_usage_percent(&self) -> f64 {
        let used = self.system_total.saturating_sub(self.system_available);
        (used as f64 / self.system_total as f64) * 100.0
    }
}

/// Tracks memory usage over time
pub struct MemoryMonitor {
    /// Last memory check time
    last_check: Instant,
    /// Check interval
    check_interval: Duration,
    /// Last memory info
    last_info: Option<MemoryInfo>,
    /// Memory at start
    start_memory: Option<u64>,
    /// Peak memory observed
    peak_memory: u64,
    /// Number of measurements
    measurement_count: u64,
    /// Total memory sum for averaging
    total_memory_sum: u64,
    
    // Line rendering statistics
    /// Total lines being rendered in UI
    pub lines_rendered: usize,
    /// Estimated visible lines in viewport
    pub visible_lines_estimate: usize,
    /// Total buffer size across all files
    pub total_buffer_lines: usize,
}

impl MemoryMonitor {
    /// Create a new memory monitor
    pub fn new() -> Self {
        Self {
            last_check: Instant::now(),
            check_interval: Duration::from_secs(5), // Check every 5 seconds
            last_info: None,
            start_memory: None,
            peak_memory: 0,
            measurement_count: 0,
            total_memory_sum: 0,
            lines_rendered: 0,
            visible_lines_estimate: 0,
            total_buffer_lines: 0,
        }
    }
    
    /// Set check interval
    pub fn set_check_interval(&mut self, interval: Duration) {
        self.check_interval = interval;
    }
    
    /// Check and update memory usage if needed
    pub fn check(&mut self) -> Option<&MemoryInfo> {
        if self.last_check.elapsed() < self.check_interval {
            return self.last_info.as_ref();
        }
        
        self.last_check = Instant::now();
        
        if let Some(info) = get_memory_info() {
            // Track statistics
            if self.start_memory.is_none() {
                self.start_memory = Some(info.physical_memory);
            }
            
            self.peak_memory = self.peak_memory.max(info.physical_memory);
            self.measurement_count += 1;
            self.total_memory_sum += info.physical_memory;
            
            // Log significant changes
            if let Some(last) = &self.last_info {
                let change_mb = (info.physical_memory as i64 - last.physical_memory as i64) as f64 / (1024.0 * 1024.0);
                
                // Log if memory changed by more than 10MB
                if change_mb.abs() > 10.0 {
                    log::info!(
                        "Memory: {:.1} MB ({:+.1} MB), Virtual: {:.1} MB, System: {:.1}%",
                        info.physical_mb(),
                        change_mb,
                        info.virtual_mb(),
                        info.system_usage_percent()
                    );
                }
            } else {
                // First measurement
                log::info!(
                    "Initial memory: {:.1} MB, Virtual: {:.1} MB, System: {:.1}%",
                    info.physical_mb(),
                    info.virtual_mb(),
                    info.system_usage_percent()
                );
            }
            
            self.last_info = Some(info);
        }
        
        self.last_info.as_ref()
    }
    
    /// Get current memory info (always fresh)
    pub fn get_current(&mut self) -> Option<&MemoryInfo> {
        self.last_check = Instant::now().checked_sub(self.check_interval).unwrap_or(Instant::now());
        self.check()
    }
    
    /// Get memory growth since start in MB
    pub fn growth_mb(&self) -> f64 {
        if let (Some(start), Some(current)) = (self.start_memory, &self.last_info) {
            (current.physical_memory as i64 - start as i64) as f64 / (1024.0 * 1024.0)
        } else {
            0.0
        }
    }
    
    /// Get peak memory in MB
    pub fn peak_mb(&self) -> f64 {
        self.peak_memory as f64 / (1024.0 * 1024.0)
    }
    
    /// Get average memory in MB
    pub fn average_mb(&self) -> f64 {
        if self.measurement_count > 0 {
            (self.total_memory_sum / self.measurement_count) as f64 / (1024.0 * 1024.0)
        } else {
            0.0
        }
    }
    
    /// Get rendering efficiency percentage
    pub fn rendering_efficiency(&self) -> f32 {
        if self.lines_rendered > 0 {
            (self.visible_lines_estimate as f32 / self.lines_rendered as f32) * 100.0
        } else {
            100.0
        }
    }
    
    /// Update line rendering statistics
    pub fn update_line_stats(&mut self, rendered: usize, visible: usize, total_buffer: usize) {
        self.lines_rendered = rendered;
        self.visible_lines_estimate = visible;
        self.total_buffer_lines = total_buffer;
    }
    
    /// Get formatted status string
    pub fn status_string(&self) -> String {
        if let Some(info) = &self.last_info {
            let efficiency = self.rendering_efficiency();
            let efficiency_str = if self.lines_rendered > 0 {
                format!(", Eff: {:.0}%", efficiency)
            } else {
                String::new()
            };
            
            format!(
                "Mem: {:.1}MB (△{:+.1}MB, Peak: {:.1}MB{})",
                info.physical_mb(),
                self.growth_mb(),
                self.peak_mb(),
                efficiency_str
            )
        } else {
            "Mem: --".to_string()
        }
    }
    
    /// Create debug overlay for egui
    pub fn render_debug_overlay(&self, ui: &mut egui::Ui) {
        egui::Area::new("memory_debug_overlay".into())
            .anchor(egui::Align2::RIGHT_TOP, [-10.0, 40.0])
            .show(ui.ctx(), |ui| {
                egui::Frame::popup(ui.style())
                    .fill(egui::Color32::from_rgba_premultiplied(0, 0, 0, 220))
                    .show(ui, |ui| {
                        ui.set_width(250.0);
                        
                        ui.colored_label(egui::Color32::YELLOW, "🔍 Memory & Rendering Stats");
                        ui.separator();
                        
                        if let Some(info) = &self.last_info {
                            ui.label(format!("Memory: {:.1} MB", info.physical_mb()));
                            ui.label(format!("Peak: {:.1} MB", self.peak_mb()));
                            ui.label(format!("Growth: {:+.1} MB", self.growth_mb()));
                        }
                        
                        ui.separator();
                        
                        ui.label(format!("Buffer lines: {}", self.total_buffer_lines));
                        ui.label(format!("Lines rendered: {}", self.lines_rendered));
                        ui.label(format!("Visible lines: ~{}", self.visible_lines_estimate));
                        
                        let efficiency = self.rendering_efficiency();
                        let color = if efficiency < 10.0 {
                            egui::Color32::RED
                        } else if efficiency < 50.0 {
                            egui::Color32::YELLOW
                        } else {
                            egui::Color32::GREEN
                        };
                        
                        ui.colored_label(color, format!("Efficiency: {:.1}%", efficiency));
                        
                        if self.lines_rendered > 1000 && efficiency < 10.0 {
                            ui.separator();
                            ui.colored_label(
                                egui::Color32::RED, 
                                "⚠️ Rendering many invisible lines!"
                            );
                            ui.label("Consider virtualization!");
                        }
                    });
            });
    }
    
    /// Get current allocated memory in bytes
    pub fn current_allocated(&self) -> usize {
        // For now, return physical memory as a proxy for Rust allocations
        // This isn't perfect but gives us something to track
        self.last_info
            .as_ref()
            .map(|info| info.physical_memory as usize)
            .unwrap_or(0)
    }
}

impl Default for MemoryMonitor {
    fn default() -> Self {
        Self::new()
    }
}

// Platform-specific memory info gathering
#[cfg(target_os = "linux")]
fn get_memory_info() -> Option<MemoryInfo> {
    use std::fs;
    
    // Read process memory from /proc/self/status
    let status = fs::read_to_string("/proc/self/status").ok()?;
    let mut vm_rss = None;
    let mut vm_size = None;
    
    for line in status.lines() {
        if let Some(rss) = line.strip_prefix("VmRSS:") {
            vm_rss = rss.trim().split_whitespace().next()
                .and_then(|s| s.parse::<u64>().ok())
                .map(|kb| kb * 1024); // Convert KB to bytes
        } else if let Some(size) = line.strip_prefix("VmSize:") {
            vm_size = size.trim().split_whitespace().next()
                .and_then(|s| s.parse::<u64>().ok())
                .map(|kb| kb * 1024); // Convert KB to bytes
        }
    }
    
    // Read system memory from /proc/meminfo
    let meminfo = fs::read_to_string("/proc/meminfo").ok()?;
    let mut mem_total = None;
    let mut mem_available = None;
    
    for line in meminfo.lines() {
        if let Some(total) = line.strip_prefix("MemTotal:") {
            mem_total = total.trim().split_whitespace().next()
                .and_then(|s| s.parse::<u64>().ok())
                .map(|kb| kb * 1024); // Convert KB to bytes
        } else if let Some(available) = line.strip_prefix("MemAvailable:") {
            mem_available = available.trim().split_whitespace().next()
                .and_then(|s| s.parse::<u64>().ok())
                .map(|kb| kb * 1024); // Convert KB to bytes
        }
    }
    
    Some(MemoryInfo {
        physical_memory: vm_rss.unwrap_or(0),
        virtual_memory: vm_size.unwrap_or(0),
        system_total: mem_total.unwrap_or(0),
        system_available: mem_available.unwrap_or(0),
    })
}

#[cfg(target_os = "windows")]
fn get_memory_info() -> Option<MemoryInfo> {
    use std::mem;
    use winapi::shared::minwindef::{DWORD, FALSE};
    use winapi::um::processthreadsapi::GetCurrentProcess;
    use winapi::um::psapi::{GetProcessMemoryInfo, PROCESS_MEMORY_COUNTERS};
    use winapi::um::sysinfoapi::{GetSystemInfo, GlobalMemoryStatusEx, MEMORYSTATUSEX, SYSTEM_INFO};
    
    unsafe {
        // Get process memory info
        let mut pmc: PROCESS_MEMORY_COUNTERS = mem::zeroed();
        pmc.cb = mem::size_of::<PROCESS_MEMORY_COUNTERS>() as DWORD;
        
        if GetProcessMemoryInfo(
            GetCurrentProcess(),
            &mut pmc as *mut _ as *mut _,
            pmc.cb
        ) == FALSE {
            return None;
        }
        
        // Get system memory info
        let mut mem_status: MEMORYSTATUSEX = mem::zeroed();
        mem_status.dwLength = mem::size_of::<MEMORYSTATUSEX>() as DWORD;
        
        if GlobalMemoryStatusEx(&mut mem_status) == FALSE {
            return None;
        }
        
        Some(MemoryInfo {
            physical_memory: pmc.WorkingSetSize as u64,
            virtual_memory: pmc.PagefileUsage as u64,
            system_total: mem_status.ullTotalPhys,
            system_available: mem_status.ullAvailPhys,
        })
    }
}

#[cfg(target_os = "macos")]
fn get_memory_info() -> Option<MemoryInfo> {
    use std::mem;
    use libc::{c_int, c_void, size_t, sysctl, CTL_HW, CTL_VM, HW_MEMSIZE, HW_PHYSMEM, VM_SWAPUSAGE};
    use mach::{
        kern_return::KERN_SUCCESS,
        mach_types::task_name_t,
        message::mach_msg_type_number_t,
        port::mach_port_t,
        task::{task_basic_info, task_info, TASK_BASIC_INFO},
        task_info::task_info_t,
        traps::mach_task_self,
    };
    
    unsafe {
        // Get process memory info
        let mut info: task_basic_info = mem::zeroed();
        let mut info_count = (mem::size_of::<task_basic_info>() / mem::size_of::<c_int>()) as mach_msg_type_number_t;
        
        let kr = task_info(
            mach_task_self() as task_name_t,
            TASK_BASIC_INFO,
            &mut info as *mut _ as task_info_t,
            &mut info_count
        );
        
        if kr != KERN_SUCCESS {
            return None;
        }
        
        // Get system memory size
        let mut mem_size: u64 = 0;
        let mut mem_size_len = mem::size_of::<u64>();
        let mut mib = [CTL_HW, HW_MEMSIZE];
        
        if sysctl(
            mib.as_mut_ptr(),
            2,
            &mut mem_size as *mut _ as *mut c_void,
            &mut mem_size_len as *mut _,
            std::ptr::null_mut(),
            0
        ) != 0 {
            return None;
        }
        
        // Note: Getting available memory on macOS is complex and requires additional APIs
        // For now, we'll use a simple approximation
        Some(MemoryInfo {
            physical_memory: info.resident_size as u64,
            virtual_memory: info.virtual_size as u64,
            system_total: mem_size,
            system_available: mem_size / 2, // Rough approximation
        })
    }
}

#[cfg(not(any(target_os = "linux", target_os = "windows", target_os = "macos")))]
fn get_memory_info() -> Option<MemoryInfo> {
    // Fallback for other platforms
    None
}