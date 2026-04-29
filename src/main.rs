mod command_palette;
mod config;
mod fs;
mod git;
mod menu;
mod preview;
mod state;
mod theme;

use command_palette::{CommandPaletteResult, CommandPaletteState};
use config::Config;
use crossterm::{
    event::{
        DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEvent, KeyEventKind,
        MouseEvent, MouseEventKind,
    },
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, SetTitle},
};
use git::{FileGitStatus, GitStatus};
use menu::{MenuAction, MenuBarState};
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::Style;
use ratatui::widgets::{Block, Borders, List, ListItem, ListState, Paragraph};
use ratatui::{backend::CrosstermBackend, Frame, Terminal};
use state::{AppState, SortOrder};
use std::env;
use std::path::PathBuf;
use syntect::highlighting::ThemeSet;
use syntect::parsing::SyntaxSet;
use theme::Theme;

struct App {
    state: AppState,
    config: Config,
    theme: Theme,
    dialog: DialogState,
    editor_content: Vec<String>,
    editor_path: Option<PathBuf>,
    editor_modified: bool,
    in_editor: bool,
    cursor_line: usize,
    cursor_col: usize,
    scroll_offset: usize,
    undo_stack: Vec<Vec<String>>,
    redo_stack: Vec<Vec<String>>,
    git_status: Option<GitStatus>,
    menu_bar: MenuBarState,
    command_palette: CommandPaletteState,
    show_about: bool,
    show_keybindings: bool,
    clipboard: Vec<PathBuf>,
    clipboard_cut: bool,
    syntax_set: SyntaxSet,
    theme_set: ThemeSet,
}

impl App {
    fn new(cwd: PathBuf) -> Self {
        let config = Config::load();
        let theme_name = &config.theme;
        let theme = Theme::from_name(theme_name)
            .unwrap_or_else(|| Theme::from_name("tokyo-night").unwrap());

        let mut state = AppState::new(cwd);
        state.show_hidden = config.show_hidden;
        state.calculate_dir_size = config.calculate_dir_size;
        state.load_dir();
        let git_status = GitStatus::get_for_path(&state.cwd);

        Self {
            state,
            config,
            theme,
            dialog: DialogState::default(),
            editor_content: Vec::new(),
            editor_path: None,
            editor_modified: false,
            in_editor: false,
            cursor_line: 0,
            cursor_col: 0,
            scroll_offset: 0,
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            git_status,
            menu_bar: MenuBarState::new(),
            command_palette: CommandPaletteState::new(),
            show_about: false,
            show_keybindings: false,
            clipboard: Vec::new(),
            clipboard_cut: false,
            syntax_set: SyntaxSet::load_defaults_newlines(),
            theme_set: ThemeSet::load_defaults(),
        }
    }

    fn refresh_git_status(&mut self) {
        self.git_status = GitStatus::get_for_path(&self.state.cwd);
    }

    fn cycle_theme(&mut self) {
        let themes = Theme::all_names();
        if let Some(current_idx) = themes.iter().position(|t| t == &self.theme.name) {
            let next_idx = (current_idx + 1) % themes.len();
            self.theme = Theme::from_name(&themes[next_idx]).unwrap();
            self.config.theme = themes[next_idx].clone();
            let _ = config::save_config(&self.config);
        }
    }

    fn toggle_hidden(&mut self) {
        self.state.show_hidden = !self.state.show_hidden;
        self.config.show_hidden = self.state.show_hidden;
        let _ = config::save_config(&self.config);
        self.state.load_dir();
    }

    fn toggle_preview(&mut self) {
        self.state.preview_open = !self.state.preview_open;
    }

    fn cycle_sort(&mut self) {
        self.state.sort_order = self.state.sort_order.next();
        self.state.sort_entries();
    }

    fn open_editor(&mut self) {
        if let Some(entry) = self.state.selected_entry() {
            if !entry.is_dir {
                if let Ok(content) = fs::read_file(&entry.path) {
                    self.editor_content = content.lines().map(|s| s.to_string()).collect();
                    self.editor_path = Some(entry.path.clone());
                    self.editor_modified = false;
                    self.in_editor = true;
                    self.cursor_line = 0;
                    self.cursor_col = 0;
                    self.scroll_offset = 0;
                    self.undo_stack.clear();
                    self.redo_stack.clear();
                }
            }
        }
    }

    fn save_editor(&mut self) {
        if let Some(path) = &self.editor_path {
            let content = self.editor_content.join("\n");
            if fs::write_file(path, &content).is_ok() {
                self.editor_modified = false;
            }
        }
    }

    fn close_editor(&mut self) {
        self.in_editor = false;
        self.editor_path = None;
        self.editor_content.clear();
        self.editor_modified = false;
        self.cursor_line = 0;
        self.cursor_col = 0;
        self.scroll_offset = 0;
        self.undo_stack.clear();
        self.redo_stack.clear();
    }

    fn push_undo(&mut self) {
        self.undo_stack.push(self.editor_content.clone());
        self.redo_stack.clear();
    }

    fn undo(&mut self) {
        if let Some(prev) = self.undo_stack.pop() {
            self.redo_stack.push(self.editor_content.clone());
            self.editor_content = prev;
            self.editor_modified = true;
        }
    }

    fn redo(&mut self) {
        if let Some(next) = self.redo_stack.pop() {
            self.undo_stack.push(self.editor_content.clone());
            self.editor_content = next;
            self.editor_modified = true;
        }
    }

    fn execute_action(&mut self, action: MenuAction) -> bool {
        match action {
            MenuAction::NewFile => {
                self.dialog.show(DialogType::NewFile);
            }
            MenuAction::NewFolder => {
                self.dialog.show(DialogType::NewFolder);
            }
            MenuAction::Rename => {
                if self.state.selected_entry().is_some() {
                    self.dialog.show(DialogType::Rename);
                    if let Some(entry) = self.state.selected_entry() {
                        self.dialog.input = entry.name.clone();
                    }
                }
            }
            MenuAction::Delete => {
                if !self.state.selected_paths.is_empty() || self.state.selected_entry().is_some() {
                    self.dialog.show(DialogType::Delete);
                    self.dialog.input = String::new();
                }
            }
            MenuAction::Copy => {
                self.clipboard = if !self.state.selected_paths.is_empty() {
                    self.state.selected_paths.iter().cloned().collect()
                } else if let Some(entry) = self.state.selected_entry() {
                    vec![entry.path.clone()]
                } else {
                    Vec::new()
                };
                self.clipboard_cut = false;
            }
            MenuAction::Cut => {
                self.clipboard = if !self.state.selected_paths.is_empty() {
                    self.state.selected_paths.iter().cloned().collect()
                } else if let Some(entry) = self.state.selected_entry() {
                    vec![entry.path.clone()]
                } else {
                    Vec::new()
                };
                self.clipboard_cut = true;
            }
            MenuAction::Paste => {
                if !self.clipboard.is_empty() {
                    for path in &self.clipboard {
                        let dest = self.state.cwd.join(path.file_name().unwrap());
                        if self.clipboard_cut {
                            let _ = fs::rename_path(path, &dest);
                        } else {
                            // TODO: Implement recursive copy in fs.rs
                            // For now just basic file copy if possible
                            if path.is_file() {
                                let _ = std::fs::copy(path, &dest);
                            }
                        }
                    }
                    if self.clipboard_cut {
                        self.clipboard.clear();
                    }
                    self.state.refresh();
                    self.refresh_git_status();
                }
            }
            MenuAction::ClearSelection => {
                self.state.clear_selection();
            }
            MenuAction::Quit => {
                return true;
            }
            MenuAction::Undo => {
                if self.in_editor {
                    self.undo();
                }
            }
            MenuAction::Redo => {
                if self.in_editor {
                    self.redo();
                }
            }
            MenuAction::ToggleHidden => {
                self.toggle_hidden();
            }
            MenuAction::TogglePreview => {
                self.toggle_preview();
            }
            MenuAction::CycleSort => {
                self.cycle_sort();
            }
            MenuAction::CycleTheme => {
                self.cycle_theme();
            }
            MenuAction::Refresh => {
                self.state.refresh();
                self.refresh_git_status();
            }
            MenuAction::OpenEditor => {
                self.open_editor();
            }
            MenuAction::GoUp => {
                self.state.cd_parent();
                self.refresh_git_status();
            }
            MenuAction::ShowKeybindings => {
                self.show_keybindings = true;
            }
            MenuAction::ShowAbout => {
                self.show_about = true;
            }
            MenuAction::OpenCommandPalette => {
                self.command_palette.open_actions();
            }
            MenuAction::OpenFileSearch => {
                self.command_palette.open_files(&self.state.entries);
            }
            MenuAction::GrepInDir => {
                self.command_palette.open_grep();
            }
            MenuAction::ExternalOpen => {
                if let Some(entry) = self.state.selected_entry() {
                    if let Err(e) = opener::open(&entry.path) {
                        self.dialog.error = Some(format!("Failed to open: {}", e));
                    }
                }
            }
            MenuAction::ToggleGitDiff => {
                self.state.show_git_diff = !self.state.show_git_diff;
            }
            MenuAction::ToggleDirSize => {
                self.state.calculate_dir_size = !self.state.calculate_dir_size;
                self.config.calculate_dir_size = self.state.calculate_dir_size;
                let _ = config::save_config(&self.config);
                self.state.load_dir();
            }
            MenuAction::ToggleBookmark => {
                let path = self.state.cwd.clone();
                if self.config.bookmarks.contains(&path) {
                    self.config.bookmarks.retain(|p| p != &path);
                } else {
                    self.config.bookmarks.push(path);
                }
                let _ = config::save_config(&self.config);
            }
            MenuAction::ShowBookmarks => {
                self.command_palette.open_bookmarks(&self.config.bookmarks);
            }
            MenuAction::JumpToFile(path) => {
                if path.is_dir() {
                    self.state.cwd = path;
                    self.state.load_dir();
                    self.refresh_git_status();
                } else {
                    self.state.select_by_path(&path);
                }
            }
            MenuAction::JumpToLocation(path, line) => {
                self.state.select_by_path_and_line(&path, line);
            }
        }
        false
    }

    fn handle_key_event(&mut self, key: KeyEvent) -> bool {
        if self.command_palette.active {
            let result = self
                .command_palette
                .handle_key_event(key, &self.state.entries);
            match result {
                CommandPaletteResult::Action(action) => {
                    return self.execute_action(action);
                }
                CommandPaletteResult::Close => {
                    return false;
                }
                CommandPaletteResult::Continue => {
                    return false;
                }
            }
        }

        if self.show_about {
            match key.code {
                KeyCode::Esc | KeyCode::Enter => {
                    self.show_about = false;
                }
                _ => {}
            }
            return false;
        }

        if self.show_keybindings {
            match key.code {
                KeyCode::Esc | KeyCode::Enter => {
                    self.show_keybindings = false;
                }
                _ => {}
            }
            return false;
        }

        if self.menu_bar.is_open() {
            let action = self.menu_bar.handle_key_event(key);
            if let Some(action) = action {
                return self.execute_action(action);
            }
            return false;
        }

        if self.in_editor {
            self.handle_editor_key(key);
            return false;
        }

        if self.dialog.dialog_type != DialogType::None {
            self.handle_dialog_key(key);
            return false;
        }

        match key.code {
            KeyCode::F(10) => {
                self.menu_bar.open(0);
            }
            KeyCode::Char('p')
                if key
                    .modifiers
                    .contains(crossterm::event::KeyModifiers::CONTROL) =>
            {
                self.command_palette.open_actions();
            }
            KeyCode::Char('f')
                if key
                    .modifiers
                    .contains(crossterm::event::KeyModifiers::CONTROL) =>
            {
                self.command_palette.open_files(&self.state.entries);
            }
            KeyCode::Char('g')
                if key
                    .modifiers
                    .contains(crossterm::event::KeyModifiers::CONTROL) =>
            {
                self.command_palette.open_grep();
            }
            KeyCode::Char('d')
                if key
                    .modifiers
                    .contains(crossterm::event::KeyModifiers::CONTROL) =>
            {
                self.state.show_git_diff = !self.state.show_git_diff;
            }
            KeyCode::Char('i')
                if key
                    .modifiers
                    .contains(crossterm::event::KeyModifiers::CONTROL) =>
            {
                let _ = self.execute_action(MenuAction::ToggleDirSize);
            }
            KeyCode::Char('b')
                if key
                    .modifiers
                    .contains(crossterm::event::KeyModifiers::CONTROL) =>
            {
                let _ = self.execute_action(MenuAction::ToggleBookmark);
            }
            KeyCode::Char('j')
                if key
                    .modifiers
                    .contains(crossterm::event::KeyModifiers::CONTROL) =>
            {
                let _ = self.execute_action(MenuAction::ShowBookmarks);
            }
            KeyCode::Char(' ') => {
                self.state.toggle_selection();
            }
            KeyCode::Up if self.state.selected > 0 => {
                self.state.selected -= 1;
            }
            KeyCode::Down => {
                let has_parent = self.state.cwd.parent().is_some();
                let max_idx = if has_parent {
                    self.state.entries.len()
                } else {
                    self.state.entries.len().saturating_sub(1)
                };
                if self.state.selected < max_idx {
                    self.state.selected += 1;
                }
            }
            KeyCode::Enter => {
                if self.state.is_parent_selected() {
                    self.state.cd_parent();
                    self.refresh_git_status();
                } else {
                    self.state.cd_into();
                    self.refresh_git_status();
                }
            }
            KeyCode::BackTab | KeyCode::Left => {
                self.state.cd_parent();
                self.refresh_git_status();
            }
            KeyCode::Tab => {
                self.toggle_preview();
            }
            KeyCode::Char('e') => {
                self.open_editor();
            }
            KeyCode::Char('o')
                if !key
                    .modifiers
                    .contains(crossterm::event::KeyModifiers::CONTROL) =>
            {
                let _ = self.execute_action(MenuAction::ExternalOpen);
            }
            KeyCode::Char('s')
                if key
                    .modifiers
                    .contains(crossterm::event::KeyModifiers::CONTROL) =>
            {
                self.save_editor();
            }
            KeyCode::Char('q')
                if key
                    .modifiers
                    .contains(crossterm::event::KeyModifiers::CONTROL) =>
            {
                self.close_editor();
            }
            KeyCode::Char('n')
                if key
                    .modifiers
                    .contains(crossterm::event::KeyModifiers::CONTROL) =>
            {
                self.dialog.show(DialogType::NewFile);
            }
            KeyCode::Char('r')
                if key
                    .modifiers
                    .contains(crossterm::event::KeyModifiers::CONTROL)
                    && self.state.selected_entry().is_some() =>
            {
                self.dialog.show(DialogType::Rename);
                if let Some(entry) = self.state.selected_entry() {
                    self.dialog.input = entry.name.clone();
                }
            }
            KeyCode::Delete if self.state.selected_entry().is_some() => {
                self.dialog.show(DialogType::Delete);
                self.dialog.input = String::new();
            }
            KeyCode::Char('t')
                if key
                    .modifiers
                    .contains(crossterm::event::KeyModifiers::CONTROL) =>
            {
                self.cycle_theme();
            }
            KeyCode::Char('o')
                if key
                    .modifiers
                    .contains(crossterm::event::KeyModifiers::CONTROL) =>
            {
                self.cycle_sort();
            }
            KeyCode::Char('h')
                if key
                    .modifiers
                    .contains(crossterm::event::KeyModifiers::CONTROL) =>
            {
                self.toggle_hidden();
            }
            KeyCode::Char('c')
                if key
                    .modifiers
                    .contains(crossterm::event::KeyModifiers::CONTROL) =>
            {
                let _ = self.execute_action(MenuAction::Copy);
            }
            KeyCode::Char('x')
                if key
                    .modifiers
                    .contains(crossterm::event::KeyModifiers::CONTROL) =>
            {
                let _ = self.execute_action(MenuAction::Cut);
            }
            KeyCode::Char('v')
                if key
                    .modifiers
                    .contains(crossterm::event::KeyModifiers::CONTROL) =>
            {
                let _ = self.execute_action(MenuAction::Paste);
            }
            KeyCode::F(5) => {
                self.state.refresh();
                self.refresh_git_status();
            }
            KeyCode::Esc => {
                if self.state.selected_paths.is_empty() {
                    self.cancel_dialog();
                } else {
                    self.state.clear_selection();
                }
            }
            _ => {}
        }
        false
    }

    fn handle_editor_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Up => {
                if self.cursor_line > 0 {
                    self.cursor_line -= 1;
                    if self.cursor_col
                        > self
                            .editor_content
                            .get(self.cursor_line)
                            .map(|l| l.len())
                            .unwrap_or(0)
                    {
                        self.cursor_col = self.editor_content[self.cursor_line].len();
                    }
                }
                if self.cursor_line < self.scroll_offset {
                    self.scroll_offset = self.cursor_line;
                }
            }
            KeyCode::Down => {
                if self.cursor_line < self.editor_content.len().saturating_sub(1) {
                    self.cursor_line += 1;
                    if self.cursor_col
                        > self
                            .editor_content
                            .get(self.cursor_line)
                            .map(|l| l.len())
                            .unwrap_or(0)
                    {
                        self.cursor_col = self.editor_content[self.cursor_line].len();
                    }
                }
                if self.cursor_line >= self.scroll_offset + 20 {
                    self.scroll_offset = self.cursor_line - 19;
                }
            }
            KeyCode::Left => {
                if self.cursor_col > 0 {
                    self.cursor_col -= 1;
                } else if self.cursor_line > 0 {
                    self.cursor_line -= 1;
                    self.cursor_col = self.editor_content[self.cursor_line].len();
                }
            }
            KeyCode::Right => {
                let line_len = self
                    .editor_content
                    .get(self.cursor_line)
                    .map(|l| l.len())
                    .unwrap_or(0);
                if self.cursor_col < line_len {
                    self.cursor_col += 1;
                } else if self.cursor_line < self.editor_content.len() - 1 {
                    self.cursor_line += 1;
                    self.cursor_col = 0;
                }
            }
            KeyCode::Home => {
                self.cursor_col = 0;
            }
            KeyCode::End => {
                self.cursor_col = self
                    .editor_content
                    .get(self.cursor_line)
                    .map(|l| l.len())
                    .unwrap_or(0);
            }
            KeyCode::Char(c)
                if !key
                    .modifiers
                    .contains(crossterm::event::KeyModifiers::CONTROL) =>
            {
                self.push_undo();
                if self.cursor_line >= self.editor_content.len() {
                    self.editor_content.push(String::new());
                }
                self.editor_content[self.cursor_line].insert(self.cursor_col, c);
                self.cursor_col += 1;
                self.editor_modified = true;
            }
            KeyCode::Backspace => {
                if self.cursor_col > 0 {
                    self.push_undo();
                    self.editor_content[self.cursor_line].remove(self.cursor_col - 1);
                    self.cursor_col -= 1;
                    self.editor_modified = true;
                } else if self.cursor_line > 0 {
                    self.push_undo();
                    let _current_len = self.editor_content[self.cursor_line].len();
                    let current = self.editor_content.remove(self.cursor_line);
                    self.cursor_line -= 1;
                    self.cursor_col = self.editor_content[self.cursor_line].len();
                    self.editor_content[self.cursor_line].push_str(&current);
                    self.editor_modified = true;
                }
            }
            KeyCode::Enter => {
                self.push_undo();
                if self.cursor_line >= self.editor_content.len() {
                    self.editor_content.push(String::new());
                }
                let current = self.editor_content[self.cursor_line].clone();
                let new_line = if self.cursor_col >= current.len() {
                    String::new()
                } else {
                    current[self.cursor_col..].to_string()
                };
                self.editor_content[self.cursor_line] = if self.cursor_col >= current.len() {
                    current
                } else {
                    current[..self.cursor_col].to_string()
                };
                self.editor_content.insert(self.cursor_line + 1, new_line);
                self.cursor_line += 1;
                self.cursor_col = 0;
                self.editor_modified = true;
            }
            KeyCode::Esc => {
                self.close_editor();
            }
            KeyCode::Char('s')
                if key
                    .modifiers
                    .contains(crossterm::event::KeyModifiers::CONTROL) =>
            {
                self.save_editor();
            }
            KeyCode::Char('q')
                if key
                    .modifiers
                    .contains(crossterm::event::KeyModifiers::CONTROL) =>
            {
                self.close_editor();
            }
            KeyCode::Char('z')
                if key
                    .modifiers
                    .contains(crossterm::event::KeyModifiers::CONTROL) =>
            {
                self.undo();
            }
            KeyCode::Char('y')
                if key
                    .modifiers
                    .contains(crossterm::event::KeyModifiers::CONTROL) =>
            {
                self.redo();
            }
            _ => {}
        }
    }

    fn handle_dialog_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Char(c) => {
                if self.dialog.dialog_type == DialogType::Delete {
                    if c == 'y' {
                        self.confirm_delete();
                    } else if c == 'n' {
                        self.cancel_dialog();
                    }
                } else {
                    self.dialog.input.push(c);
                }
            }
            KeyCode::Backspace if self.dialog.dialog_type != DialogType::Delete => {
                self.dialog.input.pop();
            }
            KeyCode::Enter => {
                self.confirm_dialog();
            }
            KeyCode::Esc => {
                self.cancel_dialog();
            }
            _ => {}
        }
    }

    fn confirm_dialog(&mut self) {
        let cwd = &self.state.cwd;
        match self.dialog.dialog_type {
            DialogType::NewFile => {
                if self.dialog.input.is_empty() {
                    return;
                }
                let path = cwd.join(&self.dialog.input);
                if let Err(e) = fs::create_file(&path) {
                    self.dialog.error = Some(e);
                } else {
                    self.dialog.hide();
                    self.state.refresh();
                    self.refresh_git_status();
                }
            }
            DialogType::NewFolder => {
                if self.dialog.input.is_empty() {
                    return;
                }
                let path = cwd.join(&self.dialog.input);
                if let Err(e) = fs::create_folder(&path) {
                    self.dialog.error = Some(e);
                } else {
                    self.dialog.hide();
                    self.state.refresh();
                    self.refresh_git_status();
                }
            }
            DialogType::Rename => {
                if self.dialog.input.is_empty() {
                    return;
                }
                if let Some(entry) = self.state.selected_entry() {
                    let new_path = cwd.join(&self.dialog.input);
                    if let Err(e) = fs::rename_path(&entry.path, &new_path) {
                        self.dialog.error = Some(e);
                    } else {
                        self.dialog.hide();
                        self.state.refresh();
                        self.refresh_git_status();
                    }
                }
            }
            DialogType::Delete => {
                self.confirm_delete();
            }
            _ => {}
        }
    }

    fn confirm_delete(&mut self) {
        let paths_to_delete: Vec<PathBuf> = if !self.state.selected_paths.is_empty() {
            self.state.selected_paths.iter().cloned().collect()
        } else if let Some(entry) = self.state.selected_entry() {
            vec![entry.path.clone()]
        } else {
            Vec::new()
        };

        for path in paths_to_delete {
            let _ = fs::delete_path(&path);
        }
        self.state.clear_selection();
        self.dialog.hide();
        self.state.refresh();
        self.refresh_git_status();
    }

    fn cancel_dialog(&mut self) {
        self.dialog.hide();
    }

    fn handle_mouse_event(&mut self, mouse: MouseEvent) {
        if self.in_editor || self.command_palette.active || self.show_about || self.show_keybindings
        {
            return;
        }

        if self.menu_bar.is_open() {
            if let MouseEventKind::Down(_button) = mouse.kind {
                if mouse.row == 0 {
                    let mut offset = 0u16;
                    for (i, menu) in self.menu_bar.menus.iter().enumerate() {
                        let menu_width = menu.name.len() as u16 + 2;
                        if mouse.column >= offset && mouse.column < offset + menu_width {
                            if self.menu_bar.open_index == Some(i) {
                                self.menu_bar.close();
                            } else {
                                self.menu_bar.open(i);
                            }
                            return;
                        }
                        offset += menu_width + 1;
                    }
                    self.menu_bar.close();
                } else if let Some(action) =
                    self.menu_bar.handle_dropdown_click(mouse.row, mouse.column)
                {
                    let _ = self.execute_action(action);
                    return;
                } else {
                    self.menu_bar.close();
                }
            }
            return;
        }

        match mouse.kind {
            MouseEventKind::ScrollUp if self.state.selected > 0 => {
                self.state.selected -= 1;
            }
            MouseEventKind::ScrollDown => {
                let has_parent = self.state.cwd.parent().is_some();
                let max_selected = if has_parent {
                    self.state.entries.len()
                } else {
                    self.state.entries.len().saturating_sub(1)
                };
                if self.state.selected < max_selected {
                    self.state.selected += 1;
                }
            }
            MouseEventKind::Down(_button) => {
                if mouse.row == 0 {
                    let mut offset = 0u16;
                    for (i, menu) in self.menu_bar.menus.iter().enumerate() {
                        let menu_width = menu.name.len() as u16 + 2;
                        if mouse.column >= offset && mouse.column < offset + menu_width {
                            self.menu_bar.open(i);
                            return;
                        }
                        offset += menu_width + 1;
                    }
                    return;
                }

                let has_parent = self.state.cwd.parent().is_some();
                let file_list_start = 5;
                let base_offset = if has_parent { 1 } else { 0 };
                let file_list_end =
                    file_list_start + base_offset as u16 + self.state.entries.len().min(20) as u16;
                if mouse.row >= file_list_start && mouse.row < file_list_end {
                    let visual_idx = (mouse.row - file_list_start) as usize;
                    if visual_idx >= base_offset {
                        let entry_idx = visual_idx - base_offset;
                        if entry_idx < self.state.entries.len() {
                            self.state.selected = visual_idx;
                        }
                    } else if has_parent && visual_idx == 0 {
                        self.state.selected = 0;
                    }
                }
            }
            _ => {}
        }
    }
}

fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup = Layout::new(
        Direction::Vertical,
        [
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ],
    )
    .split(r);

    Layout::new(
        Direction::Horizontal,
        [
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ],
    )
    .split(popup[1])[1]
}

struct EditorInfo<'a> {
    content: &'a [String],
    path: Option<&'a PathBuf>,
    modified: bool,
    cursor_line: usize,
    cursor_col: usize,
    scroll_offset: usize,
}

#[allow(clippy::too_many_arguments)]
fn draw_ui(
    state: &AppState,
    git_status: Option<&GitStatus>,
    editor: Option<&EditorInfo<'_>>,
    theme: &Theme,
    syntax_set: &SyntaxSet,
    theme_set: &ThemeSet,
    area: Rect,
    f: &mut Frame,
) {
    let in_editor = editor.is_some();
    let editor_content = editor.map(|e| e.content).unwrap_or(&[]);
    let editor_path = editor.and_then(|e| e.path);
    let editor_modified = editor.map(|e| e.modified).unwrap_or(false);
    let cursor_line = editor.map(|e| e.cursor_line).unwrap_or(0);
    let cursor_col = editor.map(|e| e.cursor_col).unwrap_or(0);
    let scroll_offset = editor.map(|e| e.scroll_offset).unwrap_or(0);
    f.render_widget(ratatui::widgets::Clear, area);

    let menu_bar_height: u16 = 1;

    let main_area = Rect {
        x: area.x,
        y: area.y + menu_bar_height,
        width: area.width,
        height: area.height.saturating_sub(menu_bar_height),
    };

    let preview_chunks = if state.preview_open && !in_editor {
        Some(
            Layout::new(
                Direction::Horizontal,
                [
                    Constraint::Percentage(50),
                    Constraint::Percentage(2),
                    Constraint::Percentage(48),
                ],
            )
            .split(main_area),
        )
    } else {
        None
    };

    let (file_list_chunk, preview_chunk) = match preview_chunks {
        Some(chunks) => (chunks[0], Some(chunks[2])),
        None => (main_area, None),
    };

    let vertical = Layout::new(
        Direction::Vertical,
        [
            Constraint::Length(3),
            Constraint::Min(1),
            Constraint::Length(3),
        ],
    )
    .split(file_list_chunk);

    let title_text = if let Some(git) = git_status {
        format!(" {} | git:{} ", state.cwd.display(), git.branch)
    } else {
        format!(" {} ", state.cwd.display())
    };
    f.render_widget(
        Paragraph::new(title_text).style(Style::default().fg(theme.title).bg(theme.status_bg)),
        vertical[0],
    );

    let mut list_items: Vec<ListItem> = Vec::new();

    let has_parent = state.cwd.parent().is_some();
    if has_parent {
        let is_selected = state.selected == 0;
        let content = "    \u{1F4C2} ..                                           <PARENT>";
        let style = if is_selected {
            Style::default().fg(theme.selected_fg).bg(theme.selected_bg)
        } else {
            Style::default().fg(theme.dir_fg)
        };
        list_items.push(ListItem::new(content).style(style));
    }

    let base_offset = if has_parent { 1 } else { 0 };
    let list_items_from_fs: Vec<ListItem> = state
        .entries
        .iter()
        .enumerate()
        .map(|(i, entry)| {
            let real_idx = base_offset + i;
            let is_selected = real_idx == state.selected;

            let git_fg = if let Some(git) = git_status {
                match git.get_file_status(&entry.name) {
                    FileGitStatus::StagedNew
                    | FileGitStatus::StagedModified
                    | FileGitStatus::StagedDeleted => ratatui::style::Color::Green,
                    FileGitStatus::Modified
                    | FileGitStatus::Deleted
                    | FileGitStatus::Renamed
                    | FileGitStatus::Copied => ratatui::style::Color::Red,
                    FileGitStatus::Untracked => ratatui::style::Color::Yellow,
                    _ => {
                        if entry.is_dir {
                            theme.dir_fg
                        } else {
                            theme.file_fg
                        }
                    }
                }
            } else {
                if entry.is_dir {
                    theme.dir_fg
                } else {
                    theme.file_fg
                }
            };

            let fg = if is_selected {
                theme.selected_fg
            } else {
                git_fg
            };
            let bg = if is_selected {
                Some(theme.selected_bg)
            } else {
                None
            };

            let icon = if entry.is_dir {
                "\u{1F4C1} "
            } else {
                let ext = entry
                    .name
                    .split('.')
                    .next_back()
                    .unwrap_or("")
                    .to_lowercase();
                match ext.as_str() {
                    "rs" => "\u{1E916} ",
                    "py" => "\u{1F40D} ",
                    "js" | "ts" => "\u{1F4DC} ",
                    "go" => "\u{1F981} ",
                    "java" => "\u{2615} ",
                    "c" | "cpp" | "h" | "hpp" => "\u{1F4BB} ",
                    "toml" | "yaml" | "yml" | "json" | "xml" => "\u{1F4DD} ",
                    "md" => "\u{1F4D6} ",
                    "png" | "jpg" | "jpeg" | "gif" | "bmp" | "ico" | "webp" | "svg" => "\u{1F5BC} ",
                    "mp3" | "flac" | "wav" | "ogg" | "m4a" | "aac" => "\u{1F3B5} ",
                    "pdf" => "\u{1F4C4} ",
                    "zip" | "tar" | "gz" | "bz2" | "xz" | "7z" | "rar" => "\u{1F4E6} ",
                    "sh" | "bash" | "zsh" | "bat" | "ps1" => "\u{1F4A0} ",
                    "css" | "scss" | "less" => "\u{1F3A8} ",
                    "html" | "htm" => "\u{1F310} ",
                    "lock" => "\u{1F512} ",
                    _ => "\u{1F4C4} ",
                }
            };
            let selection_marker = if state.selected_paths.contains(&entry.path) {
                "[*] "
            } else {
                "    "
            };

            let content = format!(
                "{}{} {:<34} {:>8} {}",
                selection_marker,
                icon,
                entry.name,
                entry.size_formatted(),
                entry.modified_formatted()
            );
            let style = if let Some(bg_color) = bg {
                Style::default().fg(fg).bg(bg_color)
            } else {
                Style::default().fg(fg)
            };
            ListItem::new(content).style(style)
        })
        .collect();

    list_items.extend(list_items_from_fs);

    let mut list_state = ListState::default();
    list_state.select(Some(state.selected));
    f.render_stateful_widget(
        List::new(list_items).block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(theme.border),
        ),
        vertical[1],
        &mut list_state,
    );

    let sort_str = match state.sort_order {
        SortOrder::Name => "Name",
        SortOrder::Size => "Size",
        SortOrder::Modified => "Date",
    };
    let entry_info = if !state.selected_paths.is_empty() {
        format!("{} selected", state.selected_paths.len())
    } else if let Some(entry) = state.selected_entry() {
        format!(
            "{} | {} | {}",
            entry.name,
            entry.size_formatted(),
            entry.modified_formatted()
        )
    } else {
        "No selection".to_string()
    };
    let hidden_str = if state.show_hidden {
        "visible"
    } else {
        "hidden"
    };
    let preview_str = if state.preview_open { "ON" } else { "OFF" };
    let status_text = format!(
        "[{}] | Hidden:{}(Ctrl+H) | Preview:{}(Tab) | Sort:{} | o:Open | Theme:{} | F10:Menu | Ctrl+P:Cmd | Ctrl+F:Search | Ctrl+G:Grep | Ctrl+D:Diff",
        entry_info, hidden_str, preview_str, sort_str, theme.name
    );
    f.render_widget(
        Paragraph::new(status_text).style(Style::default().fg(theme.status_fg).bg(theme.status_bg)),
        vertical[2],
    );

    if let Some(preview_area) = preview_chunk {
        let preview_rects = Layout::new(
            Direction::Vertical,
            [Constraint::Length(3), Constraint::Min(1)],
        )
        .split(preview_area);

        let preview_title = if state.show_git_diff {
            " Git Diff "
        } else {
            " Preview "
        };
        f.render_widget(
            Paragraph::new(preview_title)
                .style(Style::default().fg(theme.title).bg(theme.status_bg)),
            preview_rects[0],
        );

        let (preview_text, is_diff) = if let Some(entry) = state.selected_entry() {
            if state.show_git_diff {
                (
                    ratatui::text::Text::from(preview::get_git_diff_preview(
                        &entry.path,
                        &entry.name,
                    )),
                    true,
                )
            } else {
                (
                    preview::get_preview_text(
                        &entry.path,
                        &entry.name,
                        (area.width / 2) as usize - 2,
                        area.height as usize - 2,
                        syntax_set,
                        theme_set,
                    ),
                    false,
                )
            }
        } else {
            (ratatui::text::Text::from("No file selected"), false)
        };

        let preview_block = Block::default()
            .borders(Borders::ALL)
            .border_style(theme.border);
        f.render_widget(preview_block, preview_rects[1]);

        if is_diff {
            let mut lines = Vec::new();
            for line_str in preview_text.to_string().lines() {
                let style = if line_str.starts_with('+') {
                    Style::default().fg(ratatui::style::Color::Green)
                } else if line_str.starts_with('-') {
                    Style::default().fg(ratatui::style::Color::Red)
                } else if line_str.starts_with('@') {
                    Style::default().fg(ratatui::style::Color::Cyan)
                } else {
                    Style::default().fg(theme.preview_fg)
                };
                lines.push(ratatui::text::Line::styled(line_str.to_string(), style));
            }
            f.render_widget(
                Paragraph::new(lines).style(Style::default().bg(theme.preview_bg)),
                preview_rects[1],
            );
        } else {
            f.render_widget(
                Paragraph::new(preview_text).style(Style::default().bg(theme.preview_bg)),
                preview_rects[1],
            );
        }
    }

    if in_editor {
        let editor_area = centered_rect(90, 90, area);
        let title = if editor_modified {
            " Editor * "
        } else {
            " Editor "
        };
        let editor_block = Block::default()
            .title(title)
            .border_style(theme.border)
            .borders(Borders::ALL);
        f.render_widget(&editor_block, editor_area);
        let inner = editor_block.inner(editor_area);

        let visible_lines = inner.height as usize;
        let width = inner.width as usize;

        let mut lines = Vec::new();

        let syntax = editor_path
            .and_then(|p| {
                syntax_set
                    .find_syntax_by_extension(p.extension().and_then(|e| e.to_str()).unwrap_or(""))
            })
            .unwrap_or_else(|| syntax_set.find_syntax_plain_text());

        let syn_theme = &theme_set.themes["base16-ocean.dark"];
        let mut h = syntect::easy::HighlightLines::new(syntax, syn_theme);

        for i in 0..visible_lines {
            let line_idx = scroll_offset + i;
            if line_idx < editor_content.len() {
                let line_str = &editor_content[line_idx];

                let ranges: Vec<(syntect::highlighting::Style, &str)> =
                    h.highlight_line(line_str, syntax_set).unwrap_or_default();

                let mut spans = Vec::new();
                let mut current_width = 0;

                for (style, text) in ranges {
                    if current_width >= width.saturating_sub(2) {
                        break;
                    }

                    let available = width.saturating_sub(2) - current_width;
                    let (display_text, _truncated) = if text.len() > available {
                        (&text[..available], true)
                    } else {
                        (text, false)
                    };

                    let fg = ratatui::style::Color::Rgb(
                        style.foreground.r,
                        style.foreground.g,
                        style.foreground.b,
                    );

                    // Handle cursor in this range
                    let range_start = current_width;
                    let range_end = current_width + display_text.chars().count();

                    if line_idx == cursor_line
                        && cursor_col >= range_start
                        && cursor_col <= range_end
                    {
                        let mut local_spans = Vec::new();
                        let mut local_idx = 0;
                        for ch in display_text.chars() {
                            if local_idx + range_start == cursor_col {
                                local_spans.push(ratatui::text::Span::styled(
                                    "|",
                                    ratatui::style::Style::default()
                                        .fg(ratatui::style::Color::White),
                                ));
                            }
                            local_spans.push(ratatui::text::Span::styled(
                                ch.to_string(),
                                ratatui::style::Style::default().fg(fg),
                            ));
                            local_idx += 1;
                        }
                        if local_idx + range_start == cursor_col {
                            local_spans.push(ratatui::text::Span::styled(
                                "|",
                                ratatui::style::Style::default().fg(ratatui::style::Color::White),
                            ));
                        }
                        spans.extend(local_spans);
                    } else {
                        spans.push(ratatui::text::Span::styled(
                            display_text.to_string(),
                            ratatui::style::Style::default().fg(fg),
                        ));
                    }

                    current_width += display_text.chars().count();
                }

                if line_idx == cursor_line && cursor_col >= current_width {
                    spans.push(ratatui::text::Span::styled(
                        "|",
                        ratatui::style::Style::default().fg(ratatui::style::Color::White),
                    ));
                }

                lines.push(ratatui::text::Line::from(spans));
            } else {
                lines.push(ratatui::text::Line::from(ratatui::text::Span::styled(
                    "~",
                    ratatui::style::Style::default().fg(theme.border),
                )));
            }
        }

        f.render_widget(
            Paragraph::new(lines).style(Style::default().bg(theme.editor_bg)),
            inner,
        );

        let hints = " Ctrl+S:Save | Ctrl+Z:Undo | Ctrl+Y:Redo | Esc:Close ";
        let hints_area = Rect::new(
            inner.x,
            inner.y + inner.height.saturating_sub(1),
            inner.width,
            1,
        );
        f.render_widget(
            Paragraph::new(hints).style(Style::default().fg(theme.title).bg(theme.editor_bg)),
            hints_area,
        );
    }
}

fn draw_about(theme: &Theme, area: Rect, f: &mut Frame) {
    let block = Block::default()
        .title(" About ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.border))
        .style(Style::default().bg(theme.bg).fg(theme.fg));

    f.render_widget(ratatui::widgets::Clear, area);
    f.render_widget(&block, area);
    let inner = block.inner(area);

    let about_text = "Terminal File Explorer\nA terminal-based file manager\nbuilt with Rust and ratatui.\nBuild by @involvex.Funding available through GitHub Sponsors.\nhttps://github.com/sponsors/involvex. \n\nPress Enter or Esc to close.";
    f.render_widget(
        Paragraph::new(about_text).style(Style::default().bg(theme.bg).fg(theme.fg)),
        inner,
    );
}

fn draw_keybindings(theme: &Theme, area: Rect, f: &mut Frame) {
    let block = Block::default()
        .title(" Keybindings ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.border))
        .style(Style::default().bg(theme.bg).fg(theme.fg));

    f.render_widget(ratatui::widgets::Clear, area);
    f.render_widget(&block, area);
    let inner = block.inner(area);

    let bindings = vec![
        "Navigation:",
        "  Up/Down      - Move selection",
        "  Enter        - Open dir/go into",
        "  Left/BackTab - Go to parent dir",
        "  Tab          - Toggle preview pane",
        "  Space        - Toggle file selection",
        "",
        "File Operations:",
        "  Ctrl+N - New file | Ctrl+R - Rename",
        "  Delete  - Delete   | e      - Open editor",
        "  o       - Open with default app",
        "  Ctrl+C - Copy     | Ctrl+X - Cut",
        "  Ctrl+V - Paste    | Ctrl+B - Bookmark",
        "  Ctrl+J - Jump to bookmark",
        "",
        "View:",
        "  Ctrl+H - Toggle hidden files",
        "  Ctrl+O - Cycle sort order",
        "  Ctrl+T - Cycle theme",
        "  Ctrl+I - Toggle dir sizes",
        "  Ctrl+D - Toggle Git diff",
        "  F5      - Refresh",
        "",
        "F10     - Menu bar",
        "Ctrl+P - Commands | Ctrl+F - Search",
        "Ctrl+G - Grep",
        "",
        "Press Enter or Esc to close.",
    ];

    let text = bindings.join("\n");
    f.render_widget(
        Paragraph::new(text).style(Style::default().bg(theme.bg).fg(theme.fg)),
        inner,
    );
}

#[derive(Debug, Clone, PartialEq)]
pub enum DialogType {
    None,
    NewFile,
    NewFolder,
    Rename,
    Delete,
}

pub struct DialogState {
    pub dialog_type: DialogType,
    pub input: String,
    pub error: Option<String>,
}

impl Default for DialogState {
    fn default() -> Self {
        Self {
            dialog_type: DialogType::None,
            input: String::new(),
            error: None,
        }
    }
}

impl DialogState {
    pub fn show(&mut self, dialog_type: DialogType) {
        self.dialog_type = dialog_type;
        self.input.clear();
        self.error = None;
    }

    pub fn hide(&mut self) {
        self.dialog_type = DialogType::None;
        self.input.clear();
        self.error = None;
    }
}

fn draw_dialog(state: &DialogState, theme: &Theme, area: Rect, f: &mut Frame) {
    if state.dialog_type == DialogType::None {
        return;
    }

    let (title, instruction) = match state.dialog_type {
        DialogType::NewFile => ("New File", "Enter filename:"),
        DialogType::NewFolder => ("New Folder", "Enter folder name:"),
        DialogType::Rename => ("Rename", "Enter new name:"),
        DialogType::Delete => ("Delete", "Confirm deletion:"),
        DialogType::None => return,
    };

    let block = Block::default()
        .title(format!(" {} ", title))
        .borders(Borders::ALL)
        .border_style(theme.border)
        .style(Style::default().bg(theme.bg).fg(theme.fg));

    let content = if state.dialog_type == DialogType::Delete {
        format!(
            "{}\n{}\n\nPress 'y' to confirm, 'n' or 'Esc' to cancel",
            instruction, state.input
        )
    } else {
        format!(
            "{}\n\n> {}\n\n[Enter] Confirm  [Esc] Cancel",
            instruction, state.input
        )
    };

    let fg = if state.error.is_some() {
        theme.error_fg
    } else {
        theme.fg
    };

    f.render_widget(&block, area);
    let inner = block.inner(area);
    f.render_widget(
        Paragraph::new(content).style(Style::default().bg(theme.bg).fg(fg)),
        inner,
    );

    if let Some(err) = &state.error {
        let err_text = format!("ERROR: {}", err);
        f.render_widget(
            Paragraph::new(err_text).style(Style::default().bg(theme.bg).fg(theme.error_fg)),
            inner,
        );
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cwd = env::current_dir().unwrap_or_else(|_| PathBuf::from("."));

    crossterm::terminal::enable_raw_mode()?;

    let mut stdout = std::io::stdout();
    execute!(
        stdout,
        EnterAlternateScreen,
        SetTitle("Terminal File Explorer"),
        EnableMouseCapture
    )?;

    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = App::new(cwd);

    loop {
        if crossterm::event::poll(std::time::Duration::from_millis(100))? {
            match crossterm::event::read()? {
                Event::Key(key) => {
                    if key.kind != KeyEventKind::Press {
                        continue;
                    }
                    if key.code == KeyCode::Char('c')
                        && key
                            .modifiers
                            .contains(crossterm::event::KeyModifiers::CONTROL)
                    {
                        break;
                    }
                    let should_quit = app.handle_key_event(key);
                    if should_quit {
                        break;
                    }
                }
                Event::Mouse(mouse) => {
                    app.handle_mouse_event(mouse);
                }
                Event::Resize(_, _) | Event::FocusGained | Event::FocusLost | Event::Paste(_) => {}
            }
        }

        terminal.draw(|f| {
            let editor_info = if app.in_editor {
                Some(EditorInfo {
                    content: &app.editor_content,
                    path: app.editor_path.as_ref(),
                    modified: app.editor_modified,
                    cursor_line: app.cursor_line,
                    cursor_col: app.cursor_col,
                    scroll_offset: app.scroll_offset,
                })
            } else {
                None
            };
            draw_ui(
                &app.state,
                app.git_status.as_ref(),
                editor_info.as_ref(),
                &app.theme,
                &app.syntax_set,
                &app.theme_set,
                f.area(),
                f,
            );

            let menu_bar_area = Rect {
                x: f.area().x,
                y: f.area().y,
                width: f.area().width,
                height: 1,
            };
            menu::draw_menu_bar(&app.menu_bar, &app.theme, menu_bar_area, f);

            if app.dialog.dialog_type != DialogType::None {
                let dialog_area = centered_rect(40, 20, f.area());
                draw_dialog(&app.dialog, &app.theme, dialog_area, f);
            }

            if app.show_about {
                let about_area = centered_rect(50, 30, f.area());
                draw_about(&app.theme, about_area, f);
            }

            if app.show_keybindings {
                let kb_area = centered_rect(50, 60, f.area());
                draw_keybindings(&app.theme, kb_area, f);
            }

            if app.command_palette.active {
                command_palette::draw_command_palette(
                    &app.command_palette,
                    &app.theme,
                    f.area(),
                    f,
                );
            }
        })?;
    }

    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    crossterm::terminal::disable_raw_mode()?;

    Ok(())
}
