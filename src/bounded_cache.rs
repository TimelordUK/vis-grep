use std::collections::HashMap;
use std::hash::Hash;
use std::sync::Mutex;

/// A bounded LRU-style cache with a maximum size
pub struct BoundedCache<K: Hash + Eq + Clone, V: Clone> {
    cache: HashMap<K, (V, usize)>, // Value and access count
    max_size: usize,
    access_counter: usize,
}

impl<K: Hash + Eq + Clone, V: Clone> BoundedCache<K, V> {
    pub fn new(max_size: usize) -> Self {
        Self {
            cache: HashMap::with_capacity(max_size),
            max_size,
            access_counter: 0,
        }
    }

    pub fn get(&mut self, key: &K) -> Option<V> {
        self.access_counter += 1;
        
        if let Some((value, access_count)) = self.cache.get_mut(key) {
            *access_count = self.access_counter;
            Some(value.clone())
        } else {
            None
        }
    }

    pub fn insert(&mut self, key: K, value: V) {
        // Check if we need to evict
        if self.cache.len() >= self.max_size && !self.cache.contains_key(&key) {
            // Find least recently used entry
            if let Some((lru_key, _)) = self.cache
                .iter()
                .min_by_key(|(_, (_, access_count))| access_count)
                .map(|(k, v)| (k.clone(), v.clone()))
            {
                self.cache.remove(&lru_key);
            }
        }

        self.access_counter += 1;
        self.cache.insert(key, (value, self.access_counter));
    }

    pub fn get_or_insert_with<F>(&mut self, key: K, f: F) -> V
    where
        F: FnOnce() -> V,
    {
        if let Some(value) = self.get(&key) {
            value
        } else {
            let value = f();
            self.insert(key.clone(), value.clone());
            value
        }
    }
}

/// Thread-safe bounded cache
pub struct ThreadSafeBoundedCache<K: Hash + Eq + Clone, V: Clone> {
    inner: Mutex<BoundedCache<K, V>>,
}

impl<K: Hash + Eq + Clone, V: Clone> ThreadSafeBoundedCache<K, V> {
    pub fn new(max_size: usize) -> Self {
        Self {
            inner: Mutex::new(BoundedCache::new(max_size)),
        }
    }

    pub fn get_or_insert_with<F>(&self, key: K, f: F) -> V
    where
        F: FnOnce() -> V,
    {
        self.inner.lock().unwrap().get_or_insert_with(key, f)
    }
}

// Convenience type aliases for common string caches
pub type StringCache = ThreadSafeBoundedCache<String, String>;
pub type TupleStringCache<K> = ThreadSafeBoundedCache<K, String>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bounded_cache_eviction() {
        let mut cache: BoundedCache<i32, String> = BoundedCache::new(3);
        
        // Fill cache
        cache.insert(1, "one".to_string());
        cache.insert(2, "two".to_string());
        cache.insert(3, "three".to_string());
        
        // Access 1 and 2 to make them more recent
        cache.get(&1);
        cache.get(&2);
        
        // Insert new item - should evict 3
        cache.insert(4, "four".to_string());
        
        assert!(cache.get(&1).is_some());
        assert!(cache.get(&2).is_some());
        assert!(cache.get(&3).is_none()); // Should be evicted
        assert!(cache.get(&4).is_some());
    }
}