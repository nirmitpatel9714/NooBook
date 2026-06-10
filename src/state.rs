use serde_json::{Map, Value};
use std::sync::{Arc, Mutex};

/// Thread-safe shared variable store.
///
/// Wraps [`Arc<Mutex<Map>>`] so it can be cheaply cloned and safely shared
/// across all panes, workspaces, and async tasks. Variables set in one REPL
/// are available for injection into another.
#[derive(Clone)]
pub struct SharedState {
    pub store: Arc<Mutex<Map<String, Value>>>,
}

impl SharedState {
    /// Create a new empty state store.
    pub fn new() -> Self {
        Self {
            store: Arc::new(Mutex::new(Map::new())),
        }
    }

    /// Insert or overwrite a variable by key.
    pub fn set(&self, key: &str, value: Value) {
        let mut store = self.store.lock().unwrap();
        store.insert(key.to_string(), value);
    }

    /// Look up a variable by key.
    pub fn get(&self, key: &str) -> Option<Value> {
        let store = self.store.lock().unwrap();
        store.get(key).cloned()
    }

    /// Serialize the entire store as a JSON string.
    pub fn as_json_string(&self) -> String {
        let store = self.store.lock().unwrap();
        serde_json::to_string(&*store).unwrap_or_else(|_| "{}".to_string())
    }

    /// Merge the contents of a JSON object string into the store.
    pub fn import_json(&self, json_str: &str) {
        if let Ok(parsed) = serde_json::from_str::<Map<String, Value>>(json_str) {
            let mut store = self.store.lock().unwrap();
            for (k, v) in parsed {
                store.insert(k, v);
            }
        }
    }

    /// Remove a variable by key.
    pub fn remove(&self, key: &str) {
        let mut store = self.store.lock().unwrap();
        store.remove(key);
    }

    /// Check whether the store is empty.
    pub fn is_empty(&self) -> bool {
        let store = self.store.lock().unwrap();
        store.is_empty()
    }
}
