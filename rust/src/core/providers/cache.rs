use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};

static PROVIDER_CACHE: std::sync::LazyLock<Mutex<ProviderCache>> =
    std::sync::LazyLock::new(|| Mutex::new(ProviderCache::new()));

struct CacheEntry {
    data: String,
    expires_at: Instant,
}

struct ProviderCache {
    entries: HashMap<String, CacheEntry>,
}

impl ProviderCache {
    fn new() -> Self {
        Self {
            entries: HashMap::new(),
        }
    }

    fn get(&mut self, key: &str) -> Option<&str> {
        self.entries.retain(|_, v| v.expires_at > Instant::now());
        self.entries.get(key).map(|e| e.data.as_str())
    }

    fn set(&mut self, key: String, data: String, ttl: Duration) {
        self.entries.insert(
            key,
            CacheEntry {
                data,
                expires_at: Instant::now() + ttl,
            },
        );
    }
}

pub fn get_cached(key: &str) -> Option<String> {
    PROVIDER_CACHE
        .lock()
        .ok()
        .and_then(|mut c| c.get(key).map(std::string::ToString::to_string))
}

pub fn set_cached(key: &str, data: &str, ttl_secs: u64) {
    if let Ok(mut cache) = PROVIDER_CACHE.lock() {
        cache.set(
            key.to_string(),
            data.to_string(),
            Duration::from_secs(ttl_secs),
        );
    }
}
