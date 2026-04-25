mod config;
mod state;
mod theme;
mod ui;
mod fs;
mod preview;
mod editor;

use std::env;
use std::path::PathBuf;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEvent, MouseEvent, MouseEventKind},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, SetTitle},
};
use ratatui::{Terminal, backend::CrosstermBackend};
use crate::state::{AppState, SortOrder};
use crate::config::Config;
use crate::theme::Theme;
use crate::ui::dialog::{DialogState, DialogType};
use crate::ui::layout::draw;

struct App {
    state: AppState,
    config: Config,
    theme: Theme,
    dialog: DialogState,
    editor_state: editor::EditorState,
    quit_editor: bool,
}

impl App {
    fn new(cwd: PathBuf) -> Self {
        let config = Config::load();
        let theme_name = &config.theme;
        let theme = Theme::from_name(theme_name).unwrap_or_else(|| Theme::from_name("tokyo-night").unwrap());

        let mut state = AppState::new(cwd);
        state.show_hidden = config.show_hidden;
        state.load_dir();

        Self {
            state,
            config,
            theme,
            dialog: DialogState::default(),
            editor_state: editor::EditorState::default(),
            quit_editor: false,
        }
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
                if let Ok(content) = fs::operations::read_file(&entry.path) {
                    self.editor_state = editor::EditorState::new(content);
                    self.state.in_editor = true;
                    self.state.editor_path = Some(entry.path.clone());
                    self.quit_editor = false;
                }
            }
        }
    }

    fn save_editor(&mut self) {
        if let Some(path) = &self.state.editor_path {
            let content = self.editor_state.to_string();
            if fs::operations::write_file(path, &content).is_ok() {
                self.editor_state.modified = false;
                self.state.editor_modified = false;
            }
        }
    }

    fn close_editor(&mut self) {
        self.state.in_editor = false;
        self.state.editor_path = None;
        self.editor_state = editor::EditorState::default();
        self.quit_editor = false;
    }

    fn handle_dialog_input(&mut self, c: char) {
        if self.dialog.dialog_type == DialogType::Delete {
            return;
        }
        self.dialog.input.push(c);
    }

    fn handle_dialog_backspace(&mut self) {
        if self.dialog.dialog_type == DialogType::Delete {
            return;
        }
        self.dialog.input.pop();
    }

    fn confirm_dialog(&mut self) {
        if self.dialog.input.is_empty() && self.dialog.dialog_type != DialogType::Delete {
            return;
        }

        let cwd = &self.state.cwd;
        match self.dialog.dialog_type {
            DialogType::NewFile => {
                let path = cwd.join(&self.dialog.input);
                if let Err(e) = fs::operations::create_file(&path) {
                    self.dialog.error = Some(e);
                } else {
                    self.dialog.hide();
                    self.state.refresh();
                }
            }
            DialogType::NewFolder => {
                let path = cwd.join(&self.dialog.input);
                if let Err(e) = fs::operations::create_folder(&path) {
                    self.dialog.error = Some(e);
                } else {
                    self.dialog.hide();
                    self.state.refresh();
                }
            }
            DialogType::Rename => {
                if let Some(entry) = self.state.selected_entry() {
                    let new_path = cwd.join(&self.dialog.input);
                    if let Err(e) = fs::operations::rename_path(&entry.path, &new_path) {
                        self.dialog.error = Some(e);
                    } else {
                        self.dialog.hide();
                        self.state.refresh();
                    }
                }
            }
            DialogType::Delete => {
                if let Some(entry) = self.state.selected_entry() {
                    if let Err(e) = fs::operations::delete_path(&entry.path) {
                        self.dialog.error = Some(e);
                    } else {
                        self.dialog.hide();
                        self.state.refresh();
                    }
                }
            }
            _ => {}
        }
    }

    fn cancel_dialog(&mut self) {
        self.dialog.hide();
    }

    fn handle_key_event(&mut self, key: KeyEvent) {
        if self.state.in_editor {
            self.handle_editor_key(key);
            return;
        }

        if self.dialog.dialog_type != DialogType::None {
            self.handle_dialog_key(key);
            return;
        }

        match key.code {
            KeyCode::Up => {
                if self.state.selected > 0 {
                    self.state.selected -= 1;
                }
            }
            KeyCode::Down => {
                if self.state.selected < self.state.entries.len().saturating_sub(1) {
                    self.state.selected += 1;
                }
            }
            KeyCode::Enter => {
                if self.state.selected_entry().map(|e| e.is_dir).unwrap_or(false) {
                    self.state.cd_into();
                }
            }
            KeyCode::BackTab | KeyCode::Left | KeyCode::Char('h') => {
                self.state.cd_parent();
            }
            KeyCode::Tab => {
                self.toggle_preview();
            }
            KeyCode::Char('e') => {
                self.open_editor();
            }
            KeyCode::Char('s') => {
                if key.modifiers.contains(crossterm::event::KeyModifiers::CONTROL) {
                    self.save_editor();
                }
            }
            KeyCode::Char('q') => {
                if key.modifiers.contains(crossterm::event::KeyModifiers::CONTROL) {
                    self.close_editor();
                }
            }
            KeyCode::Char('n') => {
                if key.modifiers.contains(crossterm::event::KeyModifiers::CONTROL) {
                    self.dialog.show(DialogType::NewFile);
                }
            }
            KeyCode::Char('r') => {
                if key.modifiers.contains(crossterm::event::KeyModifiers::CONTROL) {
                    if self.state.selected_entry().is_some() {
                        self.dialog.show(DialogType::Rename);
                        if let Some(entry) = self.state.selected_entry() {
                            self.dialog.input = entry.name.clone();
                        }
                    }
                }
            }
            KeyCode::Delete => {
                if self.state.selected_entry().is_some() {
                    self.dialog.show(DialogType::Delete);
                    self.dialog.input = String::new();
                }
            }
            KeyCode::Char('t') => {
                if key.modifiers.contains(crossterm::event::KeyModifiers::CONTROL) {
                    self.cycle_theme();
                }
            }
            KeyCode::Char('o') => {
                if key.modifiers.contains(crossterm::event::KeyModifiers::CONTROL) {
                    self.cycle_sort();
                }
            }
            KeyCode::Char('p') => {
                if key.modifiers.contains(crossterm::event::KeyModifiers::CONTROL) {
                    self.toggle_hidden();
                }
            }
            KeyCode::F(5) => {
                self.state.refresh();
            }
            KeyCode::Esc => {
                self.cancel_dialog();
            }
            _ => {}
        }
    }

    fn handle_editor_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Up => self.editor_state.move_up(),
            KeyCode::Down => self.editor_state.move_down(),
            KeyCode::Left => self.editor_state.move_left(),
            KeyCode::Right => self.editor_state.move_right(),
            KeyCode::Home => self.editor_state.move_to_start(),
            KeyCode::End => self.editor_state.move_to_end(),
            KeyCode::Enter => self.editor_state.insert_newline(),
            KeyCode::Backspace => self.editor_state.delete_char(),
            KeyCode::Char(c) => {
                if !key.modifiers.contains(crossterm::event::KeyModifiers::CONTROL) {
                    self.editor_state.insert_char(c);
                }
            }
            KeyCode::Esc => {
                if self.editor_state.modified {
                    self.quit_editor = true;
                } else {
                    self.close_editor();
                }
            }
            KeyCode::Char('s') => {
                if key.modifiers.contains(crossterm::event::KeyModifiers::CONTROL) {
                    self.save_editor();
                }
            }
            KeyCode::Char('q') => {
                if key.modifiers.contains(crossterm::event::KeyModifiers::CONTROL) {
                    if self.editor_state.modified {
                        self.quit_editor = true;
                    } else {
                        self.close_editor();
                    }
                }
            }
            _ => {}
        }
    }

    fn handle_dialog_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Char(c) => {
                if self.dialog.dialog_type == DialogType::Delete {
                    if c == 'y' {
                        self.confirm_dialog();
                    } else if c == 'n' {
                        self.cancel_dialog();
                    }
                } else {
                    self.handle_dialog_input(c);
                }
            }
            KeyCode::Backspace => self.handle_dialog_backspace(),
            KeyCode::Enter => self.confirm_dialog(),
            KeyCode::Esc => self.cancel_dialog(),
            _ => {}
        }
    }

    fn handle_mouse_event(&mut self, mouse: MouseEvent) {
        if self.state.in_editor {
            return;
        }

        match mouse.kind {
            MouseEventKind::ScrollUp => {
                if self.state.selected > 0 {
                    self.state.selected -= 1;
                }
            }
            MouseEventKind::ScrollDown => {
                if self.state.selected < self.state.entries.len().saturating_sub(1) {
                    self.state.selected += 1;
                }
            }
            MouseEventKind::Down(_button) => {
                let height = self.state.entries.len().min(20);
                if mouse.row >= 1 && mouse.row < 1 + height as u16 {
                    let idx = (mouse.row - 1) as usize;
                    if idx < self.state.entries.len() {
                        self.state.selected = idx;
                        if self.state.selected_entry().map(|e| e.is_dir).unwrap_or(false) {
                            self.state.cd_into();
                        }
                    }
                }
            }
            _ => {}
        }
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cwd = env::current_dir().unwrap_or_else(|_| PathBuf::from("."));

    crossterm::terminal::enable_raw_mode()?;

    let mut stdout = std::io::stdout();
    execute!(stdout, EnterAlternateScreen, SetTitle("Terminal File Explorer"), EnableMouseCapture)?;

    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = App::new(cwd);

    terminal.draw(|f| {
        let area = f.area();
        draw(&app.state, &app.theme, area, f);
        if app.dialog.dialog_type != DialogType::None {
            let dialog_area = ui::centered_rect(40, 20, area);
            let dialog = ui::dialog::Dialog::new(&app.dialog, &app.theme);
            f.render_widget(dialog, dialog_area);
        }
    })?;

    loop {
        if crossterm::event::poll(std::time::Duration::from_millis(100))? {
            match crossterm::event::read()? {
                Event::Key(key) => {
                    if key.code == KeyCode::Char('c') && key.modifiers.contains(crossterm::event::KeyModifiers::CONTROL) {
                        break;
                    }
                    app.handle_key_event(key);
                }
                Event::Mouse(mouse) => {
                    app.handle_mouse_event(mouse);
                }
                Event::Resize(_, _) => {}
            }
        }

        terminal.draw(|f| {
            let area = f.area();
            draw(&app.state, &app.theme, area, f);
            if app.dialog.dialog_type != DialogType::None {
                let dialog_area = ui::centered_rect(40, 20, area);
                let dialog = ui::dialog::Dialog::new(&app.dialog, &app.theme);
                f.render_widget(dialog, dialog_area);
            }
        })?;
    }

    execute!(terminal.backend_mut(), LeaveAlternateScreen, DisableMouseCapture)?;
    crossterm::terminal::disable_raw_mode()?;

    Ok(())
}

mod ui {
    pub mod layout;
    pub mod file_list;
    pub mod preview;
    pub mod status;
    pub mod dialog;

    pub fn centered_rect(percent_x: u16, percent_y: u16, r: ratatui::layout::Rect) -> ratatui::layout::Rect {
        use ratatui::layout::{Constraint, Direction, Layout};
        let popup = Layout::new(
            Direction::Vertical,
            [
                Constraint::percentage((100 - percent_y) / 2),
                Constraint::percentage(percent_y),
                Constraint::percentage((100 - percent_y) / 2),
            ],
        )
        .split(r);

        Layout::new(
            Direction::Horizontal,
            [
                Constraint::percentage((100 - percent_x) / 2),
                Constraint::percentage(percent_x),
                Constraint::percentage((100 - percent_x) / 2),
            ],
        )
        .split(popup[1])[1]
    }
}