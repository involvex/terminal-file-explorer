# Terminal File Explorer Agent Guide

## Development Commands

- Build: `cargo build --release`
- Run: `cargo run`
- Test: No test suite currently configured

## Project Structure

- Entry point: `src/main.rs`
- Core modules: `config`, `fs`, `preview`, `state`, `theme`
- Configuration: Stored at `~/.config/terminal-file-explorer/config.toml`

## Notable Implementation Details

- Uses ratatui v0.28 for terminal UI
- Uses crossterm v0.28 for terminal event handling
- Theme switching persists to config file
- Editor supports basic undo/redo functionality
- File operations (create, delete, rename) use dialog confirmations

## Configuration Format

The config.toml file uses:

```toml
theme = "tokyo-night"
show_hidden = false
preview_enabled = true
```

## Key Non-Obvious Behaviors

- Ctrl+T cycles through 8 predefined themes
- Ctrl+P toggles hidden file visibility
- Tab toggles preview pane
- Ctrl+O cycles sort order (name, size, date)
- F5 refreshes directory listing
- In editor: Ctrl+S saves, Ctrl+Z undoes, Ctrl+Y redoes
