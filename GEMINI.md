# GEMINI.md

## Project Overview
`terminal-file-explorer` is a Rust-based Terminal User Interface (TUI) application built with `ratatui` and `crossterm`. It provides a feature-rich file browsing experience directly in the terminal, including file operations, previews (text, audio metadata, images), a built-in editor with undo/redo, and multiple color themes.

### Key Technologies
- **Language:** Rust (Edition 2021)
- **TUI Framework:** [ratatui](https://crates.io/crates/ratatui)
- **Terminal Backend:** [crossterm](https://crates.io/crates/crossterm)
- **Serialization:** [serde](https://crates.io/crates/serde), [toml](https://crates.io/crates/toml)
- **Utilities:** [chrono](https://crates.io/crates/chrono) (time), [id3](https://crates.io/crates/id3) (audio metadata), [directories](https://crates.io/crates/directories) (config paths)
- **Task Runner:** `package.json` (managed via `bun`)

### Architecture
- `main.rs`: Entry point, application loop, event handling, and UI rendering logic.
- `state.rs`: Core application state management (`AppState`, `FileEntry`).
- `config.rs`: Configuration loading and saving (`config.toml`).
- `fs.rs`: File system operation abstractions.
- `theme.rs`: Color theme definitions and application.
- `preview.rs`: Logic for generating file previews.
- `menu.rs`: Menu bar state and rendering.
- `command_palette.rs`: Command palette implementation.
- `git.rs`: Git status integration.

---

## Building and Running

### Prerequisites
- [Rust](https://www.rust-lang.org/tools/install) (latest stable)
- [Bun](https://bun.sh/) (optional, for running scripts)

### Commands
| Task | Command |
|------|---------|
| **Build (Release)** | `cargo build --release` or `bun run build` |
| **Run (Dev)** | `cargo run` or `bun run dev` |
| **Format** | `cargo fmt --all` or `bun run format` |
| **Lint/Fix** | `cargo fix` or `bun run fix` |
| **Check** | `cargo check` or `bun run check` |

---

## Development Conventions

### Code Style
- Follow standard Rust naming conventions (`snake_case` for variables/functions, `PascalCase` for types).
- Use `cargo fmt` regularly to maintain consistent formatting.
- Avoid `unwrap()` on `Option` or `Result` in library code; prefer `expect()` with a descriptive message or proper error handling.

### State Management
- The `AppState` struct in `state.rs` is the source of truth for the current directory and selection.
- UI components should be pure functions that take a reference to the state/theme and a `Frame`.

### UI/UX
- Support both keyboard and mouse interactions where applicable.
- Ensure all new keybindings are documented in the `README.md` and added to the in-app help (if applicable).
- New themes should be added to `theme.rs` and the theme names list in `main.rs`.

### Configuration
- Persistent settings are stored in `~/.config/terminal-file-explorer/config.toml` (on Linux) or equivalent folders on other OSs.
- Always use `Config::load()` to initialize settings.

---

## Workspace Layout
- `src/`: Rust source code.
- `target/`: Build artifacts.
- `.kilo/`: Context for internal tools (e.g., Kilo editor integration).
- `package.json`: Script definitions for the developer workflow.
