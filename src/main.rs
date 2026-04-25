mod config;
mod fs;
mod preview;
mod state;
mod theme;

use config::Config;
use crossterm::{
    event::{
        DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEvent, MouseEvent,
        MouseEventKind,
    },
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, SetTitle},
};
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::Style;
use ratatui::widgets::{Block, Borders, List, ListItem, ListState, Paragraph};
use ratatui::{backend::CrosstermBackend, Frame, Terminal};
use state::{AppState, SortOrder};
use std::env;
use std::path::PathBuf;
use theme::Theme;

struct App {
    state: AppState,
    config: Config,
    theme: Theme,
    dialog: DialogState,
    editor_content: String,
    editor_path: Option<PathBuf>,
    editor_modified: bool,
    in_editor: bool,
}

impl App {
    fn new(cwd: PathBuf) -> Self {
        let config = Config::load();
        let theme_name = &config.theme;
        let theme = Theme::from_name(theme_name)
            .unwrap_or_else(|| Theme::from_name("tokyo-night").unwrap());

        let mut state = AppState::new(cwd);
        state.show_hidden = config.show_hidden;
        state.load_dir();

        Self {
            state,
            config,
            theme,
            dialog: DialogState::default(),
            editor_content: String::new(),
            editor_path: None,
            editor_modified: false,
            in_editor: false,
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
                if let Ok(content) = fs::read_file(&entry.path) {
                    self.editor_content = content;
                    self.editor_path = Some(entry.path.clone());
                    self.editor_modified = false;
                    self.in_editor = true;
                }
            }
        }
    }

    fn save_editor(&mut self) {
        if let Some(path) = &self.editor_path {
            if fs::write_file(path, &self.editor_content).is_ok() {
                self.editor_modified = false;
            }
        }
    }

    fn close_editor(&mut self) {
        self.in_editor = false;
        self.editor_path = None;
        self.editor_content = String::new();
        self.editor_modified = false;
    }

    fn handle_key_event(&mut self, key: KeyEvent) {
        if self.in_editor {
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
                if self
                    .state
                    .selected_entry()
                    .map(|e| e.is_dir)
                    .unwrap_or(false)
                {
                    self.state.cd_into();
                }
            }
            KeyCode::BackTab | KeyCode::Left => {
                self.state.cd_parent();
            }
            KeyCode::Tab => {
                self.toggle_preview();
            }
            KeyCode::Char('e') => {
                self.open_editor();
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
                    .contains(crossterm::event::KeyModifiers::CONTROL) =>
            {
                if self.state.selected_entry().is_some() {
                    self.dialog.show(DialogType::Rename);
                    if let Some(entry) = self.state.selected_entry() {
                        self.dialog.input = entry.name.clone();
                    }
                }
            }
            KeyCode::Delete => {
                if self.state.selected_entry().is_some() {
                    self.dialog.show(DialogType::Delete);
                    self.dialog.input = String::new();
                }
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
            KeyCode::Char('p')
                if key
                    .modifiers
                    .contains(crossterm::event::KeyModifiers::CONTROL) =>
            {
                self.toggle_hidden();
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
            KeyCode::Char(c)
                if !key
                    .modifiers
                    .contains(crossterm::event::KeyModifiers::CONTROL) =>
            {
                self.editor_content.push(c);
                self.editor_modified = true;
            }
            KeyCode::Backspace => {
                self.editor_content.pop();
                self.editor_modified = true;
            }
            KeyCode::Enter => {
                self.editor_content.push('\n');
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
            KeyCode::Backspace => {
                if self.dialog.dialog_type != DialogType::Delete {
                    self.dialog.input.pop();
                }
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
        if let Some(entry) = self.state.selected_entry() {
            if let Err(e) = fs::delete_path(&entry.path) {
                self.dialog.error = Some(e);
            } else {
                self.dialog.hide();
                self.state.refresh();
            }
        }
    }

    fn cancel_dialog(&mut self) {
        self.dialog.hide();
    }

    fn handle_mouse_event(&mut self, mouse: MouseEvent) {
        if self.in_editor {
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
                        if self
                            .state
                            .selected_entry()
                            .map(|e| e.is_dir)
                            .unwrap_or(false)
                        {
                            self.state.cd_into();
                        }
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

fn draw_ui(
    state: &AppState,
    in_editor: bool,
    editor_content: &str,
    theme: &Theme,
    area: Rect,
    f: &mut Frame,
) {
    f.render_widget(ratatui::widgets::Clear, area);

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
            .split(area),
        )
    } else {
        None
    };

    let (file_list_chunk, preview_chunk) = match preview_chunks {
        Some(chunks) => (chunks[0], Some(chunks[2])),
        None => (area, None),
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

    let title = format!(" {} ", state.cwd.display());
    f.render_widget(
        Paragraph::new(title).style(Style::default().fg(theme.title).bg(theme.status_bg)),
        vertical[0],
    );

    let list_items: Vec<ListItem> = state
        .entries
        .iter()
        .enumerate()
        .map(|(i, entry)| {
            let is_selected = i == state.selected;
            let fg = if entry.is_dir {
                theme.dir_fg
            } else {
                theme.file_fg
            };
            let prefix = if entry.is_dir { "[D] " } else { "[F] " };
            let content = format!(
                "{}{:<40} {:>8} {}",
                prefix,
                entry.name,
                entry.size_formatted(),
                entry.modified_formatted()
            );
            let style = if is_selected {
                Style::default().fg(theme.selected_fg).bg(theme.selected_bg)
            } else {
                Style::default().fg(fg)
            };
            ListItem::new(content).style(style)
        })
        .collect();

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
    let entry_info = if let Some(entry) = state.selected_entry() {
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
    let preview_str = if state.preview_open { "on" } else { "off" };
    let status_text = format!(
        " [{}] | Hidden: {} | Preview: {} | Sort: {} | Theme: {} | F5:Refresh | Tab:Preview | Ctrl+T:Theme | Ctrl+O:Sort",
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

        f.render_widget(
            Paragraph::new(" Preview ").style(Style::default().fg(theme.title).bg(theme.status_bg)),
            preview_rects[0],
        );

        let content = if let Some(entry) = state.selected_entry() {
            preview::get_preview_content(
                &entry.path,
                &entry.name,
                (area.width / 2) as usize - 2,
                area.height as usize - 2,
            )
        } else {
            "No file selected".to_string()
        };
        let preview_block = Block::default()
            .borders(Borders::ALL)
            .border_style(theme.border);
        f.render_widget(preview_block, preview_rects[1]);
        f.render_widget(
            Paragraph::new(content)
                .style(Style::default().fg(theme.preview_fg).bg(theme.preview_bg)),
            preview_rects[1],
        );
    }

    if in_editor {
        let editor_area = centered_rect(90, 90, area);
        let editor_block = Block::default()
            .title(" Editor ")
            .border_style(theme.border)
            .borders(Borders::ALL);
        f.render_widget(&editor_block, editor_area);
        let inner = editor_block.inner(editor_area);
        f.render_widget(
            Paragraph::new(editor_content)
                .style(Style::default().fg(theme.editor_fg).bg(theme.editor_bg)),
            inner,
        );
    }
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
                    if key.code == KeyCode::Char('c')
                        && key
                            .modifiers
                            .contains(crossterm::event::KeyModifiers::CONTROL)
                    {
                        break;
                    }
                    app.handle_key_event(key);
                }
                Event::Mouse(mouse) => {
                    app.handle_mouse_event(mouse);
                }
                Event::Resize(_, _) | Event::FocusGained | Event::FocusLost | Event::Paste(_) => {}
            }
        }

        terminal.draw(|f| {
            draw_ui(
                &app.state,
                app.in_editor,
                &app.editor_content,
                &app.theme,
                f.area(),
                f,
            );
            if app.dialog.dialog_type != DialogType::None {
                let dialog_area = centered_rect(40, 20, f.area());
                draw_dialog(&app.dialog, &app.theme, dialog_area, f);
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
