use crate::config::ConfigMap;
use crate::lsp::{LspClient, LspConfig as LspClientConfig, LspToken};
use crate::pane::Pane;
use crate::state::SharedState;
use crate::store;
use std::collections::HashMap;
use std::time::Instant;

const AUTO_SESSION_ID: &str = "_autosave";

/// A single notebook tab containing a set of panes (cells).
///
/// Each workspace has:
/// - A user-visible name
/// - A vertical stack of [`Pane`]s
/// - An active pane index for keyboard navigation
pub struct Workspace {
    pub name: String,
    pub panes: Vec<Pane>,
    pub active_pane: usize,
}

impl Workspace {
    /// Create a new workspace with a single auto-mode REPL pane.
    pub fn new(name: String, config: &ConfigMap, state: SharedState) -> Self {
        Self::with_language(name, config, "auto", state)
    }

    /// Create a new workspace with a single REPL pane for the given language.
    pub fn with_language(
        name: String,
        config: &ConfigMap,
        language: &str,
        state: SharedState,
    ) -> Self {
        let mut pane = Pane::new(0, language.to_string(), state, config.clone());
        let _ = pane.start_session(config);
        Self {
            name,
            panes: vec![pane],
            active_pane: 0,
        }
    }

    /// Mutable access to the active pane.
    pub fn current_pane_mut(&mut self) -> &mut Pane {
        &mut self.panes[self.active_pane]
    }

    /// Insert a new auto-mode cell after the active pane.
    pub fn add_cell(&mut self, config: &ConfigMap, state: SharedState) {
        let id = self.panes.len();
        let mut pane = Pane::new(id, "auto".to_string(), state, config.clone());
        let _ = pane.start_session(config);
        let insert_pos = self.active_pane + 1;
        self.panes.insert(insert_pos, pane);
        self.active_pane = insert_pos;
    }

    /// Remove the active cell. At least one cell is always kept.
    pub fn remove_cell(&mut self) {
        if self.panes.len() <= 1 {
            return;
        }
        self.panes.remove(self.active_pane);
        if self.active_pane >= self.panes.len() {
            self.active_pane = self.panes.len() - 1;
        }
    }

    /// Move the active cell up one position.
    pub fn move_cell_up(&mut self) {
        if self.active_pane > 0 {
            self.panes.swap(self.active_pane, self.active_pane - 1);
            self.active_pane -= 1;
        }
    }

    /// Move the active cell down one position.
    pub fn move_cell_down(&mut self) {
        if self.active_pane + 1 < self.panes.len() {
            self.panes.swap(self.active_pane, self.active_pane + 1);
            self.active_pane += 1;
        }
    }

    /// Poll output from all panes (non-blocking).
    pub fn poll(&mut self) {
        for pane in &mut self.panes {
            pane.poll_output();
        }
    }

    /// Return the index of an existing pane for `language`, or create one.
    pub fn ensure_pane(&mut self, language: &str, config: &ConfigMap, state: SharedState) -> usize {
        if let Some(pos) = self
            .panes
            .iter()
            .position(|p| p.active_language == language)
        {
            return pos;
        }
        let id = self.panes.len();
        let mut pane = Pane::new(id, language.to_string(), state, config.clone());
        let _ = pane.start_session(config);
        self.panes.push(pane);
        self.panes.len() - 1
    }
}

/// Top-level application state.
///
/// Holds all workspaces (tabs), the active workspace index, language
/// configurations, and the shared cross-language variable store.
pub struct App {
    pub workspaces: Vec<Workspace>,
    pub active_workspace: usize,
    pub config: ConfigMap,
    pub running: bool,
    pub state: SharedState,
    pub last_autosave: Instant,
    pub renaming_cell: bool,
    pub renaming_workspace: bool,
    /// Lazily-started LSP clients per language key.
    pub lsp_clients: HashMap<String, LspClient>,
    /// Cached LSP tokens for the active pane, recomputed before each draw.
    pub lsp_cache: Option<(Vec<LspToken>, String, String)>,
}

impl App {
    /// Create a new app with a default "Workspace 1" and Python REPL.
    pub fn new(config: ConfigMap) -> Self {
        let state = SharedState::new();
        let workspace = Workspace::new("Workspace 1".to_string(), &config, state.clone());
        Self {
            workspaces: vec![workspace],
            active_workspace: 0,
            config,
            running: true,
            state,
            last_autosave: Instant::now(),
            renaming_cell: false,
            renaming_workspace: false,
            lsp_clients: HashMap::new(),
            lsp_cache: None,
        }
    }

    /// Create a new app with a `noorc`-specified default language and aliases.
    pub fn with_noorc(
        config: ConfigMap,
        language: Option<&str>,
        aliases: HashMap<String, String>,
    ) -> Self {
        let lang = language.unwrap_or("auto");
        let state = SharedState::new();
        let mut workspace =
            Workspace::with_language("Workspace 1".to_string(), &config, lang, state.clone());
        workspace.panes[0].aliases = aliases;
        Self {
            workspaces: vec![workspace],
            active_workspace: 0,
            config,
            running: true,
            state,
            last_autosave: Instant::now(),
            renaming_cell: false,
            renaming_workspace: false,
            lsp_clients: HashMap::new(),
            lsp_cache: None,
        }
    }

    /// Mutable access to the active workspace.
    pub fn current_workspace_mut(&mut self) -> &mut Workspace {
        &mut self.workspaces[self.active_workspace]
    }

    /// Mutable access to the active pane (in the active workspace).
    pub fn current_pane_mut(&mut self) -> &mut Pane {
        self.current_workspace_mut().current_pane_mut()
    }

    /// Add a new cell after the active cell.
    pub fn add_cell(&mut self) {
        let config = self.config.clone();
        let state = self.state.clone();
        self.current_workspace_mut().add_cell(&config, state);
    }

    /// Remove the active cell.
    pub fn remove_cell(&mut self) {
        self.current_workspace_mut().remove_cell();
    }

    /// Move the active cell up.
    pub fn move_cell_up(&mut self) {
        self.current_workspace_mut().move_cell_up();
    }

    /// Move the active cell down.
    pub fn move_cell_down(&mut self) {
        self.current_workspace_mut().move_cell_down();
    }

    /// Add a new workspace tab.
    pub fn add_workspace(&mut self) {
        let name = format!("Workspace {}", self.workspaces.len() + 1);
        let config = self.config.clone();
        let state = self.state.clone();
        let ws = Workspace::new(name, &config, state);
        self.workspaces.push(ws);
        self.active_workspace = self.workspaces.len() - 1;
    }

    /// Remove the active workspace tab. At least one workspace is always kept.
    pub fn remove_workspace(&mut self) {
        if self.workspaces.len() <= 1 {
            return;
        }
        self.workspaces.remove(self.active_workspace);
        if self.active_workspace >= self.workspaces.len() {
            self.active_workspace = self.workspaces.len() - 1;
        }
    }

    /// Switch to the next workspace (right tab).
    pub fn next_workspace(&mut self) {
        if self.active_workspace + 1 < self.workspaces.len() {
            self.active_workspace += 1;
        }
        self.renaming_cell = false;
        self.renaming_workspace = false;
    }

    /// Switch to the previous workspace (left tab).
    pub fn previous_workspace(&mut self) {
        if self.active_workspace > 0 {
            self.active_workspace -= 1;
        }
        self.renaming_cell = false;
        self.renaming_workspace = false;
    }

    /// Toggle rename mode for the active cell.
    ///
    /// In rename mode, the input buffer shows the current name.
    pub fn toggle_rename_cell(&mut self) {
        self.renaming_workspace = false;
        self.renaming_cell = !self.renaming_cell;
        if self.renaming_cell {
            let pane = self.current_pane_mut();
            pane.input_buffer = pane.name.clone();
            pane.cursor_pos = pane.name.len();
        }
    }

    /// Toggle rename mode for the active workspace.
    pub fn toggle_rename_workspace(&mut self) {
        self.renaming_cell = false;
        self.renaming_workspace = !self.renaming_workspace;
        if self.renaming_workspace {
            let name = self.current_workspace_mut().name.clone();
            let pane = self.current_pane_mut();
            pane.input_buffer = name.clone();
            pane.cursor_pos = name.len();
        }
    }

    /// Commit the current rename operation (cell or workspace).
    pub fn commit_rename(&mut self) {
        if self.renaming_cell {
            let name = self.current_pane_mut().input_buffer.clone();
            self.current_pane_mut().name = name;
            self.current_pane_mut().input_buffer.clear();
            self.current_pane_mut().cursor_pos = 0;
            self.renaming_cell = false;
        } else if self.renaming_workspace {
            let name = {
                let pane = self.current_pane_mut();
                pane.input_buffer.clone()
            };
            self.current_workspace_mut().name = name;
            self.current_pane_mut().input_buffer.clear();
            self.current_pane_mut().cursor_pos = 0;
            self.renaming_workspace = false;
        }
    }

    /// Cancel the current rename operation without saving.
    pub fn cancel_rename(&mut self) {
        if self.renaming_cell || self.renaming_workspace {
            self.current_pane_mut().input_buffer.clear();
            self.current_pane_mut().cursor_pos = 0;
            self.renaming_cell = false;
            self.renaming_workspace = false;
        }
    }

    /// Poll output from all panes across all workspaces.
    pub fn poll_all_panes(&mut self) {
        for ws in &mut self.workspaces {
            ws.poll();
        }
    }

    /// Get or start an LSP client for the given language key.
    /// Returns `None` if the language has no LSP config or the server fails to start.
    pub fn get_lsp_client(&mut self, lang: &str) -> Option<&mut LspClient> {
        if self.lsp_clients.contains_key(lang) {
            return self.lsp_clients.get_mut(lang);
        }
        let lsp_cfg = self.config.get(lang).and_then(|c| c.lsp.clone())?;
        match LspClient::start(&LspClientConfig {
            cmd: lsp_cfg.cmd,
            args: lsp_cfg.args,
            language_id: lsp_cfg.language_id.clone(),
        }) {
            Ok(client) => {
                self.lsp_clients.insert(lang.to_string(), client);
                self.lsp_clients.get_mut(lang)
            }
            Err(e) => {
                eprintln!("LSP start error for '{}': {}", lang, e);
                None
            }
        }
    }

    /// Invalidate the LSP cache so it is recomputed on the next draw.
    pub fn invalidate_lsp_cache(&mut self) {
        self.lsp_cache = None;
    }

    /// Compute LSP tokens for the active pane, caching the result.
    /// Returns `true` if tokens were computed (LSP is available).
    pub fn compute_lsp_tokens(&mut self) -> bool {
        let (lang, text) = {
            let ws = self.current_workspace_mut();
            let lang = ws.panes[ws.active_pane].active_language.clone();
            let text = ws.panes[ws.active_pane].input_buffer.clone();
            (lang, text)
        };

        // Check if cache is still valid
        if let Some((_, ref cached_text, ref cached_lang)) = self.lsp_cache
            && cached_text == &text
            && cached_lang == &lang
        {
            return true;
        }

        let lsp_cfg = match self.config.get(&lang).and_then(|c| c.lsp.clone()) {
            Some(cfg) => cfg,
            None => return false,
        };

        let client = match self.get_lsp_client(&lang) {
            Some(c) => c,
            None => return false,
        };

        let uri = format!(
            "noo:///document.{}",
            if lang == "cpp" || lang == "cxx" {
                "cpp"
            } else {
                &lang
            }
        );

        match client.get_tokens(&text, &uri, &lsp_cfg.language_id) {
            Ok(tokens) => {
                let filtered: Vec<LspToken> = tokens
                    .into_iter()
                    .filter(|t| t.token_type != crate::lsp::NORM_NORMAL)
                    .collect();
                self.lsp_cache = Some((filtered, text, lang));
                true
            }
            Err(e) => {
                eprintln!("LSP token error: {}", e);
                false
            }
        }
    }

    /// Record a command in persistent history.
    pub fn record_command(&self, language: &str, command: &str, output_lines: &[String]) {
        store::push_command(language, command, output_lines);
    }

    /// Save the current session state under the given key.
    ///
    /// Serializes all workspaces, panes, history, and cursor positions to JSON.
    pub fn save_session(&self, key: &str) {
        let workspaces: Vec<store::SavedWorkspace> = self
            .workspaces
            .iter()
            .map(|ws| {
                let cells = ws
                    .panes
                    .iter()
                    .map(|p| store::SavedCell {
                        name: p.name.clone(),
                        active_language: p.active_language.clone(),
                        history: p.history.clone(),
                        execution_count: p.execution_count,
                        output_lines: p.output_lines.clone(),
                        input_buffer: p.input_buffer.clone(),
                        cursor_pos: p.cursor_pos,
                    })
                    .collect();
                store::SavedWorkspace {
                    name: ws.name.clone(),
                    active_pane: ws.active_pane,
                    cells,
                }
            })
            .collect();

        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis()
            .to_string();

        let record = store::SessionRecord {
            id: key.to_string(),
            name: key.to_string(),
            created_at: now.clone(),
            updated_at: now,
            workspaces,
        };
        store::update_session(key, record);
    }

    /// Load saved workspace data from a session key.
    pub fn load_workspaces_from_session(key: &str) -> Option<Vec<store::SavedWorkspace>> {
        let sessions = store::list_sessions();
        sessions
            .into_iter()
            .find(|s| s.id == key)
            .map(|s| s.workspaces)
    }

    /// Autosave the current state under the `_autosave` key.
    pub fn auto_save(&mut self) {
        self.save_session(AUTO_SESSION_ID);
        self.last_autosave = Instant::now();
    }

    /// Autosave if more than 10 seconds have elapsed since the last save.
    pub fn check_autosave_interval(&mut self) {
        if self.last_autosave.elapsed() > std::time::Duration::from_secs(10) {
            self.auto_save();
        }
    }

    /// Restore the app state from the `_autosave` session.
    ///
    /// Rebuilds all workspaces, panes, and starts fresh REPL sessions.
    /// Returns `true` on success.
    pub fn restore_from_autosave(&mut self) -> bool {
        let saved_workspaces = match Self::load_workspaces_from_session(AUTO_SESSION_ID) {
            Some(w) => w,
            None => return false,
        };

        self.workspaces = saved_workspaces
            .into_iter()
            .map(|saved_ws| {
                let panes: Vec<Pane> = saved_ws
                    .cells
                    .iter()
                    .enumerate()
                    .map(|(i, c)| {
                        let mut pane = Pane::new(
                            i,
                            c.active_language.clone(),
                            self.state.clone(),
                            self.config.clone(),
                        );
                        pane.name = c.name.clone();
                        pane.history = c.history.clone();
                        pane.history_index = c.history.len();
                        pane.execution_count = c.execution_count;
                        pane.output_lines = c.output_lines.clone();
                        pane.input_buffer = c.input_buffer.clone();
                        pane.cursor_pos = c.cursor_pos;
                        pane.aliases = self
                            .workspaces
                            .first()
                            .and_then(|ws| ws.panes.first())
                            .map(|p| p.aliases.clone())
                            .unwrap_or_default();
                        let _ = pane.start_session(&self.config);
                        pane
                    })
                    .collect();

                Workspace {
                    name: saved_ws.name.clone(),
                    panes,
                    active_pane: saved_ws.active_pane,
                }
            })
            .collect();

        self.active_workspace = 0;
        true
    }
}
