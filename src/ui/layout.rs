use crate::state::AppState;
use crate::theme::Theme;
use crate::ui::file_list::FileList;
use crate::ui::preview::Preview;
use crate::ui::status::StatusBar;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::widgets::Widget;

pub fn draw(state: &AppState, theme: &Theme, area: Rect, f: &mut ratatui::Frame) {
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
        vec![area]
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
    let title_widget = ratatui::widgets::Paragraph::new(title)
        .style(theme.title)
        .bg(theme.status_bg);

    let border = ratatui::widgets::Block::default()
        .borders(ratatui::widgets::Borders::TOP)
        .border_style(theme.border);

    f.render_widget(title_widget.clone(), file_list_area[0]);

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
        let preview_title_widget = ratatui::widgets::Paragraph::new(preview_title)
            .style(theme.title)
            .bg(theme.status_bg);
        f.render_widget(preview_title_widget, preview_area[0]);

        let preview_widget = Preview::new(state, theme);
        f.render_widget(preview_widget, preview_area[1]);
    }

    if state.in_editor {
        let editor_area = centered_rect(90, 90, area);
        let editor_block = ratatui::widgets::Block::default()
            .title(" Editor ")
            .border_style(theme.border)
            .borders(ratatui::widgets::Borders::ALL);
        let editor_content = ratatui::widgets::Paragraph::new(&state.editor_content)
            .style(theme.editor_fg)
            .bg(theme.editor_bg)
            .scroll_offset((0, 0));
        f.render_widget(editor_block, editor_area);
    }
}

fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
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
