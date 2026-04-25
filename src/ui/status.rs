use crate::state::{AppState, SortOrder};
use crate::theme::Theme;
use ratatui::{layout::Rect, widgets::Widget, Frame};

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
            SortOrder::Name => "Name",
            SortOrder::Size => "Size",
            SortOrder::Modified => "Date",
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
            " [{}] | Hidden: {} | Preview: {} | Theme: {} | F5:Refresh | Tab:Preview | Ctrl+T:Theme | Ctrl+O:Sort",
            entry_info,
            hidden_str,
            preview_str,
            self.theme.name
        );

        let paragraph = ratatui::widgets::Paragraph::new(status_text).style(
            ratatui::style::Style::default()
                .fg(self.theme.status_fg)
                .bg(self.theme.status_bg),
        );

        f.render_widget(paragraph, area);
    }
}
