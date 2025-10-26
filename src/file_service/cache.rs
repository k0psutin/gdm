use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

/// A simple key-value in-memory cache implementation
pub struct DefaultCache {
    pub cache: Mutex<HashMap<String, String>>,
}

impl DefaultCache {
    pub fn new<'a>() -> &'a DefaultCache {
        let cache: Mutex<HashMap<String, String>> = {
            let mut _cache = HashMap::new();
            Mutex::new(_cache)
        };
        static INSTANCE: OnceLock<DefaultCache> = OnceLock::new();
        INSTANCE.get_or_init(|| DefaultCache { cache })
    }
}

#[cfg_attr(test, mockall::automock)]
impl Cache for DefaultCache {
    fn has_key(&self, key: &str) -> bool {
        self.cache.lock().unwrap().contains_key(key)
    }

    fn get(&self, key: &str) -> Option<String> {
        self.cache.lock().unwrap().get(key).cloned()
    }

    fn insert(&self, key: &str, value: &str) {
        self.cache
            .lock()
            .unwrap()
            .insert(key.to_string(), value.to_string());
    }

    #[cfg(test)]
    fn clear(&self) {
        self.cache.lock().unwrap().clear();
    }
}

pub trait Cache {
    fn has_key(&self, key: &str) -> bool;
    fn get(&self, key: &str) -> Option<String>;
    fn insert(&self, key: &str, value: &str);
    #[cfg(test)]
    #[allow(dead_code)]
    fn clear(&self);
}


#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;

    #[test]
    #[serial]
    fn test_cache_insert_and_get() {
        let cache = DefaultCache::new();
        cache.clear(); // Clear the singleton cache before test
        cache.insert("key1", "value1");
        assert_eq!(cache.get("key1"), Some("value1".to_string()));
        assert!(cache.has_key("key1"));
        assert!(!cache.has_key("key2"));
    }

    #[test]
    #[serial]
    fn test_cache_get_should_return_none_for_missing_key() {
        let cache = DefaultCache::new();
        cache.clear(); // Clear the singleton cache before test
        assert_eq!(cache.get("key1"), None);
    }

    #[test]
    #[serial]
    fn test_cache_insert_overwrites_existing_key() {
        let cache = DefaultCache::new();
        cache.clear(); // Clear the singleton cache before test
        cache.insert("key1", "value1");
        cache.insert("key1", "value2");
        assert_eq!(cache.get("key1"), Some("value2".to_string()));
    }
}
