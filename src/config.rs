use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::Path;

/// Configuration for a single language REPL.
///
/// Deserialized from `languages.json`.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct LanguageConfig {
    /// The executable to run (e.g., `"python"`, `"node"`).
    pub cmd: String,
    /// Arguments passed to the executable (e.g., `["-i"]`).
    pub args: Vec<String>,
    /// Execution mode. Currently always `"repl"`.
    pub mode: String,
}

/// Map of language alias (e.g., `"py"`, `"js"`) to [`LanguageConfig`].
pub type ConfigMap = HashMap<String, LanguageConfig>;

/// Load language configuration from a JSON file on disk.
pub fn load_config<P: AsRef<Path>>(path: P) -> std::io::Result<ConfigMap> {
    let content = fs::read_to_string(path)?;
    let config: ConfigMap = serde_json::from_str(&content)?;
    Ok(config)
}

/// Load language configuration from a raw JSON string.
pub fn load_from_str(content: &str) -> std::io::Result<ConfigMap> {
    let config: ConfigMap = serde_json::from_str(content)?;
    Ok(config)
}
