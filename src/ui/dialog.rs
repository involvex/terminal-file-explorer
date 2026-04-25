use ratatui::{
    layout::Rect,
    style::Style,
    widgets::{Block, Borders, Paragraph, Widget},
    Frame,
};

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
    theme: &'a crate::theme::Theme,
}

impl<'a> Dialog<'a> {
    pub fn new(dialog_state: &'a DialogState, theme: &'a crate::theme::Theme) -> Self {
        Self {
            dialog_state,
            theme,
        }
    }

    pub fn render(self, area: Rect, f: &mut Frame) {
        if self.dialog_state.dialog_type == DialogType::None {
            return;
        }

        let (title, instruction) = match self.dialog_state.dialog_type {
            DialogType::NewFile => ("New File", "Enter filename:"),
            DialogType::NewFolder => ("New Folder", "Enter folder name:"),
            DialogType::Rename => ("Rename", "Enter new name:"),
            DialogType::Delete => ("Delete", "Confirm deletion (y/n):"),
            DialogType::None => return,
        };

        let block = Block::default()
            .title(format!(" {} ", title))
            .borders(Borders::ALL)
            .border_style(self.theme.border)
            .style(Style::default().bg(self.theme.bg).fg(self.theme.fg));

        let content = if self.dialog_state.dialog_type == DialogType::Delete {
            format!(
                "{}\n\n{}\n\n{}",
                instruction,
                self.dialog_state.input,
                "Press 'y' to confirm, 'n' or 'Esc' to cancel"
            )
        } else {
            format!(
                "{}\n\n> {}\n\n[Enter] Confirm  [Esc] Cancel",
                instruction, self.dialog_state.input
            )
        };

        let paragraph =
            Paragraph::new(content).style(Style::default().bg(self.theme.bg).fg(self.theme.fg));

        let inner_area = block.inner(area);
        f.render_widget(block, area);
        f.render_widget(paragraph, inner_area);
    }
}
