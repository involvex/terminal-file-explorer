# Terminal File Explorer

A terminal file explorer built with Rust and ratatui.

## Features

- **File browsing** - Navigate directories with arrow keys or mouse
- **File operations** - Create, delete, rename files and folders
- **File preview** - Preview text files, audio metadata, and images
- **Built-in editor** - Edit files with undo/redo support
- **8 color themes** - Tokyo Night, Dracula, Nord, Solarized Dark, Catppuccin Mocha, One Dark, Monokai, Gruvbox Dark
- **Sorting** - Sort by name, size, or modified date

## Keybindings

| Key | Action |
|-----|--------|
| `↑ / ↓` | Navigate files |
| `Enter` | Open directory / Go back on `..` |
| `Backspace` or `←` | Go to parent directory |
| `Tab` | Toggle preview pane |
| `Ctrl+E` | Open file in editor |
| `Ctrl+S` | Save (in editor) |
| `Ctrl+Z` | Undo (in editor) |
| `Ctrl+Y` | Redo (in editor) |
| `Ctrl+T` | Cycle theme |
| `Ctrl+O` | Cycle sort order |
| `Ctrl+P` | Toggle hidden files |
| `Ctrl+N` | Create new file |
| `Ctrl+R` | Rename file/folder |
| `Delete` | Delete file/folder |
| `F5` | Refresh directory |
| `Esc` | Close dialog/editor |
| `Ctrl+C` | Quit |

### Mouse

- **Click** - Select file
- **Scroll** - Navigate file list

## Configuration

Config is stored at `~/.config/terminal-file-explorer/config.toml`:

```toml
theme = "tokyo-night"
show_hidden = false
preview_enabled = true
```

## Build

```bash
cargo build --release
```

## Run

```bash
cargo run
```

## Themes

- Tokyo Night
- Dracula
- Nord
- Solarized Dark
- Catppuccin Mocha
- One Dark
- Monokai
- Gruvbox Dark
