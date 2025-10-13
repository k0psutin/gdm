use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

pub struct Cache {
    cache: Mutex<HashMap<String, String>>,
}

impl Cache {
    #[cfg(not(tarpaulin_include))]
    pub fn new<'a>() -> &'a Cache {
        let cache: Mutex<HashMap<String, String>> = {
            let mut _cache = HashMap::new();
            Mutex::new(_cache)
        };
        static INSTANCE: OnceLock<Cache> = OnceLock::new();
        INSTANCE.get_or_init(|| Cache { cache })
    }

    pub fn has_key(&self, key: &str) -> bool {
        self.cache.lock().unwrap().contains_key(key)
    }

    pub fn get(&self, key: &str) -> Option<String> {
        self.cache.lock().unwrap().get(key).cloned()
    }

    pub fn insert(&self, key: String, value: String) {
        self.cache.lock().unwrap().insert(key, value);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cache_insert_and_get() {
        let cache = Cache::new();
        cache.insert("key1".to_string(), "value1".to_string());
        assert_eq!(cache.get("key1"), Some("value1".to_string()));
        assert!(cache.has_key("key1"));
        assert!(!cache.has_key("key2"));
    }

    #[test]
    fn test_cache_get_should_return_none_for_missing_key() {
        let cache = Cache::new();
        assert_eq!(cache.get("key1"), None);
    }

    #[test]
    fn test_cache_insert_overwrites_existing_key() {
        let cache = Cache::new();
        cache.insert("key1".to_string(), "value1".to_string());
        cache.insert("key1".to_string(), "value2".to_string());
        assert_eq!(cache.get("key1"), Some("value2".to_string()));
    }
}