# nooshell

A multi-language REPL notebook and shell. Run interactive REPL sessions for multiple languages in notebook-style workspaces with persistent command history.

## Features

- **Notebook mode** (`noo nbmode`) — TUI with multiple workspaces, each containing notebook cells with independent REPL sessions
- **CLI mode** (`noo`) — single-pane REPL with bash-like arrow-key history navigation
- **Multi-language** — Python, JavaScript (Node.js), configurable via `languages.json`
- **Workspaces** — horizontal tabs, each with its own set of vertically stacked notebook cells
- **Cell management** — add (`Ctrl+T`), remove (`Ctrl+W`), reorder (`Shift+Up`/`Shift+Down`)
- **Command history** — persists across sessions to `%APPDATA%\nooshell\history.json`
- **Session management** — save/restore workspace state; manage from TUI (`Ctrl+M`) or CLI

## Usage

```
noo                 CLI mode
noo nbmode          Notebook TUI
noo manage          Management TUI
noo history         Show command history
noo clearc          Clear command history
noo sessions        List saved sessions
noo delses <id>     Delete a session
```

### Notebook keybindings

| Key | Action |
|-----|--------|
| `Left` / `Right` | Switch workspace |
| `Up` / `Down` | Navigate cells |
| `Shift+Up` / `Shift+Down` | Move cell |
| `Ctrl+T` | New cell |
| `Ctrl+W` | Remove cell |
| `Ctrl+N` | New workspace |
| `Ctrl+Up` / `Ctrl+Down` | History in cell |
| `Enter` | Execute cell |
| `Ctrl+M` | Management TUI |
| `Esc` | Exit |

## Configuration

Edit `languages.json` to add or change language REPLs:

```json
{
  "py": { "cmd": "python", "args": ["-i"], "mode": "repl" },
  "js": { "cmd": "node",  "args": ["-i"], "mode": "repl" }
}
```

## Build

```sh
cargo build --release
```
