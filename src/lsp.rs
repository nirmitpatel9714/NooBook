use serde_json::{json, Value};
use std::collections::HashMap;
use std::io::{BufRead, BufReader, Read, Write};
use std::process::{Child, ChildStdin, Command, Stdio};

/// Configuration for an LSP server used for syntax highlighting.
#[derive(Debug, Clone)]
pub struct LspConfig {
    pub cmd: String,
    pub args: Vec<String>,
    pub language_id: String,
}

/// A semantic token with byte-offset positioning.
#[derive(Debug, Clone)]
pub struct LspToken {
    /// Byte offset from start of document.
    pub offset: usize,
    /// Length in bytes.
    pub length: usize,
    /// Normalized token type:
    ///   0=normal, 1=keyword, 2=string, 3=comment, 4=number,
    ///   5=variable, 6=builtin, 7=type
    pub token_type: u32,
}

/// Manages a single LSP server process and communicates via JSON-RPC over stdio.
pub struct LspClient {
    stdin: ChildStdin,
    _child: Child,
    request_id: u64,
    /// Map from LSP token type name (lowercase) to normalized type.
    type_name_to_normalized: HashMap<String, u32>,
    /// Buffer for reading response headers.
    reader: BufReader<Box<dyn std::io::Read + Send + 'static>>,
    /// Whether the server has been initialized.
    initialized: bool,
    /// Current document version counter.
    doc_version: i64,
}

// ── Normalized token type constants ──
pub const NORM_NORMAL: u32 = 0;
pub const NORM_KEYWORD: u32 = 1;
pub const NORM_STRING: u32 = 2;
pub const NORM_COMMENT: u32 = 3;
pub const NORM_NUMBER: u32 = 4;
pub const NORM_VARIABLE: u32 = 5;
pub const NORM_BUILTIN: u32 = 6;
pub const NORM_TYPE: u32 = 7;
pub const NORM_PARAMETER: u32 = 8;
pub const NORM_OPERATOR: u32 = 9;

impl LspClient {
    /// Spawn an LSP server and perform the initialize handshake.
    pub fn start(config: &LspConfig) -> Result<Self, String> {
        let mut child = Command::new(&config.cmd)
            .args(&config.args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|e| format!("Failed to spawn LSP '{}': {}", config.cmd, e))?;

        let stdin = child.stdin.take()
            .ok_or_else(|| "LSP stdin not available".to_string())?;
        let stdout = child.stdout.take()
            .ok_or_else(|| "LSP stdout not available".to_string())?;
        let reader = BufReader::new(Box::new(stdout) as Box<dyn std::io::Read + Send>);

        let mut client = LspClient {
            stdin,
            _child: child,
            request_id: 1,
            type_name_to_normalized: Self::default_normalized_map(),
            reader,
            initialized: false,
            doc_version: 0,
        };

        client.initialize()?;
        Ok(client)
    }

    fn default_normalized_map() -> HashMap<String, u32> {
        let mut m = HashMap::new();
        m.insert("keyword".to_string(), NORM_KEYWORD);
        m.insert("string".to_string(), NORM_STRING);
        m.insert("comment".to_string(), NORM_COMMENT);
        m.insert("number".to_string(), NORM_NUMBER);
        m.insert("variable".to_string(), NORM_VARIABLE);
        m.insert("function".to_string(), NORM_BUILTIN);
        m.insert("method".to_string(), NORM_BUILTIN);
        m.insert("macro".to_string(), NORM_BUILTIN);
        m.insert("type".to_string(), NORM_TYPE);
        m.insert("class".to_string(), NORM_TYPE);
        m.insert("struct".to_string(), NORM_TYPE);
        m.insert("enum".to_string(), NORM_TYPE);
        m.insert("interface".to_string(), NORM_TYPE);
        m.insert("parameter".to_string(), NORM_PARAMETER);
        m.insert("operator".to_string(), NORM_OPERATOR);
        m.insert("property".to_string(), NORM_VARIABLE);
        m.insert("namespace".to_string(), NORM_TYPE);
        m.insert("decorator".to_string(), NORM_BUILTIN);
        m.insert("modifier".to_string(), NORM_KEYWORD);
        m.insert("regexp".to_string(), NORM_STRING);
        m.insert("event".to_string(), NORM_BUILTIN);
        m.insert("enummember".to_string(), NORM_VARIABLE);
        m.insert("typeparameter".to_string(), NORM_TYPE);
        m
    }

    fn send_request(&mut self, method: &str, params: Value) -> Result<Value, String> {
        let id = self.request_id;
        self.request_id += 1;
        let msg = json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": method,
            "params": params,
        });
        self.write_message(&msg)?;
        self.read_response(id)
    }

    fn send_notification(&mut self, method: &str, params: Value) -> Result<(), String> {
        let msg = json!({
            "jsonrpc": "2.0",
            "method": method,
            "params": params,
        });
        self.write_message(&msg)
    }

    fn write_message(&mut self, msg: &Value) -> Result<(), String> {
        let body = serde_json::to_string(msg)
            .map_err(|e| format!("LSP JSON serialization: {}", e))?;
        let header = format!("Content-Length: {}\r\n\r\n", body.len());
        self.stdin.write_all(header.as_bytes())
            .and_then(|_| self.stdin.write_all(body.as_bytes()))
            .and_then(|_| self.stdin.flush())
            .map_err(|e| format!("LSP write error: {}", e))
    }

    fn read_message(&mut self) -> Result<Value, String> {
        loop {
            let mut content_length: Option<usize> = None;
            loop {
                let mut line = String::new();
                self.reader.read_line(&mut line)
                    .map_err(|e| format!("LSP read error: {}", e))?;
                let trimmed = line.trim_end_matches("\r\n").trim_end_matches('\n');
                if trimmed.is_empty() {
                    break;
                }
                if let Some(len) = trimmed.strip_prefix("Content-Length: ") {
                    content_length = Some(len.parse::<usize>()
                        .map_err(|_| "Invalid Content-Length".to_string())?);
                }
            }
            let len = content_length
                .ok_or_else(|| "Missing Content-Length header".to_string())?;

            let mut buf = vec![0u8; len];
            self.reader.read_exact(&mut buf)
                .map_err(|e| format!("LSP body read error: {}", e))?;
            let body = String::from_utf8(buf)
                .map_err(|_| "Invalid UTF-8 in LSP response".to_string())?;
            let value: Value = serde_json::from_str(&body)
                .map_err(|e| format!("LSP JSON parse: {}", e))?;

            // Only return messages with an id (responses) or method (requests)
            if value.get("id").is_some() || value.get("method").is_some() {
                return Ok(value);
            }
        }
    }

    fn read_response(&mut self, expected_id: u64) -> Result<Value, String> {
        loop {
            let msg = self.read_message()?;
            if let Some(id) = msg.get("id").and_then(|v| v.as_u64()) {
                if id == expected_id {
                    if let Some(error) = msg.get("error") {
                        return Err(format!("LSP error response: {:?}", error));
                    }
                    return Ok(msg.get("result").cloned().unwrap_or(Value::Null));
                }
            }
        }
    }

    fn initialize(&mut self) -> Result<(), String> {
        let params = json!({
            "processId": null,
            "capabilities": {
                "textDocument": {
                    "semanticTokens": {
                        "dynamicRegistration": false,
                        "requests": { "full": { "delta": false } },
                        "tokenTypes": [],
                        "tokenModifiers": []
                    }
                }
            }
        });
        let result = self.send_request("initialize", params)?;

        // Override default mapping with the server's declared token types
        if let Some(legend) = result["capabilities"]["semanticTokensProvider"]["legend"].as_object() {
            if let Some(types) = legend["tokenTypes"].as_array() {
                let mut m = Self::default_normalized_map();
                for (i, v) in types.iter().enumerate() {
                    if let Some(name) = v.as_str() {
                        let lower = name.to_lowercase();
                        // Keep the default mapping but also allow exact index lookup
                        if !m.contains_key(&lower) {
                            m.insert(lower.clone(), NORM_NORMAL);
                        }
                        // Store index-based lookup as well
                        m.insert(format!("__idx_{}", i), m.get(&lower).copied().unwrap_or(NORM_NORMAL));
                    }
                }
                self.type_name_to_normalized = m;
            }
        }

        self.send_notification("initialized", json!({}))?;
        self.initialized = true;
        Ok(())
    }

    /// Open (or update) a virtual document and request semantic tokens.
    /// Returns a list of `LspToken`s with byte-offset positions.
    pub fn get_tokens(&mut self, text: &str, uri: &str, language_id: &str) -> Result<Vec<LspToken>, String> {
        if !self.initialized {
            return Err("LSP client not initialized".to_string());
        }

        self.doc_version += 1;
        if self.doc_version == 1 {
            self.open_document(uri, language_id, text)?;
        } else {
            self.change_document(uri, text, self.doc_version)?;
        }

        let raw_tokens = self.request_semantic_tokens(uri)?;
        Ok(self.convert_offsets(text, &raw_tokens))
    }

    fn open_document(&mut self, uri: &str, language_id: &str, text: &str) -> Result<(), String> {
        let params = json!({
            "textDocument": {
                "uri": uri,
                "languageId": language_id,
                "version": self.doc_version,
                "text": text,
            }
        });
        self.send_notification("textDocument/didOpen", params)
    }

    fn change_document(&mut self, uri: &str, text: &str, version: i64) -> Result<(), String> {
        let params = json!({
            "textDocument": { "uri": uri, "version": version },
            "contentChanges": [{ "text": text }],
        });
        self.send_notification("textDocument/didChange", params)
    }

    fn request_semantic_tokens(&mut self, uri: &str) -> Result<Vec<(u32, u32, u32, u32)>, String> {
        let params = json!({
            "textDocument": { "uri": uri }
        });
        let result = self.send_request("textDocument/semanticTokens/full", params)?;

        let data = match result.get("data") {
            Some(Value::Array(arr)) => arr,
            _ => return Ok(Vec::new()),
        };

        let mut tokens: Vec<(u32, u32, u32, u32)> = Vec::new();
        for chunk in data.chunks(5) {
            if chunk.len() < 5 { break; }
            let delta_line = chunk[0].as_u64().unwrap_or(0) as u32;
            let delta_col = chunk[1].as_u64().unwrap_or(0) as u32;
            let length = chunk[2].as_u64().unwrap_or(0) as u32;
            let type_idx = chunk[3].as_u64().unwrap_or(0) as u32;
            tokens.push((delta_line, delta_col, length, type_idx));
        }
        Ok(tokens)
    }

    /// Convert delta-encoded semantic tokens to byte-offset based tokens.
    fn convert_offsets(&self, text: &str, raw_tokens: &[(u32, u32, u32, u32)]) -> Vec<LspToken> {
        let mut line_starts: Vec<usize> = Vec::new();
        line_starts.push(0);
        for (i, c) in text.char_indices() {
            if c == '\n' {
                line_starts.push(i + 1);
            }
        }

        let mut result: Vec<LspToken> = Vec::new();
        let mut cur_line: u32 = 0;
        let mut cur_col: u32 = 0;

        for &(delta_line, delta_col, len, type_idx) in raw_tokens {
            cur_line += delta_line;
            if delta_line == 0 {
                cur_col += delta_col;
            } else {
                cur_col = delta_col;
            }

            let line_start = *line_starts.get(cur_line as usize).unwrap_or(&text.len());
            let offset = line_start + (cur_col as usize);

            let norm_type = self.normalize_type(type_idx);

            result.push(LspToken {
                offset,
                length: len as usize,
                token_type: norm_type,
            });
        }

        result
    }

    fn normalize_type(&self, type_idx: u32) -> u32 {
        // First try exact index lookup from server's declared legend
        let idx_key = format!("__idx_{}", type_idx);
        if let Some(&norm) = self.type_name_to_normalized.get(&idx_key) {
            return norm;
        }
        // Fall back to name-based lookup — we don't have the name list here,
        // so use a reasonable default based on common legends
        match type_idx {
            10 => NORM_KEYWORD,
            13 | 18 => NORM_STRING,
            17 => NORM_COMMENT,
            14 | 19 => NORM_NUMBER,
            8 | 3 => NORM_VARIABLE,
            12 | 7 => NORM_BUILTIN,
            1 | 2 | 5 => NORM_TYPE,
            0 => NORM_TYPE,    // namespace
            20 => NORM_STRING, // regexp
            16 => NORM_OPERATOR,
            21 => NORM_OPERATOR,
            22 => NORM_BUILTIN,// decorator
            11 => NORM_KEYWORD,// modifier
            9 => NORM_BUILTIN, // macro
            4 | 6 => NORM_VARIABLE, // property, enumMember, event
            15 => NORM_KEYWORD,
            _ => NORM_NORMAL,
        }
    }
}

impl Drop for LspClient {
    fn drop(&mut self) {
        let _ = self.send_request("shutdown", json!({}));
        let _ = self.send_notification("exit", json!({}));
    }
}
