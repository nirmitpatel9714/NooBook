use crate::config::ConfigMap;
use std::collections::HashMap;
use std::fs;
use std::path::Path;

/// A parsed `.ns` script — a sequence of lines, each tagged with a language key.
///
/// Each line is a tuple of `(language_alias, code)`. Lines without a language
/// prefix inherit the language from the preceding line.
pub struct NsScript {
    pub lines: Vec<(Option<String>, String)>,
}

// ── Variable analysis helpers ──

/// Keywords excluded from variable-name analysis.
const KEYWORDS: &[&str] = &[
    "print", "console", "log", "typeof", "undefined", "true", "false", "null",
    "if", "else", "for", "while", "return", "import", "from", "def", "class",
    "let", "var", "const", "function", "new", "this", "in", "of", "not", "and", "or",
    "try", "catch", "finally", "throw", "async", "await", "yield", "global",
    "require", "module", "exports", "__dirname", "__filename", "process",
    "Object", "Array", "String", "Number", "Boolean", "JSON", "Math", "Date",
    "console", "dir", "globals", "builtins",
];

fn is_keyword(s: &str) -> bool {
    KEYWORDS.contains(&s)
}

fn is_number(s: &str) -> bool {
    s.chars().all(|c| c.is_ascii_digit() || c == '.')
}

fn sanitize_var_name(s: &str) -> Option<String> {
    let cleaned: String = s
        .chars()
        .filter(|c| c.is_alphanumeric() || *c == '_')
        .collect();
    if cleaned.len() > 1
        && (cleaned.starts_with(|c: char| c.is_alphabetic() || c == '_'))
        && !is_keyword(&cleaned)
    {
        Some(cleaned)
    } else {
        None
    }
}

/// Extract variable names being assigned to (left side of `=`).
///
/// Handles string-delimited content to avoid false positives.
fn extract_assignments(code: &str) -> Vec<String> {
    let mut vars = Vec::new();
    let mut in_string = false;
    let mut string_char = ' ';
    let bytes = code.as_bytes();
    let len = bytes.len();
    let mut i = 0;
    while i < len {
        let ch = bytes[i] as char;
        if in_string {
            if ch == string_char && (i == 0 || bytes[i - 1] != b'\\') {
                in_string = false;
            }
            i += 1;
            continue;
        }
        if ch == '"' || ch == '\'' {
            in_string = true;
            string_char = ch;
            i += 1;
            continue;
        }
        if ch == '=' && i > 0 && bytes[i - 1] as char != '=' && i + 1 < len && bytes[i + 1] as char != '=' {
            let mut j = i.saturating_sub(1);
            while j > 0 && (bytes[j - 1] as char).is_whitespace() {
                j -= 1;
            }
            let start = j;
            while j > 0
                && ((bytes[j - 1] as char).is_alphanumeric() || bytes[j - 1] as char == '_')
            {
                j -= 1;
            }
            let candidate = &code[j..start];
            if let Some(cleaned) = sanitize_var_name(candidate) {
                vars.push(cleaned);
            }
        }
        i += 1;
    }
    vars
}

/// Extract all identifiers from code (excluding keywords, numbers, string contents).
fn extract_words(code: &str) -> Vec<String> {
    let mut words = Vec::new();
    let mut current = String::new();
    let mut in_string = false;
    let mut string_char = ' ';
    for ch in code.chars() {
        if in_string {
            if ch == string_char {
                in_string = false;
            }
            continue;
        }
        if ch == '"' || ch == '\'' {
            in_string = true;
            string_char = ch;
            if !current.is_empty() {
                if !is_keyword(&current) && !is_number(&current) {
                    words.push(current.clone());
                }
                current.clear();
            }
            continue;
        }
        if ch.is_alphanumeric() || ch == '_' {
            current.push(ch);
        } else {
            if !current.is_empty() && !is_keyword(&current) && !is_number(&current) {
                words.push(current.clone());
            }
            current.clear();
        }
    }
    if !current.is_empty() && !is_keyword(&current) && !is_number(&current) {
        words.push(current);
    }
    words
}

/// Compute a variable cleanup schedule for compiled scripts.
///
/// For each variable, determines the last line index where it is referenced.
/// Returns a list of `(line_index, "del var1, var2, ...")` sorted by line index.
pub fn compute_cleanup_schedule(lines: &[(Option<String>, String)]) -> Vec<(usize, String)> {
    let mut var_first: HashMap<String, usize> = HashMap::new();
    let mut var_last: HashMap<String, usize> = HashMap::new();

    for (i, (_, code)) in lines.iter().enumerate() {
        for var in extract_assignments(code) {
            var_first.entry(var.clone()).or_insert(i);
            var_last.insert(var.clone(), i);
        }
        for var in extract_words(code) {
            var_last.insert(var, i);
        }
    }

    let mut cleanup: Vec<(usize, String)> = Vec::new();
    for (var, &last) in &var_last {
        if var_first.contains_key(var) {
            cleanup.push((last, format!("del {}", var)));
        }
    }

    cleanup.sort_by_key(|(i, _)| *i);
    let mut deduped: Vec<(usize, Vec<String>)> = Vec::new();
    for (line, cmd) in cleanup {
        let var = cmd[4..].to_string();
        if let Some(last) = deduped.last_mut() {
            if last.0 == line {
                last.1.push(var);
                continue;
            }
        }
        deduped.push((line, vec![var]));
    }
    deduped
        .into_iter()
        .map(|(line, vars)| (line, format!("del {}", vars.join(", "))))
        .collect()
}

impl NsScript {
    /// Load a `.ns` script from a file path.
    pub fn load<P: AsRef<Path>>(path: P, config_map: &ConfigMap) -> std::io::Result<Self> {
        let content = fs::read_to_string(path)?;
        Self::from_string(&content, config_map)
    }

    /// Parse a `.ns` script from a string.
    ///
    /// Lines starting with a language key (matching `config_map`) set the active language.
    /// Lines starting with `del ` are treated as manual delete directives (no language context).
    /// All other lines inherit the preceding line's language.
    pub fn from_string(content: &str, config_map: &ConfigMap) -> std::io::Result<Self> {
        let mut lines = Vec::new();
        let mut last_lang: Option<String> = None;

        for line in content.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            if let Some((first, rest)) = line.split_once(' ') {
                let first = first.trim();
                if config_map.contains_key(first) {
                    last_lang = Some(first.to_string());
                    lines.push((last_lang.clone(), rest.trim().to_string()));
                    continue;
                }
            }
            if line.starts_with("del ") {
                lines.push((None, line.to_string()));
                continue;
            }
            if let Some(ref lang) = last_lang {
                lines.push((Some(lang.clone()), line.to_string()));
            }
        }
        Ok(Self { lines })
    }

    /// Run a `.ns` script from its raw content string, using the embedded `languages.json`.
    pub async fn run_embedded(content: &str, languages_json: &str) {
        Self::run_embedded_with_cleanup(content, languages_json, &[]).await;
    }

    /// Run a `.ns` script with an explicit cleanup schedule (used by the compiler).
    ///
    /// Each script line is executed in a **separate fresh subprocess** (not a
    /// persistent session). State is injected before each line and dumped after.
    pub async fn run_embedded_with_cleanup(
        content: &str,
        languages_json: &str,
        cleanup: &[(usize, String)],
    ) {
        let config = std::sync::Arc::new(
            crate::config::load_from_str(languages_json).unwrap_or_default(),
        );
        let script = match NsScript::from_string(content, config.as_ref()) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("Failed to parse script: {}", e);
                return;
            }
        };

        let mut cleanup_map: HashMap<usize, Vec<String>> = HashMap::new();
        for (line_idx, cmd) in cleanup {
            cleanup_map
                .entry(*line_idx)
                .or_default()
                .push(cmd.clone());
        }

        let state = crate::state::SharedState::new();
        let mut handles = Vec::new();

        for (i, (alias, code)) in script.lines.iter().enumerate() {
            if alias.is_none() && code.starts_with("del ") {
                let var_name = code[4..].trim().to_string();
                state.remove(&var_name);
                continue;
            }

            let lang = alias.as_deref().unwrap_or("py").to_string();
            let code = code.clone();
            let config = config.clone();
            let state = state.clone();

            let after_cleanup: Vec<String> = cleanup_map.remove(&i).unwrap_or_default();

            handles.push(tokio::spawn(async move {
                if let Some(cfg) = config.get(&lang) {
                    let (tx, mut rx) = tokio::sync::mpsc::channel(100);
                    if let Ok(session) = crate::execution::ProcessSession::start(cfg, tx) {
                        if let Some(inj) = crate::bridge::injection_code(&state, &lang) {
                            session.send_input(&inj).await;
                        }
                        session.send_input(&code).await;

                        if let Some(dump) = crate::bridge::dump_code(&lang) {
                            session.send_input(&dump).await;
                        }

                        for cmd in &after_cleanup {
                            let lang = &*lang;
                            match lang {
                                "py" => {
                                    session.send_input(cmd).await;
                                }
                                "js" => {
                                    let vars: Vec<&str> =
                                        cmd.trim_start_matches("del ").split(", ").collect();
                                    let js_del: String = vars
                                        .iter()
                                        .map(|v| format!("delete global.{}", v.trim()))
                                        .collect::<Vec<_>>()
                                        .join("; ");
                                    session.send_input(&js_del).await;
                                }
                                _ => {
                                    session.send_input(cmd).await;
                                }
                            }
                        }

                        tokio::time::sleep(tokio::time::Duration::from_millis(1000)).await;
                        while let Ok(line) = rx.try_recv() {
                            let trimmed = line.trim_start_matches('>').trim_start();
                            if let Some(rest) =
                                trimmed.strip_prefix(crate::bridge::STATE_PREFIX)
                            {
                                state.import_json(rest);
                            } else if !line.is_empty() {
                                println!("{}", line);
                            }
                        }
                    }
                }
            }));
        }

        for h in handles {
            let _ = h.await;
        }
    }
}
