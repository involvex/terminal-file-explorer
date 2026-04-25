use crate::state::{AppState, FileEntry};
use crate::theme::Theme;
use ratatui::{
    layout::Rect,
    style::Style,
    widgets::{Block, Borders, List, ListItem, ListState, Widget},
    Frame,
};

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
