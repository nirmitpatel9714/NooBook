use crate::bridge;
use crate::config::ConfigMap;
use crate::execution::ProcessSession;
use crate::state::SharedState;
use std::collections::HashMap;
use std::env;
use std::time::Duration;
use tokio::sync::mpsc;

/// Check if a line is a REPL startup banner that should be hidden.
fn is_banner_line(line: &str) -> bool {
    let t = line.trim();
    // Python version header: "Python 3.14.5 (tags/...) on win32"
    (t.starts_with("Python ") && t.contains(" on "))
    // Any help/copyright/license info line (Python, Node, etc.)
    || (t.len() < 80 && (t.starts_with("Type ") || t.starts_with("Copyright ") || t.eq_ignore_ascii_case("all rights reserved.")))
    // Node.js welcome
    || t.starts_with("Welcome to Node.js")
    // PowerShell header
    || t == "Windows PowerShell"
}

/// Check if a line is a bare REPL prompt (no user output) that should be hidden.
fn is_prompt_noise(line: &str) -> bool {
    let t = line.trim();
    t == ">>>"
        || t == "..."
        || t == ">"
        || t.starts_with(">>> ")
        || t.starts_with("... ")
        || t.starts_with("> ")
}

/// Detect the most likely language for a command using keyword heuristics.
///
/// Returns `Ok(language)` if a clear winner is found (score gap ≥ 2), or
/// `Err(candidates)` if the command is ambiguous across multiple languages.
fn detect_language(input: &str) -> Result<String, Vec<String>> {
    let trimmed = input.trim();
    let mut scores: HashMap<&str, i32> = HashMap::new();

    macro_rules! score {
        ($lang:expr, $pts:expr) => {
            *scores.entry($lang).or_insert(0) += $pts;
        };
    }

    // ── Python high-confidence patterns ──
    if trimmed.starts_with("print(") || trimmed.starts_with("print ") {
        score!("py", 3);
    }
    if trimmed.starts_with("import ") || trimmed.starts_with("from ") {
        score!("py", 3);
    }
    if trimmed.starts_with("def ") || trimmed.starts_with("class ") {
        score!("py", 3);
    }
    if trimmed.starts_with("elif ")
        || trimmed.starts_with("else:")
        || trimmed.starts_with("except:")
        || trimmed.starts_with("except ")
    {
        score!("py", 3);
    }
    if trimmed.starts_with("return ")
        || trimmed.starts_with("yield ")
        || trimmed.starts_with("raise ")
        || trimmed.starts_with("assert ")
        || trimmed.starts_with("pass")
        || trimmed.starts_with("break")
        || trimmed.starts_with("continue")
    {
        score!("py", 3);
    }
    if trimmed.starts_with("with ")
        || trimmed.starts_with("try:")
        || trimmed.starts_with("try ")
        || trimmed.starts_with("finally:")
    {
        score!("py", 3);
    }
    if trimmed.starts_with("lambda ") || trimmed.starts_with("del ") {
        score!("py", 3);
    }
    if trimmed.contains("globals()")
        || trimmed.contains("locals()")
        || trimmed.contains("exec(")
        || trimmed.contains("eval(")
        || trimmed.contains("__name__")
        || trimmed.contains("__main__")
    {
        score!("py", 3);
    }
    if trimmed.starts_with("@") {
        score!("py", 2);
    }
    if trimmed.contains(" in ") && (trimmed.contains(" for ") || trimmed.contains(" if ")) {
        score!("py", 1);
    }

    // ── JavaScript high-confidence patterns ──
    if trimmed.starts_with("console.") {
        score!("js", 3);
    }
    if trimmed.starts_with("var ") || trimmed.starts_with("let ") || trimmed.starts_with("const ") {
        score!("js", 3);
    }
    if trimmed.starts_with("function ")
        || trimmed.starts_with("async ")
        || trimmed.starts_with("await ")
    {
        score!("js", 3);
    }
    if trimmed.starts_with("typeof ")
        || trimmed.starts_with("instanceof ")
        || trimmed.starts_with("delete ")
    {
        score!("js", 3);
    }
    if trimmed.starts_with("require(")
        || trimmed.starts_with("setTimeout(")
        || trimmed.starts_with("setInterval(")
    {
        score!("js", 3);
    }
    if trimmed.starts_with("throw ") {
        score!("js", 2);
    }
    if trimmed.contains("===") || trimmed.contains("!==") {
        score!("js", 3);
    }
    if trimmed.contains("=>") {
        score!("js", 2);
    }
    if trimmed.contains("this.") || trimmed.contains("global.") {
        score!("js", 2);
    }
    if trimmed.contains("process.")
        || trimmed.contains("module.")
        || trimmed.contains("exports.")
        || trimmed.contains("require(")
    {
        score!("js", 3);
    }
    if trimmed.contains("Array.")
        || trimmed.contains("Object.")
        || trimmed.contains("JSON.")
        || trimmed.contains("Math.")
        || trimmed.contains("Date.")
        || trimmed.contains("Promise")
    {
        score!("js", 1);
    }

    // ── PowerShell high-confidence patterns ──
    if trimmed.starts_with("Write-")
        || trimmed.starts_with("Get-")
        || trimmed.starts_with("Set-")
        || trimmed.starts_with("New-")
        || trimmed.starts_with("Remove-")
        || trimmed.starts_with("Start-")
        || trimmed.starts_with("Stop-")
        || trimmed.starts_with("Restart-")
        || trimmed.starts_with("Format-")
        || trimmed.starts_with("Out-")
        || trimmed.starts_with("Import-")
        || trimmed.starts_with("Export-")
    {
        score!("ps", 3);
    }
    if trimmed.contains("Where-Object")
        || trimmed.contains("Select-Object")
        || trimmed.contains("ForEach-Object")
        || trimmed.contains("Sort-Object")
        || trimmed.contains("Group-Object")
    {
        score!("ps", 3);
    }
    if trimmed.contains("$_")
        || trimmed.contains("$?")
        || trimmed.contains("$args")
        || trimmed.contains("$PS")
    {
        score!("ps", 3);
    }
    if trimmed.contains("$true") || trimmed.contains("$false") || trimmed.contains("$null") {
        score!("ps", 3);
    }
    if trimmed.contains(" -eq ")
        || trimmed.contains(" -ne ")
        || trimmed.contains(" -gt ")
        || trimmed.contains(" -lt ")
        || trimmed.contains(" -ge ")
        || trimmed.contains(" -le ")
        || trimmed.contains(" -like ")
        || trimmed.contains(" -notlike ")
        || trimmed.contains(" -match ")
        || trimmed.contains(" -notmatch ")
        || trimmed.contains(" -contains ")
        || trimmed.contains(" -notcontains ")
    {
        score!("ps", 3);
    }
    if trimmed.contains(" -is ") || trimmed.contains(" -isnot ") || trimmed.contains(" -as ") {
        score!("ps", 2);
    }
    if trimmed.contains("|")
        && (trimmed.contains("Where")
            || trimmed.contains("Select")
            || trimmed.contains("ForEach")
            || trimmed.contains("Sort"))
    {
        score!("ps", 1);
    }

    // Score neutral patterns: dollar-prefixed variables are PowerShell
    if trimmed.starts_with('$') {
        score!("ps", 2);
    }

    let mut sorted: Vec<_> = scores.into_iter().collect();
    sorted.sort_by_key(|b| std::cmp::Reverse(b.1));

    if sorted.is_empty() {
        return Err(vec![]);
    }
    if sorted.len() == 1 || sorted[0].1 - sorted[1].1 >= 2 {
        Ok(sorted[0].0.to_string())
    } else {
        Err(sorted.into_iter().map(|(l, _)| l.to_string()).collect())
    }
}

/// A single notebook cell connected to one REPL subprocess.
///
/// Each pane maintains its own:
/// - Input buffer and cursor position
/// - Output history for display
/// - Command history with navigation index
/// - Language-specific REPL session ([`ProcessSession`])
/// - [`SharedState`] reference for the cross-language state bridge
pub struct Pane {
    pub id: usize,
    /// User-visible cell name (set via rename mode).
    pub name: String,
    /// Currently active language key (e.g., `"py"`, `"js"`).
    pub active_language: String,
    /// Current input buffer contents.
    pub input_buffer: String,
    /// Cursor position within `input_buffer`.
    pub cursor_pos: usize,
    /// Lines of output received from the REPL since last clear.
    pub output_lines: Vec<String>,
    /// Active REPL subprocess connection.
    pub session: Option<ProcessSession>,
    /// Receiver for output lines from the subprocess.
    pub output_receiver: mpsc::Receiver<String>,
    /// Sender half kept for restarting sessions.
    output_sender: mpsc::Sender<String>,
    /// Per-cell command history.
    pub history: Vec<String>,
    /// Current position in the history for up/down navigation.
    pub history_index: usize,
    /// Number of executions performed.
    pub execution_count: usize,
    /// Command aliases defined via `noorc`.
    pub aliases: HashMap<String, String>,
    /// Reference to the shared cross-language variable store.
    pub state: SharedState,
    /// Language configuration map (used by auto mode to detect & start sessions).
    pub config_map: ConfigMap,
}

impl Pane {
    /// Create a new pane with the given ID and default language.
    ///
    /// The session is not started until [`start_session`](Self::start_session) is called.
    pub fn new(
        id: usize,
        default_language: String,
        state: SharedState,
        config_map: ConfigMap,
    ) -> Self {
        let (output_sender, output_receiver) = mpsc::channel(100);
        Self {
            id,
            name: String::new(),
            active_language: default_language,
            input_buffer: String::new(),
            cursor_pos: 0,
            output_lines: Vec::new(),
            session: None,
            output_receiver,
            output_sender,
            history: Vec::new(),
            history_index: 0,
            execution_count: 0,
            aliases: HashMap::new(),
            state,
            config_map,
        }
    }

    /// Start a REPL subprocess for the pane's active language.
    ///
    /// Looks up `active_language` in the config map and spawns the process.
    /// For `"auto"` language mode, no session is started (detection happens at
    /// execution time).
    /// Errors (missing config, process spawn failure) are pushed to `output_lines`.
    pub fn start_session(&mut self, config_map: &ConfigMap) -> Result<(), String> {
        if self.active_language == "auto" {
            return Ok(());
        }
        if let Some(config) = config_map.get(&self.active_language) {
            match ProcessSession::start(config, self.output_sender.clone()) {
                Ok(session) => {
                    self.session = Some(session);
                    Ok(())
                }
                Err(e) => {
                    let err_msg = format!("Failed to start process: {}", e);
                    self.output_lines.push(err_msg.clone());
                    Err(err_msg)
                }
            }
        } else {
            let err_msg = format!("Language {} not found in config.", self.active_language);
            self.output_lines.push(err_msg.clone());
            Err(err_msg)
        }
    }

    /// Process input from `input_buffer`.
    ///
    /// Built-in commands (`clear`, `cd`, `ls`, `exit`, `noo`, aliases) are handled
    /// directly. Otherwise:
    /// - In **auto mode**: detects the language from the command, runs it in a
    ///   temporary REPL, and reports ambiguity if more than one language matches.
    /// - In **specific language mode**: injects shared state, sends user code,
    ///   dumps state back.
    ///
    /// Returns `Some("exit")` or `Some("nbmode")` for mode-switching commands.
    pub async fn handle_input(&mut self) -> Option<String> {
        if self.input_buffer.trim().is_empty() {
            return None;
        }
        let input = self.input_buffer.clone();

        if self.history.is_empty() || self.history.last().unwrap() != &input {
            self.history.push(input.clone());
        }
        self.history_index = self.history.len();

        self.execution_count += 1;
        self.input_buffer.clear();
        self.cursor_pos = 0;

        let expanded = self.aliases.get(input.trim()).cloned();
        let input = expanded.as_deref().unwrap_or(&input).to_string();

        let parts: Vec<&str> = input.split_whitespace().collect();
        match parts[0] {
            "clear" => {
                self.output_lines.clear();
                return None;
            }
            "cd" => {
                if parts.len() > 1
                    && let Err(e) = env::set_current_dir(parts[1])
                {
                    self.output_lines.push(format!("cd error: {}", e));
                }
                return None;
            }
            "ls" | "la" => {
                if let Ok(entries) = std::fs::read_dir(".") {
                    let mut files = Vec::new();
                    for entry in entries.flatten() {
                        let name = entry.file_name().to_string_lossy().to_string();
                        let md = entry.metadata().ok();
                        let is_dir = md.map(|m| m.is_dir()).unwrap_or(false);
                        if is_dir {
                            files.push(format!("[DIR] {}", name));
                        } else {
                            files.push(name);
                        }
                    }
                    self.output_lines.push(files.join("  "));
                }
                return None;
            }
            "noo" => {
                if parts.len() > 1 && parts[1] == "nbmode" {
                    return Some("nbmode".to_string());
                }
                return None;
            }
            "exit" => {
                return Some("exit".to_string());
            }
            _ => {}
        }

        if self.active_language == "auto" {
            self.handle_auto_input(&input).await;
        } else if let Some(session) = &mut self.session {
            if let Some(code) = bridge::injection_code(&self.state, &self.active_language) {
                session.send_input(&code).await;
            }
            session.send_input(&input).await;
            if let Some(code) = bridge::dump_code(&self.active_language) {
                session.send_input(&code).await;
            }
        } else {
            self.output_lines.push("No active session.".to_string());
        }

        None
    }

    /// Handle input in auto mode: detect language and run in a fresh temp session.
    async fn handle_auto_input(&mut self, input: &str) {
        match detect_language(input) {
            Ok(lang) => {
                if let Some(config) = self.config_map.get(&lang) {
                    let (tx, mut rx) = mpsc::channel(100);
                    match ProcessSession::start(config, tx) {
                        Ok(session) => {
                            let inj_code = bridge::injection_code(&self.state, &lang);
                            let dump_code = bridge::dump_code(&lang);

                            if let Some(ref code) = inj_code {
                                session.send_input(code).await;
                            }
                            session.send_input(input).await;
                            if let Some(ref code) = dump_code {
                                session.send_input(code).await;
                            }

                            let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
                            let mut idle = 0u32;
                            loop {
                                match rx.try_recv() {
                                    Ok(line) => {
                                        idle = 0;
                                        let trimmed = line.trim_start_matches('>').trim_start();
                                        if let Some(rest) =
                                            trimmed.strip_prefix(bridge::STATE_PREFIX)
                                        {
                                            self.state.import_json(rest);
                                        } else if !is_banner_line(&line) && !is_prompt_noise(&line)
                                        {
                                            let is_echo = Some(line.as_str())
                                                == inj_code.as_deref()
                                                || line == input
                                                || Some(line.as_str()) == dump_code.as_deref();
                                            if !is_echo {
                                                self.output_lines.push(line);
                                            }
                                        }
                                    }
                                    Err(mpsc::error::TryRecvError::Empty) => {
                                        if tokio::time::Instant::now() >= deadline || idle > 6 {
                                            break;
                                        }
                                        idle += 1;
                                        tokio::time::sleep(Duration::from_millis(50)).await;
                                    }
                                    Err(mpsc::error::TryRecvError::Disconnected) => break,
                                }
                            }
                        }
                        Err(e) => {
                            self.output_lines
                                .push(format!("Auto: failed to start {} — {}", lang, e));
                        }
                    }
                } else {
                    self.output_lines
                        .push(format!("Auto: detected {} but no config found.", lang));
                }
            }
            Err(candidates) => {
                if candidates.is_empty() {
                    self.output_lines.push(
                        "Auto: couldn't detect language. Use py(<code>), js(<code>), etc."
                            .to_string(),
                    );
                } else {
                    let suggestions: Vec<String> = candidates
                        .iter()
                        .map(|l| format!("{}(<code>)", l))
                        .collect();
                    self.output_lines.push(format!(
                        "Auto: ambiguous command — matched {}. Be specific with: {}",
                        candidates.join(", "),
                        suggestions.join(", "),
                    ));
                }
            }
        }
    }

    /// Drain the output channel non-blockingly.
    ///
    /// Lines prefixed with [`crate::bridge::STATE_PREFIX`] are intercepted and merged into
    /// [`SharedState`]; all other lines are pushed to `output_lines`.
    pub fn poll_output(&mut self) {
        while let Ok(line) = self.output_receiver.try_recv() {
            let trimmed = line.trim_start_matches('>').trim_start();
            if let Some(rest) = trimmed.strip_prefix(bridge::STATE_PREFIX) {
                self.state.import_json(rest);
            } else if !is_banner_line(&line) && !is_prompt_noise(&line) {
                self.output_lines.push(line);
            }
        }
    }
}
