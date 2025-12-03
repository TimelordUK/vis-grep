use std::time::{Duration, Instant};

/// Monitor idle time and trigger auto-shutdown after a configurable period
pub struct IdleMonitor {
    /// Last activity time (any user interaction)
    last_activity: Instant,
    
    /// Idle timeout duration (e.g., 30 minutes)
    idle_timeout: Duration,
    
    /// Whether idle monitoring is enabled
    enabled: bool,
    
    /// Grace period after startup before monitoring begins
    startup_grace_period: Duration,
    
    /// Application start time
    start_time: Instant,
}

impl IdleMonitor {
    pub fn new(idle_timeout_minutes: u64, enabled: bool) -> Self {
        Self {
            last_activity: Instant::now(),
            idle_timeout: Duration::from_secs(idle_timeout_minutes * 60),
            enabled,
            startup_grace_period: Duration::from_secs(300), // 5 minutes grace period
            start_time: Instant::now(),
        }
    }
    
    /// Record user activity
    pub fn record_activity(&mut self) {
        self.last_activity = Instant::now();
    }
    
    /// Check if we should auto-shutdown
    pub fn should_shutdown(&self) -> bool {
        if !self.enabled {
            return false;
        }
        
        // Don't shutdown during grace period (allows remote login)
        if self.start_time.elapsed() < self.startup_grace_period {
            return false;
        }
        
        self.last_activity.elapsed() > self.idle_timeout
    }
    
    /// Get time until shutdown (if applicable)
    pub fn time_until_shutdown(&self) -> Option<Duration> {
        if !self.enabled {
            return None;
        }
        
        if self.start_time.elapsed() < self.startup_grace_period {
            return None;
        }
        
        let elapsed = self.last_activity.elapsed();
        if elapsed < self.idle_timeout {
            Some(self.idle_timeout - elapsed)
        } else {
            Some(Duration::ZERO)
        }
    }
    
    /// Get a status string for UI display
    pub fn status_string(&self) -> String {
        if !self.enabled {
            return String::new();
        }
        
        if let Some(remaining) = self.time_until_shutdown() {
            if remaining == Duration::ZERO {
                "Auto-shutdown: Idle timeout reached".to_string()
            } else {
                let minutes = remaining.as_secs() / 60;
                let seconds = remaining.as_secs() % 60;
                if minutes > 0 {
                    format!("Auto-shutdown in: {}m {}s", minutes, seconds)
                } else {
                    format!("Auto-shutdown in: {}s", seconds)
                }
            }
        } else {
            "Auto-shutdown: In grace period".to_string()
        }
    }
}