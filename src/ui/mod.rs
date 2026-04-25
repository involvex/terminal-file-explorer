use crate::state::{AppState, FileEntry};
use crate::theme::Theme;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style, Stylize},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph, Widget},
    Frame,
};

pub fn draw(state: &AppState, theme: &Theme, area: Rect, f: &mut Frame) {
    f.render_widget(ratatui::widgets::Clear, area);

    let chunks = if state.preview_open && !state.in_editor {
        Layout::new(
            Direction::Horizontal,
            [
                Constraint::Percentage(50),
                Constraint::Percentage(2),
                Constraint::Percentage(48),
            ],
        )
        .split(area)
    } else {
        Layout::new(Direction::Horizontal, [Constraint::Percentage(100)])
            .split(area)
            .to_vec()
    };

    let file_list_chunk = chunks[0];
    let file_list_area = Layout::new(
        Direction::Vertical,
        [
            Constraint::Length(3),
            Constraint::Min(1),
            Constraint::Length(3),
        ],
    )
    .split(file_list_chunk);

    let title = format!(" {} ", state.cwd.display());
    let title_widget =
        Paragraph::new(title).style(Style::default().fg(theme.title).bg(theme.status_bg));

    f.render_widget(title_widget, file_list_area[0]);

    let list_content = file_list_area[1];
    let file_list = FileList::new(state, theme);
    f.render_widget(file_list, list_content);

    let status_widget = StatusBar::new(state, theme);
    f.render_widget(status_widget, file_list_area[2]);

    if state.preview_open && !state.in_editor && chunks.len() >= 3 {
        let preview_chunk = chunks[2];
        let preview_area = Layout::new(
            Direction::Vertical,
            [Constraint::Length(3), Constraint::Min(1)],
        )
        .split(preview_chunk);

        let preview_title = " Preview ";
        let preview_title_widget = Paragraph::new(preview_title)
            .style(Style::default().fg(theme.title).bg(theme.status_bg));
        f.render_widget(preview_title_widget, preview_area[0]);

        let preview_widget = Preview::new(state, theme);
        f.render_widget(preview_widget, preview_area[1]);
    }

    if state.in_editor {
        let editor_area = centered_rect(90, 90, area);
        let editor_block = Block::default()
            .title(" Editor ")
            .border_style(theme.border)
            .borders(Borders::ALL);
        let content = state.editor_content.as_str();
        let editor_content =
            Paragraph::new(content).style(Style::default().fg(theme.editor_fg).bg(theme.editor_bg));
        f.render_widget(editor_block, editor_area);
        let inner = editor_block.inner(editor_area);
        f.render_widget(editor_content, inner);
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

pub struct FileList<'a> {
    state: &'a AppState,
    theme: &'a Theme,
}

impl<'a> FileList<'a> {
    pub fn new(state: &'a AppState, theme: &'a Theme) -> Self {
        Self { state, theme }
    }
}

impl<'a> Widget for FileList<'a> {
    fn render(self, area: Rect, f: &mut Frame) {
        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(self.theme.border);

        let list_items: Vec<ListItem> = self
            .state
            .entries
            .iter()
            .enumerate()
            .map(|(i, entry)| {
                let is_selected = i == self.state.selected;
                let fg = if entry.is_dir {
                    self.theme.dir_fg
                } else {
                    self.theme.file_fg
                };

                let prefix = if entry.is_dir { "[D] " } else { "[F] " };
                let size = entry.size_formatted();
                let modified = entry.modified_formatted();
                let content = format!("{}{:<40} {:>8} {}", prefix, entry.name, size, modified);

                let style = if is_selected {
                    Style::default()
                        .fg(self.theme.selected_fg)
                        .bg(self.theme.selected_bg)
                } else {
                    Style::default().fg(fg)
                };

                ListItem::new(content).style(style)
            })
            .collect();

        let list = List::new(list_items).block(block);

        let mut list_state = ListState::default();
        list_state.select(Some(self.state.selected));

        f.render_stateful_widget(list, area, &mut list_state);
    }
}

pub struct Preview<'a> {
    state: &'a AppState,
    theme: &'a Theme,
}

impl<'a> Preview<'a> {
    pub fn new(state: &'a AppState, theme: &'a Theme) -> Self {
        Self { state, theme }
    }
}

impl<'a> Widget for Preview<'a> {
    fn render(self, area: Rect, f: &mut Frame) {
        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(self.theme.border);

        let content = if let Some(entry) = self.state.selected_entry() {
            crate::preview::get_preview_content(
                &entry.path,
                &entry.name,
                area.width as usize - 2,
                area.height as usize - 2,
            )
        } else {
            "No file selected".to_string()
        };

        let paragraph = Paragraph::new(content).style(
            Style::default()
                .fg(self.theme.preview_fg)
                .bg(self.theme.preview_bg),
        );

        f.render_widget(block, area);
        f.render_widget(paragraph, area);
    }
}

pub struct StatusBar<'a> {
    state: &'a AppState,
    theme: &'a Theme,
}

impl<'a> StatusBar<'a> {
    pub fn new(state: &'a AppState, theme: &'a Theme) -> Self {
        Self { state, theme }
    }
}

impl<'a> Widget for StatusBar<'a> {
    fn render(self, area: Rect, f: &mut Frame) {
        let sort_str = match self.state.sort_order {
            crate::state::SortOrder::Name => "Name",
            crate::state::SortOrder::Size => "Size",
            crate::state::SortOrder::Modified => "Date",
        };

        let entry_info = if let Some(entry) = self.state.selected_entry() {
            format!(
                "{} | {} | {}",
                entry.name,
                entry.size_formatted(),
                entry.modified_formatted()
            )
        } else {
            "No selection".to_string()
        };

        let hidden_str = if self.state.show_hidden {
            "visible"
        } else {
            "hidden"
        };
        let preview_str = if self.state.preview_open { "on" } else { "off" };

        let status_text = format!(
            " [{}] | Hidden: {} | Preview: {} | Sort: {} | Theme: {} | F5:Refresh | Tab:Preview | Ctrl+T:Theme | Ctrl+O:Sort",
            entry_info,
            hidden_str,
            preview_str,
            sort_str,
            self.theme.name
        );

        let paragraph = Paragraph::new(status_text).style(
            Style::default()
                .fg(self.theme.status_fg)
                .bg(self.theme.status_bg),
        );

        f.render_widget(paragraph, area);
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

pub struct Dialog<'a> {
    dialog_state: &'a DialogState,
    theme: &'a Theme,
}

impl<'a> Dialog<'a> {
    pub fn new(dialog_state: &'a DialogState, theme: &'a Theme) -> Self {
        Self {
            dialog_state,
            theme,
        }
    }
}

impl<'a> Widget for Dialog<'a> {
    fn render(self, area: Rect, f: &mut Frame) {
        if self.dialog_state.dialog_type == DialogType::None {
            return;
        }

        let (title, instruction) = match self.dialog_state.dialog_type {
            DialogType::NewFile => ("New File", "Enter filename:"),
            DialogType::NewFolder => ("New Folder", "Enter folder name:"),
            DialogType::Rename => ("Rename", "Enter new name:"),
            DialogType::Delete => ("Delete", "Confirm deletion:"),
            DialogType::None => return,
        };

        let block = Block::default()
            .title(format!(" {} ", title))
            .borders(Borders::ALL)
            .border_style(self.theme.border)
            .style(Style::default().bg(self.theme.bg).fg(self.theme.fg));

        let content = if self.dialog_state.dialog_type == DialogType::Delete {
            format!(
                "{}\n{}\n\nPress 'y' to confirm, 'n' or 'Esc' to cancel",
                instruction, self.dialog_state.input
            )
        } else {
            format!(
                "{}\n\n> {}\n\n[Enter] Confirm  [Esc] Cancel",
                instruction, self.dialog_state.input
            )
        };

        if let Some(err) = &self.dialog_state.error {
            let content = format!("{}\n\nERROR: {}", content, err);
            let paragraph = Paragraph::new(content)
                .style(Style::default().bg(self.theme.bg).fg(self.theme.error_fg));
            f.render_widget(block, area);
            let inner = block.inner(area);
            f.render_widget(paragraph, inner);
        } else {
            let paragraph =
                Paragraph::new(content).style(Style::default().bg(self.theme.bg).fg(self.theme.fg));
            f.render_widget(block, area);
            let inner = block.inner(area);
            f.render_widget(paragraph, inner);
        }
    }
}
