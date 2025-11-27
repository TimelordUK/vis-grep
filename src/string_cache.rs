use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use once_cell::sync::Lazy;

/// Global string cache for interning commonly used strings
/// This significantly reduces memory usage by sharing identical strings
pub struct StringCache {
    cache: HashMap<String, Arc<str>>,
    /// Track cache statistics
    hits: usize,
    misses: usize,
    /// Maximum cache size to prevent unbounded growth
    max_size: usize,
}

impl StringCache {
    pub fn new(max_size: usize) -> Self {
        Self {
            cache: HashMap::with_capacity(max_size / 10), // Pre-allocate 10% of max size
            hits: 0,
            misses: 0,
            max_size,
        }
    }

    /// Get or insert a string into the cache
    /// Returns an Arc<str> that can be cheaply cloned
    pub fn intern(&mut self, s: &str) -> Arc<str> {
        if let Some(cached) = self.cache.get(s) {
            self.hits += 1;
            Arc::clone(cached)
        } else {
            self.misses += 1;
            
            // If cache is at capacity, remove oldest entries (simple FIFO)
            if self.cache.len() >= self.max_size {
                // Remove ~10% of entries
                let to_remove = self.max_size / 10;
                let keys_to_remove: Vec<_> = self.cache.keys()
                    .take(to_remove)
                    .cloned()
                    .collect();
                for key in keys_to_remove {
                    self.cache.remove(&key);
                }
            }
            
            let arc_str: Arc<str> = Arc::from(s);
            self.cache.insert(s.to_string(), Arc::clone(&arc_str));
            arc_str
        }
    }

    /// Get cache statistics
    pub fn stats(&self) -> (usize, usize, usize) {
        (self.cache.len(), self.hits, self.misses)
    }
    
    /// Clear the cache
    pub fn clear(&mut self) {
        self.cache.clear();
        self.hits = 0;
        self.misses = 0;
    }
}

/// Global string cache instance
static STRING_CACHE: Lazy<Mutex<StringCache>> = Lazy::new(|| {
    Mutex::new(StringCache::new(10_000)) // Cache up to 10k unique strings
});

/// Intern a string into the global cache
pub fn intern_string(s: &str) -> Arc<str> {
    STRING_CACHE.lock().unwrap().intern(s)
}

/// Get global cache statistics
pub fn cache_stats() -> (usize, usize, usize) {
    STRING_CACHE.lock().unwrap().stats()
}

/// Clear the global cache
pub fn clear_cache() {
    STRING_CACHE.lock().unwrap().clear()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_string_interning() {
        let mut cache = StringCache::new(10);
        
        let s1 = cache.intern("hello");
        let s2 = cache.intern("hello");
        
        // Should return the same Arc
        assert!(Arc::ptr_eq(&s1, &s2));
        
        let (size, hits, misses) = cache.stats();
        assert_eq!(size, 1);
        assert_eq!(hits, 1);
        assert_eq!(misses, 1);
    }
    
    #[test]
    fn test_cache_eviction() {
        let mut cache = StringCache::new(5);
        
        // Fill cache beyond capacity
        for i in 0..10 {
            cache.intern(&format!("string_{}", i));
        }
        
        let (size, _, _) = cache.stats();
        assert!(size <= 5);
    }
}