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
}

#[cfg_attr(test, mockall::automock)]
impl CacheImpl for Cache {
    fn get_cache(&self) -> &Mutex<HashMap<String, String>> {
        &self.cache
    }
}

pub trait CacheImpl {
    fn get_cache(&self) -> &Mutex<HashMap<String, String>>;

    fn has_key(&self, key: &str) -> bool {
        self.get_cache().lock().unwrap().contains_key(key)
    }

    fn get(&self, key: &str) -> Option<String> {
        self.get_cache().lock().unwrap().get(key).cloned()
    }

    fn insert(&self, key: &str, value: &str) {
        self.get_cache()
            .lock()
            .unwrap()
            .insert(key.to_string(), value.to_string());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cache_insert_and_get() {
        let cache = Cache::new();
        cache.insert("key1", "value1");
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
        cache.insert("key1", "value1");
        cache.insert("key1", "value2");
        assert_eq!(cache.get("key1"), Some("value2".to_string()));
    }
}
