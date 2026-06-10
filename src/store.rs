use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

/// Return the platform-specific data directory (`%APPDATA%/nooshell` or `~/.local/share/nooshell`).
fn data_dir() -> PathBuf {
    let base = std::env::var("APPDATA")
        .or_else(|_| std::env::var("HOME"))
        .unwrap_or_else(|_| ".".to_string());
    let dir = PathBuf::from(base).join("nooshell");
    let _ = fs::create_dir_all(&dir);
    dir
}

fn history_path() -> PathBuf {
    data_dir().join("history.json")
}

fn sessions_path() -> PathBuf {
    data_dir().join("sessions.json")
}

/// Generate a millisecond timestamp string.
fn timestamp() -> String {
    let start = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();
    format!("{}", start)
}

// ── Command history ──

/// A single command execution record persisted to disk.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CommandRecord {
    /// Unique nanosecond-precision ID.
    pub id: u64,
    /// Session identifier (reserved for future use).
    pub session_key: String,
    /// Language the command was executed in.
    pub language: String,
    /// The command text.
    pub command: String,
    /// ISO-ish timestamp of execution.
    pub timestamp: String,
    /// First line of output (for quick preview).
    pub output_preview: String,
}

/// In-memory representation of `history.json`.
#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct HistoryStore {
    pub commands: Vec<CommandRecord>,
}

/// Load command history from disk.
pub fn load_history() -> HistoryStore {
    fs::read_to_string(history_path())
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

/// Persist command history to disk.
pub fn save_history(store: &HistoryStore) {
    if let Ok(json) = serde_json::to_string_pretty(store) {
        let _ = fs::write(history_path(), json);
    }
}

/// Add a new command record to the history and persist.
pub fn push_command(language: &str, command: &str, output: &[String]) {
    let mut store = load_history();
    let id = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos() as u64;
    let preview = output.first().cloned().unwrap_or_default();
    store.commands.push(CommandRecord {
        id,
        session_key: String::new(),
        language: language.to_string(),
        command: command.to_string(),
        timestamp: timestamp(),
        output_preview: preview,
    });
    save_history(&store);
}

/// Return the on-disk path to the history file (for display).
pub fn history_path_str() -> String {
    history_path().to_string_lossy().to_string()
}

/// Clear all command history.
pub fn clear_history() {
    save_history(&HistoryStore::default());
}

// ── Session persistence ──

/// Serializable snapshot of a single notebook cell.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SavedCell {
    pub name: String,
    pub active_language: String,
    pub history: Vec<String>,
    pub execution_count: usize,
    pub output_lines: Vec<String>,
    pub input_buffer: String,
    pub cursor_pos: usize,
}

/// Serializable snapshot of a single workspace.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SavedWorkspace {
    pub name: String,
    pub active_pane: usize,
    pub cells: Vec<SavedCell>,
}

/// A saved session containing one or more workspaces.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SessionRecord {
    /// Unique session ID (e.g., `"_autosave"` or a user-given name).
    pub id: String,
    /// Human-readable session name.
    pub name: String,
    pub created_at: String,
    pub updated_at: String,
    pub workspaces: Vec<SavedWorkspace>,
}

/// In-memory representation of `sessions.json`.
#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct SessionStore {
    pub sessions: Vec<SessionRecord>,
}

/// Load saved sessions from disk.
pub fn load_sessions() -> SessionStore {
    fs::read_to_string(sessions_path())
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

/// Persist the session store to disk.
pub fn save_sessions(store: &SessionStore) {
    if let Ok(json) = serde_json::to_string_pretty(store) {
        let _ = fs::write(sessions_path(), json);
    }
}

/// Append a new session record (deprecated; prefer [`update_session`]).
pub fn push_session(record: SessionRecord) {
    let mut store = load_sessions();
    store.sessions.push(record);
    save_sessions(&store);
}

/// Insert or update a session record by ID.
pub fn update_session(id: &str, record: SessionRecord) {
    let mut store = load_sessions();
    if let Some(pos) = store.sessions.iter().position(|s| s.id == id) {
        store.sessions[pos] = record;
    } else {
        store.sessions.push(record);
    }
    save_sessions(&store);
}

/// Delete a session by ID. Returns `true` if a session was removed.
pub fn delete_session(id: &str) -> bool {
    let mut store = load_sessions();
    let len_before = store.sessions.len();
    store.sessions.retain(|s| s.id != id);
    let removed = store.sessions.len() < len_before;
    save_sessions(&store);
    removed
}

/// Return all saved session records.
pub fn list_sessions() -> Vec<SessionRecord> {
    load_sessions().sessions
}
