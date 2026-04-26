# Plan: External Opener Implementation

This plan outlines the steps to add an "External Opener" feature, allowing users to open the selected file using the system's default application.

## Objective
Implement a shortcut (`o`) that triggers the system's default handler for the selected file (e.g., opening a `.pdf` in a PDF viewer, a `.png` in an image viewer, etc.).

## Key Files & Context
- `Cargo.toml`: Add the `opener` crate as a dependency.
- `src/menu.rs`: Define the new `MenuAction::ExternalOpen`.
- `src/main.rs`: Integrate the action into the event loop and handle the system call.
- `src/command_palette.rs`: Add the action to the searchable commands.

## Implementation Steps

### 1. Add Dependency (`Cargo.toml`)
- Add `opener = "0.7"` to the `[dependencies]` section.

### 2. Update `MenuAction` (`src/menu.rs`)
- Add `ExternalOpen` variant to the `MenuAction` enum.

### 3. Implement Action Handling (`src/main.rs`)
- In `execute_action`, add a match arm for `MenuAction::ExternalOpen`.
- Use `opener::open(&path)` to trigger the system default application.
- Add an error message or status update if the opening fails.

### 4. Add Keybinding (`src/main.rs`)
- Map `KeyCode::Char('o')` in the main event handler to trigger `MenuAction::ExternalOpen`.

### 5. UI Integration
- **Menu Bar** (`src/menu.rs`): Add "Open with Default App" to the **File** menu.
- **Command Palette** (`src/command_palette.rs`): Add "Open with Default App" to the list of commands.
- **Status Bar** (`src/main.rs`): Add `o:Open` to the shortcut hints.

### 6. Documentation
- Update `README.md` with the new shortcut.
- Update the in-app **Keybindings** help dialog.

## Verification & Testing
- Select various file types (text, image, PDF, etc.) and press `o`.
- Verify the correct system application opens the file.
- Verify directories can also be opened (usually opens the system file explorer at that path).
- Verify the app remains responsive and doesn't crash if the open call fails.
