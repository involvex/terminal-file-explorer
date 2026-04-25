use crate::state::AppState;
use crate::theme::Theme;
use ratatui::{layout::Rect, style::Style, widgets::Widget, Frame};

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
        let block = ratatui::widgets::Block::default()
            .borders(ratatui::widgets::Borders::ALL)
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

        let paragraph = ratatui::widgets::Paragraph::new(content)
            .style(
                Style::default()
                    .fg(self.theme.preview_fg)
                    .bg(self.theme.preview_bg),
            )
            .scroll_offset((0, 0));

        f.render_widget(block, area);
        f.render_widget(paragraph, area);
    }
}
