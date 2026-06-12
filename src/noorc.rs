use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

/// Startup configuration loaded from the `noorc` file.
///
/// The file is located at `%APPDATA%/NooBook/noorc` (Windows) or
/// `~/.config/NooBook/noorc` (Unix) and supports:
/// - `language <key>` — set default language
/// - `alias <name> = "<command>"` — define command aliases
/// - Bare lines — run as startup commands
#[derive(Default)]
pub struct Noorc {
    /// Default language for the initial REPL pane.
    pub language: Option<String>,
    /// Command aliases (name → expanded command).
    pub aliases: HashMap<String, String>,
    /// Commands to run on startup (bare lines in the noorc file).
    pub startup: Vec<String>,
}

/// Return the platform-specific path to the `noorc` file.
fn noorc_path() -> PathBuf {
    let base = std::env::var("APPDATA")
        .or_else(|_| std::env::var("HOME"))
        .unwrap_or_else(|_| ".".to_string());
    PathBuf::from(base).join("NooBook").join("noorc")
}

impl Noorc {
    /// Load the `noorc` file from the default location.
    ///
    /// If the file does not exist or cannot be read, returns a default empty config.
    pub fn load() -> Self {
        let path = noorc_path();
        let content = match fs::read_to_string(&path) {
            Ok(c) => c,
            Err(_) => return Self::default(),
        };

        let mut noorc = Noorc::default();

        for line in content.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }

            if let Some(rest) = line.strip_prefix("language ") {
                let lang = rest.trim().trim_matches('"').trim_matches('\'');
                noorc.language = Some(lang.to_string());
            } else if let Some(rest) = line.strip_prefix("alias ") {
                if let Some((name, cmd)) = rest.split_once('=') {
                    let name = name.trim();
                    let cmd = cmd.trim().trim_matches('"').trim_matches('\'');
                    if !name.is_empty() && !cmd.is_empty() {
                        noorc.aliases.insert(name.to_string(), cmd.to_string());
                    }
                }
            } else {
                noorc.startup.push(line.to_string());
            }
        }

        noorc
    }
}
