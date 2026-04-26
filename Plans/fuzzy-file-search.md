# Plan: Fuzzy File Search Implementation

This plan outlines the steps to add a Fuzzy File Search feature to `terminal-file-explorer`, allowing users to quickly find and jump to files in the current directory.

## Objective
Implement a modal file search (triggered by `Ctrl+F`) that filters files based on user input and jumps to the selected file in the main explorer list.

## Key Files & Context
- `src/menu.rs`: Define new `MenuAction` variants.
- `src/state.rs`: Add logic to jump to a specific path.
- `src/command_palette.rs`: Extend to support file searching mode.
- `src/main.rs`: Integrate the new actions and keybindings.

## Implementation Steps

### 1. Update `MenuAction` (`src/menu.rs`)
- Add `OpenFileSearch` variant.
- Add `JumpToFile(std::path::PathBuf)` variant.

### 2. Update `AppState` (`src/state.rs`)
- Add `select_by_path(&mut self, path: &std::path::Path)` method to `AppState` to find and select a file by its path.

### 3. Extend `CommandPalette` (`src/command_palette.rs`)
- Add `CommandPaletteMode` enum (`Actions`, `Files`).
- Update `CommandPaletteState` to include a `mode` field.
- Add `open_actions(&mut self)` to initialize with menu actions.
- Add `open_files(&mut self, entries: &[FileEntry])` to initialize with files from the current directory.
- Update `update_query` to handle the different modes if necessary (the current logic might already work).

### 4. Update Application Logic (`src/main.rs`)
- **Keybindings**: Map `Ctrl+F` to trigger `MenuAction::OpenFileSearch`.
- **Action Handling**:
    - In `execute_action`, handle `OpenFileSearch` by calling `self.command_palette.open_files(&self.state.entries)`.
    - Handle `JumpToFile(path)` by calling `self.state.select_by_path(&path)`.
- **UI**: Ensure the Command Palette displays correctly in "Files" mode (e.g., different title or category).

## Verification & Testing
- Run the app and press `Ctrl+F`.
- Verify the list contains files from the current directory.
- Type a query and verify the list filters correctly.
- Select a file and press `Enter`.
- Verify the main list selection jumps to that file.
- Verify `Ctrl+P` still works for commands.
