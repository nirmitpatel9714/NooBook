use crate::lsp::LspToken;
use ratatui::style::{Color, Style};
use ratatui::text::Span;
use std::collections::HashSet;
use std::sync::LazyLock;

// ── Token kinds ──

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TokenKind {
    Keyword,
    Builtin,
    String,
    Comment,
    Number,
    Variable,
    Normal,
}

impl TokenKind {
    pub fn style(&self) -> Style {
        match self {
            TokenKind::Keyword => Style::default().fg(Color::Blue),
            TokenKind::Builtin => Style::default().fg(Color::Cyan),
            TokenKind::String => Style::default().fg(Color::Green),
            TokenKind::Comment => Style::default().fg(Color::DarkGray),
            TokenKind::Number => Style::default().fg(Color::Yellow),
            TokenKind::Variable => Style::default().fg(Color::Magenta),
            TokenKind::Normal => Style::default(),
        }
    }
}

struct Token {
    text: String,
    kind: TokenKind,
}

// ── Language keyword / built-in tables ──

macro_rules! str_set {
    ($($item:expr),* $(,)?) => {{
        let mut s = HashSet::new();
        $(s.insert($item.to_string());)*
        s
    }};
}

static PY_KEYWORDS: LazyLock<HashSet<String>> = LazyLock::new(|| {
    str_set![
        "False", "None", "True", "and", "as", "assert", "async", "await", "break", "class",
        "continue", "def", "del", "elif", "else", "except", "finally", "for", "from", "global",
        "if", "import", "in", "is", "lambda", "nonlocal", "not", "or", "pass", "raise", "return",
        "try", "while", "with", "yield",
    ]
});

static PY_BUILTINS: LazyLock<HashSet<String>> = LazyLock::new(|| {
    str_set![
        "abs",
        "all",
        "any",
        "bin",
        "bool",
        "bytearray",
        "bytes",
        "chr",
        "complex",
        "dict",
        "dir",
        "divmod",
        "enumerate",
        "eval",
        "exec",
        "filter",
        "float",
        "format",
        "frozenset",
        "getattr",
        "globals",
        "hasattr",
        "hash",
        "help",
        "hex",
        "id",
        "input",
        "int",
        "isinstance",
        "issubclass",
        "iter",
        "len",
        "list",
        "locals",
        "map",
        "max",
        "memoryview",
        "min",
        "next",
        "object",
        "oct",
        "open",
        "ord",
        "pow",
        "print",
        "property",
        "range",
        "repr",
        "reversed",
        "round",
        "set",
        "setattr",
        "slice",
        "sorted",
        "staticmethod",
        "str",
        "sum",
        "super",
        "tuple",
        "type",
        "vars",
        "zip",
        "__import__",
    ]
});

static JS_KEYWORDS: LazyLock<HashSet<String>> = LazyLock::new(|| {
    str_set![
        "async",
        "await",
        "break",
        "case",
        "catch",
        "class",
        "const",
        "continue",
        "debugger",
        "default",
        "delete",
        "do",
        "else",
        "enum",
        "export",
        "extends",
        "false",
        "finally",
        "for",
        "function",
        "if",
        "import",
        "in",
        "instanceof",
        "let",
        "new",
        "null",
        "of",
        "return",
        "super",
        "switch",
        "this",
        "throw",
        "true",
        "try",
        "typeof",
        "var",
        "void",
        "while",
        "with",
        "yield",
    ]
});

static JS_BUILTINS: LazyLock<HashSet<String>> = LazyLock::new(|| {
    str_set![
        "Array",
        "Boolean",
        "Date",
        "Error",
        "Function",
        "Infinity",
        "JSON",
        "Math",
        "NaN",
        "Number",
        "Object",
        "Promise",
        "RangeError",
        "ReferenceError",
        "RegExp",
        "String",
        "SyntaxError",
        "TypeError",
        "URIError",
        "console",
        "decodeURI",
        "decodeURIComponent",
        "encodeURI",
        "encodeURIComponent",
        "escape",
        "eval",
        "global",
        "isFinite",
        "isNaN",
        "parseFloat",
        "parseInt",
        "undefined",
        "unescape",
        "process",
        "require",
        "module",
        "exports",
        "setTimeout",
        "setInterval",
        "clearTimeout",
        "clearInterval",
    ]
});

static PS_KEYWORDS: LazyLock<HashSet<String>> = LazyLock::new(|| {
    let mut s = HashSet::new();
    for w in [
        "begin",
        "break",
        "catch",
        "continue",
        "data",
        "do",
        "dynamicparam",
        "else",
        "elseif",
        "end",
        "exit",
        "filter",
        "finally",
        "for",
        "foreach",
        "from",
        "function",
        "if",
        "in",
        "param",
        "process",
        "return",
        "switch",
        "throw",
        "trap",
        "try",
        "until",
        "using",
        "var",
        "while",
    ] {
        s.insert(w.to_string());
    }
    s.insert("$true".to_string());
    s.insert("$false".to_string());
    s.insert("$null".to_string());
    s
});

static PS_BUILTINS: LazyLock<HashSet<String>> = LazyLock::new(|| {
    let mut s = HashSet::new();
    for w in [
        "Write-Host",
        "Write-Output",
        "Write-Error",
        "Write-Warning",
        "Write-Verbose",
        "Write-Debug",
        "Write-Progress",
        "Get-ChildItem",
        "Get-Content",
        "Get-Process",
        "Get-Service",
        "Get-Command",
        "Get-Help",
        "Get-Member",
        "Get-Item",
        "Get-Location",
        "Get-Date",
        "Get-Variable",
        "Get-ItemProperty",
        "Set-Location",
        "Set-Content",
        "Set-Item",
        "Set-Variable",
        "Add-Content",
        "New-Item",
        "New-Object",
        "New-Variable",
        "Remove-Item",
        "Remove-Variable",
        "Copy-Item",
        "Move-Item",
        "Rename-Item",
        "Test-Path",
        "Join-Path",
        "Split-Path",
        "ConvertTo-Json",
        "ConvertFrom-Json",
        "Where-Object",
        "Select-Object",
        "ForEach-Object",
        "Sort-Object",
        "Group-Object",
        "Measure-Object",
        "Compare-Object",
        "Format-Table",
        "Format-List",
        "Format-Wide",
        "Format-Custom",
        "Out-File",
        "Out-Null",
        "Out-String",
        "Out-Default",
        "Import-Module",
        "Export-ModuleMember",
        "Start-Process",
        "Stop-Process",
        "Start-Sleep",
        "Resolve-Path",
        "Clear-Host",
        "Write-Host",
    ] {
        s.insert(w.to_string());
    }
    s
});

fn default_set() -> HashSet<String> {
    HashSet::new()
}
static EMPTY_SET: LazyLock<HashSet<String>> = LazyLock::new(default_set);

fn get_keywords(lang: &str) -> &'static HashSet<String> {
    match lang {
        "py" => &PY_KEYWORDS,
        "js" => &JS_KEYWORDS,
        "ps" => &PS_KEYWORDS,
        _ => &EMPTY_SET,
    }
}

fn get_builtins(lang: &str) -> &'static HashSet<String> {
    match lang {
        "py" => &PY_BUILTINS,
        "js" => &JS_BUILTINS,
        "ps" => &PS_BUILTINS,
        _ => &EMPTY_SET,
    }
}

// ── Tokenizer ──

fn classify_word(word: &str, keywords: &HashSet<String>, builtins: &HashSet<String>) -> TokenKind {
    if word.is_empty() {
        return TokenKind::Normal;
    }
    if keywords.contains(word) {
        return TokenKind::Keyword;
    }
    if builtins.contains(word) {
        return TokenKind::Builtin;
    }
    if word.starts_with(|c: char| c.is_ascii_digit()) {
        return TokenKind::Number;
    }
    TokenKind::Normal
}

fn is_word_char(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

fn is_word_start(c: char) -> bool {
    c.is_alphabetic() || c == '_'
}

fn tokenize(text: &str, lang: &str) -> Vec<Token> {
    let keywords = get_keywords(lang);
    let builtins = get_builtins(lang);
    let mut tokens: Vec<Token> = Vec::new();
    let mut chars = text.chars().peekable();

    macro_rules! push {
        ($s:expr, $k:expr) => {
            tokens.push(Token {
                text: $s.to_string(),
                kind: $k,
            })
        };
    }

    while let Some(ch) = chars.next() {
        // ── Line comments ──
        if ch == '#' && (lang == "py" || lang == "ps") {
            let mut s = String::new();
            s.push(ch);
            while let Some(&c) = chars.peek() {
                if c == '\n' {
                    break;
                }
                s.push(c);
                chars.next();
            }
            push!(s, TokenKind::Comment);
            continue;
        }
        if ch == '/' && chars.peek() == Some(&'/') && lang == "js" {
            let mut s = String::new();
            s.push_str("//");
            chars.next();
            while let Some(&c) = chars.peek() {
                if c == '\n' {
                    break;
                }
                s.push(c);
                chars.next();
            }
            push!(s, TokenKind::Comment);
            continue;
        }

        // ── PowerShell variables ──
        if ch == '$' && lang == "ps" {
            let mut s = String::new();
            s.push('$');
            // ${...} syntax
            if chars.peek() == Some(&'{') {
                s.push('{');
                chars.next();
                while let Some(&c) = chars.peek() {
                    if c == '}' {
                        break;
                    }
                    s.push(c);
                    chars.next();
                }
                if chars.peek() == Some(&'}') {
                    s.push('}');
                    chars.next();
                }
            } else {
                while let Some(&c) = chars.peek() {
                    if is_word_char(c) || c == ':' {
                        s.push(c);
                        chars.next();
                    } else {
                        break;
                    }
                }
            }
            if s.len() > 1 {
                // check for $true/$false/$null
                if keywords.contains(s.as_str()) {
                    push!(s, TokenKind::Keyword);
                } else {
                    push!(s, TokenKind::Variable);
                }
            } else {
                push!(s, TokenKind::Normal);
            }
            continue;
        }

        // ── PowerShell parameters (-ParamName) ──
        if ch == '-' && lang == "ps" {
            let mut s = String::new();
            s.push('-');
            if chars.peek().is_some_and(|&c| c.is_alphabetic()) {
                while let Some(&c) = chars.peek() {
                    if is_word_char(c) || c == ':' {
                        s.push(c);
                        chars.next();
                    } else {
                        break;
                    }
                }
            }
            push!(s, TokenKind::Normal);
            continue;
        }

        // ── Strings ──
        if ch == '\'' {
            let mut s = String::new();
            s.push('\'');
            // Check triple single quotes (Python)
            let triple =
                lang == "py" && chars.peek() == Some(&'\'') && chars.clone().nth(1) == Some('\'');
            if triple {
                s.push_str("''");
                chars.next();
                chars.next();
                loop {
                    match chars.next() {
                        Some('\'')
                            if chars.peek() == Some(&'\'')
                                && chars.clone().nth(1) == Some('\'') =>
                        {
                            s.push_str("'''");
                            chars.next();
                            chars.next();
                            break;
                        }
                        Some(c) => s.push(c),
                        None => break,
                    }
                }
            } else {
                loop {
                    match chars.next() {
                        Some('\'') => {
                            s.push('\'');
                            break;
                        }
                        Some('\\') => {
                            s.push('\\');
                            if let Some(esc) = chars.next() {
                                s.push(esc);
                            }
                        }
                        Some(c) => s.push(c),
                        None => break,
                    }
                }
            }
            push!(s, TokenKind::String);
            continue;
        }

        if ch == '"' {
            let mut s = String::new();
            s.push('"');
            // Check triple double quotes (Python)
            let triple =
                lang == "py" && chars.peek() == Some(&'"') && chars.clone().nth(1) == Some('"');
            if triple {
                s.push_str("\"\"");
                chars.next();
                chars.next();
                loop {
                    match chars.next() {
                        Some('"')
                            if chars.peek() == Some(&'"') && chars.clone().nth(1) == Some('"') =>
                        {
                            s.push_str("\"\"\"");
                            chars.next();
                            chars.next();
                            break;
                        }
                        Some(c) => s.push(c),
                        None => break,
                    }
                }
            } else {
                loop {
                    match chars.next() {
                        Some('"') => {
                            s.push('"');
                            break;
                        }
                        Some('\\') => {
                            s.push('\\');
                            if let Some(esc) = chars.next() {
                                s.push(esc);
                            }
                        }
                        Some(c) => s.push(c),
                        None => break,
                    }
                }
            }
            push!(s, TokenKind::String);
            continue;
        }

        // ── Template literal (backtick) for JS ──
        if ch == '`' && lang == "js" {
            let mut s = String::new();
            s.push('`');
            loop {
                match chars.next() {
                    Some('`') => {
                        s.push('`');
                        break;
                    }
                    Some('\\') => {
                        s.push('\\');
                        if let Some(esc) = chars.next() {
                            s.push(esc);
                        }
                    }
                    Some(c) => s.push(c),
                    None => break,
                }
            }
            push!(s, TokenKind::String);
            continue;
        }

        // ── Identifiers / words ──
        if is_word_start(ch) {
            let mut word = String::new();
            word.push(ch);
            while let Some(&c) = chars.peek() {
                if is_word_char(c) {
                    word.push(c);
                    chars.next();
                } else {
                    break;
                }
            }
            // Check for PowerShell verb-noun pattern
            if lang == "ps" && chars.peek() == Some(&'-') {
                // Could be a cmdlet like Write-Host
                let saved = chars.clone();
                chars.next(); // consume '-'
                if chars.peek().is_some_and(|&c| c.is_alphabetic()) {
                    word.push('-');
                    while let Some(&c) = chars.peek() {
                        if is_word_char(c) {
                            word.push(c);
                            chars.next();
                        } else {
                            break;
                        }
                    }
                    if builtins.contains(word.as_str()) {
                        push!(word, TokenKind::Builtin);
                        continue;
                    }
                } else {
                    // Not a verb-noun, put back '-'
                    chars = saved;
                }
            }
            let kind = classify_word(&word, keywords, builtins);
            push!(word, kind);
            continue;
        }

        // ── Numbers (starting with digit) ──
        if ch.is_ascii_digit() {
            let mut num = String::new();
            num.push(ch);
            while let Some(&c) = chars.peek() {
                if c.is_ascii_digit()
                    || c == '.'
                    || c == 'e'
                    || c == 'E'
                    || c == 'x'
                    || c == 'X'
                    || c == 'o'
                    || c == 'O'
                    || c == 'b'
                    || c == 'B'
                    || c == '_'
                {
                    num.push(c);
                    chars.next();
                } else {
                    break;
                }
            }
            push!(num, TokenKind::Number);
            continue;
        }

        // ── JS regex literal (starts with /, but we need to be careful) ──
        // skip for now, too complex

        // ── Operators / punctuation ──
        let mut op = String::new();
        op.push(ch);
        // Multi-char operators
        if matches!(
            ch,
            '=' | '!' | '<' | '>' | '&' | '|' | '+' | '-' | '*' | '/' | '%' | '^' | '~'
        ) && let Some(&next) = chars.peek()
        {
            let pair = format!("{}{}", ch, next);
            if matches!(
                pair.as_str(),
                "==" | "!="
                    | "<="
                    | ">="
                    | "&&"
                    | "||"
                    | "++"
                    | "--"
                    | "=>"
                    | "+="
                    | "-="
                    | "*="
                    | "/="
                    | "%="
                    | "&="
                    | "|="
                    | "^="
                    | "<<"
                    | ">>"
                    | "->"
                    | "::"
            ) {
                op.push(next);
                chars.next();
            }
        }
        push!(op, TokenKind::Normal);
    }

    tokens
}

/// Lightweight language detection for highlighting purposes.
/// When `lang` is `"auto"`, guesses the language from the first token.
fn resolve_lang<'a>(text: &'a str, lang: &'a str) -> &'a str {
    if lang != "auto" {
        return lang;
    }
    let trimmed = text.trim_start();
    if trimmed.is_empty() {
        return "auto";
    }
    // Python
    if trimmed.starts_with("print(")
        || trimmed.starts_with("print ")
        || trimmed.starts_with("import ")
        || trimmed.starts_with("from ")
        || trimmed.starts_with("def ")
        || trimmed.starts_with("class ")
        || trimmed.starts_with("elif ")
        || trimmed.starts_with("else:")
        || trimmed.starts_with("except")
        || trimmed.starts_with("finally")
        || trimmed.starts_with("with ")
        || trimmed.starts_with("try:")
        || trimmed.starts_with("return ")
        || trimmed.starts_with("yield ")
        || trimmed.starts_with("raise ")
        || trimmed.starts_with("assert ")
        || trimmed.starts_with("pass")
        || trimmed.starts_with("break")
        || trimmed.starts_with("continue")
        || trimmed.starts_with("@")
        || trimmed.starts_with("lambda ")
        || trimmed.starts_with("del ")
        || trimmed.starts_with("async ")
        || trimmed.starts_with("await ")
        || trimmed.starts_with("global ")
        || trimmed.starts_with("nonlocal ")
    {
        return "py";
    }
    // JavaScript
    if trimmed.starts_with("console.")
        || trimmed.starts_with("function ")
        || trimmed.starts_with("const ")
        || trimmed.starts_with("let ")
        || trimmed.starts_with("var ")
        || trimmed.starts_with("typeof ")
        || trimmed.starts_with("instanceof ")
        || trimmed.starts_with("require(")
        || trimmed.starts_with("setTimeout(")
        || trimmed.starts_with("setInterval(")
        || trimmed.starts_with("clearTimeout(")
        || trimmed.starts_with("async ")
        || trimmed.starts_with("await ")
        || trimmed.starts_with("throw ")
        || trimmed.starts_with("delete ")
        || trimmed.starts_with("export ")
        || trimmed.starts_with("import ")
        || trimmed.starts_with("class ")
        || trimmed.starts_with("new ")
        || trimmed.starts_with("try ")
        || trimmed.starts_with("catch ")
        || trimmed.starts_with("finally ")
        || trimmed.starts_with("switch ")
        || trimmed.starts_with("case ")
        || trimmed.starts_with("default:")
        || trimmed.starts_with("debugger")
        || trimmed.starts_with("do ")
        || trimmed.starts_with("typeof(")
    {
        return "js";
    }
    // PowerShell
    if trimmed.starts_with('$')
        || trimmed.starts_with("Write-")
        || trimmed.starts_with("Get-")
        || trimmed.starts_with("Set-")
        || trimmed.starts_with("New-")
        || trimmed.starts_with("Remove-")
        || trimmed.starts_with("Start-")
        || trimmed.starts_with("Stop-")
        || trimmed.starts_with("Where-Object")
        || trimmed.starts_with("ForEach-")
        || trimmed.starts_with("Format-")
        || trimmed.starts_with("Out-")
        || trimmed.starts_with("Import-")
        || trimmed.starts_with("Export-")
        || trimmed.starts_with("Add-")
        || trimmed.starts_with("Copy-")
        || trimmed.starts_with("Move-")
        || trimmed.starts_with("Rename-")
        || trimmed.starts_with("Test-Path")
        || trimmed.starts_with("ConvertTo-")
        || trimmed.starts_with("ConvertFrom-")
        || trimmed.starts_with("Select-Object")
        || trimmed.starts_with("Sort-Object")
        || trimmed.starts_with("Group-Object")
        || trimmed.starts_with("Measure-Object")
        || trimmed.starts_with("Clear-Host")
        || trimmed.starts_with("Write-Host")
        || trimmed.starts_with("Start-Sleep")
        || trimmed.starts_with("New-Object")
        || trimmed.starts_with("Write-Output")
        || trimmed.starts_with("Write-Error")
    {
        return "ps";
    }
    // Check for `===` or `!==` (strong JS indicator)
    if trimmed.contains("===") || trimmed.contains("!==") {
        return "js";
    }
    // Check for `=>` (arrow function)
    if trimmed.contains("=>") && !trimmed.starts_with('=') {
        return "js";
    }
    // Check for PowerShell operators
    if trimmed.contains(" -eq ")
        || trimmed.contains(" -ne ")
        || trimmed.contains(" -gt ")
        || trimmed.contains(" -lt ")
        || trimmed.contains(" -ge ")
        || trimmed.contains(" -le ")
        || trimmed.contains(" -like ")
        || trimmed.contains(" -match ")
    {
        return "ps";
    }
    // Try matching the first identifier word against keyword sets
    let first_word = trimmed
        .split(|c: char| !c.is_alphanumeric() && c != '_')
        .next()
        .unwrap_or("");
    if !first_word.is_empty() {
        if PY_KEYWORDS.contains(first_word) || PY_BUILTINS.contains(first_word) {
            return "py";
        }
        if JS_KEYWORDS.contains(first_word) || JS_BUILTINS.contains(first_word) {
            return "js";
        }
        // Lowercase first word for PowerShell keyword check
        let lower = first_word.to_lowercase();
        if PS_KEYWORDS.contains(&lower) {
            return "ps";
        }
    }
    // Fall back to the original lang
    "auto"
}

fn color_to_ansi(color: Color) -> &'static str {
    match color {
        Color::Reset => "\x1b[0m",
        Color::Black => "\x1b[30m",
        Color::Red => "\x1b[31m",
        Color::Green => "\x1b[32m",
        Color::Yellow => "\x1b[33m",
        Color::Blue => "\x1b[34m",
        Color::Magenta => "\x1b[35m",
        Color::Cyan => "\x1b[36m",
        Color::White => "\x1b[37m",
        Color::DarkGray => "\x1b[90m",
        Color::LightRed => "\x1b[91m",
        Color::LightGreen => "\x1b[92m",
        Color::LightYellow => "\x1b[93m",
        Color::LightBlue => "\x1b[94m",
        Color::LightMagenta => "\x1b[95m",
        Color::LightCyan => "\x1b[96m",
        _ => "\x1b[0m",
    }
}

/// Apply syntax highlighting to text and return (before_cursor, after_cursor)
/// strings with ANSI escape codes for CLI mode.
pub fn highlight_ansi(text: &str, lang: &str, cursor: usize) -> (String, String) {
    let resolved = resolve_lang(text, lang);
    let tokens = tokenize(text, resolved);
    let mut before = String::new();
    let mut after = String::new();
    let mut pos = 0;
    let reset = "\x1b[0m";

    let ansi_for = |kind: TokenKind| -> &'static str {
        match kind.style().fg {
            Some(c) => color_to_ansi(c),
            None => reset,
        }
    };

    for token in tokens {
        let end = pos + token.text.len();
        if end <= cursor {
            let ansi = ansi_for(token.kind);
            before.push_str(ansi);
            before.push_str(&token.text);
            before.push_str(reset);
        } else if pos >= cursor {
            let ansi = ansi_for(token.kind);
            after.push_str(ansi);
            after.push_str(&token.text);
            after.push_str(reset);
        } else {
            let split_at = cursor - pos;
            if split_at < token.text.len() && token.text.is_char_boundary(split_at) {
                let (left, right) = token.text.split_at(split_at);
                let ansi = ansi_for(token.kind);
                if !left.is_empty() {
                    before.push_str(ansi);
                    before.push_str(left);
                    before.push_str(reset);
                }
                if !right.is_empty() {
                    after.push_str(ansi);
                    after.push_str(right);
                    after.push_str(reset);
                }
            } else {
                let ansi = ansi_for(token.kind);
                before.push_str(ansi);
                before.push_str(&token.text);
                before.push_str(reset);
            }
        }
        pos = end;
    }

    (before, after)
}

/// Split the input text into syntax-highlighted spans at the given cursor
/// position (byte index). Returns `(before_cursor, after_cursor)` spans.
pub fn highlight_split(
    text: &str,
    lang: &str,
    cursor: usize,
) -> (Vec<Span<'static>>, Vec<Span<'static>>) {
    let resolved = resolve_lang(text, lang);
    let tokens = tokenize(text, resolved);
    let mut before: Vec<Span<'static>> = Vec::new();
    let mut after: Vec<Span<'static>> = Vec::new();
    let mut pos = 0;

    for token in tokens {
        let end = pos + token.text.len();
        if end <= cursor {
            before.push(Span::styled(token.text, token.kind.style()));
        } else if pos >= cursor {
            after.push(Span::styled(token.text, token.kind.style()));
        } else {
            let split_at = cursor - pos;
            if split_at < token.text.len() && token.text.is_char_boundary(split_at) {
                let (left, right) = token.text.split_at(split_at);
                if !left.is_empty() {
                    before.push(Span::styled(left.to_string(), token.kind.style()));
                }
                if !right.is_empty() {
                    after.push(Span::styled(right.to_string(), token.kind.style()));
                }
            } else {
                before.push(Span::styled(token.text, token.kind.style()));
            }
        }
        pos = end;
    }

    (before, after)
}

/// Map an LSP normalized token type to a style.
fn lsp_kind_style(norm_type: u32) -> Style {
    match norm_type {
        crate::lsp::NORM_KEYWORD => Style::default().fg(Color::Blue),
        crate::lsp::NORM_BUILTIN => Style::default().fg(Color::Cyan),
        crate::lsp::NORM_STRING => Style::default().fg(Color::Green),
        crate::lsp::NORM_COMMENT => Style::default().fg(Color::DarkGray),
        crate::lsp::NORM_NUMBER => Style::default().fg(Color::Yellow),
        crate::lsp::NORM_VARIABLE => Style::default().fg(Color::Magenta),
        crate::lsp::NORM_TYPE => Style::default().fg(Color::LightCyan),
        crate::lsp::NORM_PARAMETER => Style::default().fg(Color::LightMagenta),
        crate::lsp::NORM_OPERATOR => Style::default().fg(Color::White),
        _ => Style::default(),
    }
}

/// Build a flat list of `(offset, length, style)` segments from LSP tokens,
/// filling gaps with `Normal` style.
fn build_lsp_segments(text: &str, lsp_tokens: &[LspToken]) -> Vec<(usize, usize, Style)> {
    let mut segments: Vec<(usize, usize, Style)> = Vec::new();
    let mut pos = 0usize;
    for token in lsp_tokens {
        if token.offset > pos {
            segments.push((pos, token.offset - pos, Style::default()));
        }
        segments.push((token.offset, token.length, lsp_kind_style(token.token_type)));
        pos = token.offset + token.length;
    }
    if pos < text.len() {
        segments.push((pos, text.len() - pos, Style::default()));
    }
    segments
}

/// Apply LSP-based syntax highlighting to text and return ANSI-escaped
/// (before_cursor, after_cursor) strings.
pub fn highlight_ansi_lsp(
    text: &str,
    _lang: &str,
    cursor: usize,
    lsp_tokens: &[LspToken],
) -> (String, String) {
    let reset = "\x1b[0m";
    let segments = build_lsp_segments(text, lsp_tokens);
    let mut before = String::new();
    let mut after = String::new();

    for &(offset, length, style) in &segments {
        let end = offset + length;
        if end <= cursor {
            before.push_str(&style_text(&text[offset..end], style, reset));
        } else if offset >= cursor {
            after.push_str(&style_text(&text[offset..end], style, reset));
        } else {
            let split_at = cursor - offset;
            if split_at < length && text.is_char_boundary(offset + split_at) {
                before.push_str(&style_text(&text[offset..offset + split_at], style, reset));
                after.push_str(&style_text(&text[offset + split_at..end], style, reset));
            } else {
                before.push_str(&style_text(&text[offset..end], style, reset));
            }
        }
    }

    (before, after)
}

fn style_text(s: &str, style: Style, reset: &str) -> String {
    match style.fg {
        Some(c) => format!("{}{}{}", color_to_ansi(c), s, reset),
        None => s.to_string(),
    }
}

/// Apply LSP-based syntax highlighting and return `(before_cursor, after_cursor)`
/// as ratatui `Span` vectors.
pub fn highlight_split_lsp(
    text: &str,
    _lang: &str,
    cursor: usize,
    lsp_tokens: &[LspToken],
) -> (Vec<Span<'static>>, Vec<Span<'static>>) {
    let segments = build_lsp_segments(text, lsp_tokens);
    let mut before: Vec<Span<'static>> = Vec::new();
    let mut after: Vec<Span<'static>> = Vec::new();

    for &(offset, length, style) in &segments {
        let end = offset + length;
        if end <= cursor {
            before.push(Span::styled(text[offset..end].to_string(), style));
        } else if offset >= cursor {
            after.push(Span::styled(text[offset..end].to_string(), style));
        } else {
            let split_at = cursor - offset;
            if split_at < length && text.is_char_boundary(offset + split_at) {
                before.push(Span::styled(
                    text[offset..offset + split_at].to_string(),
                    style,
                ));
                after.push(Span::styled(
                    text[offset + split_at..end].to_string(),
                    style,
                ));
            } else {
                before.push(Span::styled(text[offset..end].to_string(), style));
            }
        }
    }

    (before, after)
}
