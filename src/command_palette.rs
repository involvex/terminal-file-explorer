use crate::theme::Theme;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::Style;
use ratatui::widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph};
use ratatui::Frame;

use crate::menu::MenuAction;

#[derive(Debug, Clone)]
pub struct CommandItem {
    pub name: String,
    pub category: String,
    pub shortcut: String,
    pub action: MenuAction,
}

impl CommandItem {
    pub fn new(name: &str, category: &str, shortcut: &str, action: MenuAction) -> Self {
        Self {
            name: name.to_string(),
            category: category.to_string(),
            shortcut: shortcut.to_string(),
            action,
        }
    }

    pub fn display(&self) -> String {
        if self.shortcut.is_empty() {
            format!("{}: {}", self.category, self.name)
        } else {
            format!("{}: {}  [{}]", self.category, self.name, self.shortcut)
        }
    }

    pub fn matches_query(&self, query: &str) -> bool {
        let query_lower = query.to_lowercase();
        let display_lower = self.display().to_lowercase();
        display_lower.contains(&query_lower)
    }
}

#[derive(Debug, Clone)]
pub struct CommandPaletteState {
    pub query: String,
    pub all_items: Vec<CommandItem>,
    pub filtered: Vec<CommandItem>,
    pub selected: usize,
    pub active: bool,
}

impl CommandPaletteState {
    pub fn new() -> Self {
        let all_items = vec![
            CommandItem::new("New File", "File", "Ctrl+N", MenuAction::NewFile),
            CommandItem::new("New Folder", "File", "", MenuAction::NewFolder),
            CommandItem::new("Rename", "File", "Ctrl+R", MenuAction::Rename),
            CommandItem::new("Delete", "File", "Del", MenuAction::Delete),
            CommandItem::new("Quit", "File", "Ctrl+Q", MenuAction::Quit),
            CommandItem::new("Open in Editor", "Edit", "e", MenuAction::OpenEditor),
            CommandItem::new("Undo", "Edit", "Ctrl+Z", MenuAction::Undo),
            CommandItem::new("Redo", "Edit", "Ctrl+Y", MenuAction::Redo),
            CommandItem::new(
                "Toggle Hidden Files",
                "View",
                "Ctrl+P",
                MenuAction::ToggleHidden,
            ),
            CommandItem::new("Toggle Preview", "View", "Tab", MenuAction::TogglePreview),
            CommandItem::new("Cycle Sort Order", "View", "Ctrl+O", MenuAction::CycleSort),
            CommandItem::new("Cycle Theme", "View", "Ctrl+T", MenuAction::CycleTheme),
            CommandItem::new("Refresh", "View", "F5", MenuAction::Refresh),
            CommandItem::new("Go Up", "View", "\u{2190}", MenuAction::GoUp),
            CommandItem::new("Keybindings", "Help", "", MenuAction::ShowKeybindings),
            CommandItem::new("About", "Help", "", MenuAction::ShowAbout),
        ];

        let filtered = all_items.clone();
        Self {
            query: String::new(),
            all_items,
            filtered,
            selected: 0,
            active: false,
        }
    }

    pub fn open(&mut self) {
        self.active = true;
        self.query.clear();
        self.filtered = self.all_items.clone();
        self.selected = 0;
    }

    pub fn close(&mut self) {
        self.active = false;
        self.query.clear();
    }

    pub fn update_query(&mut self) {
        if self.query.is_empty() {
            self.filtered = self.all_items.clone();
        } else {
            self.filtered = self
                .all_items
                .iter()
                .filter(|item| item.matches_query(&self.query))
                .cloned()
                .collect();
        }
        if self.selected >= self.filtered.len() {
            self.selected = self.filtered.len().saturating_sub(1);
        }
    }

    pub fn move_up(&mut self) {
        if self.selected > 0 {
            self.selected -= 1;
        }
    }

    pub fn move_down(&mut self) {
        if self.selected < self.filtered.len().saturating_sub(1) {
            self.selected += 1;
        }
    }

    pub fn select_item(&self) -> Option<MenuAction> {
        self.filtered.get(self.selected).map(|i| i.action.clone())
    }

    pub fn handle_key_event(&mut self, key: crossterm::event::KeyEvent) -> CommandPaletteResult {
        use crossterm::event::{KeyCode, KeyModifiers};

        match key.code {
            KeyCode::Esc => {
                self.close();
                CommandPaletteResult::Close
            }
            KeyCode::Up => {
                self.move_up();
                CommandPaletteResult::Continue
            }
            KeyCode::Down => {
                self.move_down();
                CommandPaletteResult::Continue
            }
            KeyCode::Enter => {
                let action = self.select_item();
                self.close();
                if let Some(a) = action {
                    CommandPaletteResult::Action(a)
                } else {
                    CommandPaletteResult::Close
                }
            }
            KeyCode::Backspace => {
                self.query.pop();
                self.update_query();
                CommandPaletteResult::Continue
            }
            KeyCode::Char(c)
                if !key
                    .modifiers
                    .contains(KeyModifiers::CONTROL | KeyModifiers::ALT) =>
            {
                self.query.push(c);
                self.update_query();
                CommandPaletteResult::Continue
            }
            _ => CommandPaletteResult::Continue,
        }
    }
}

pub enum CommandPaletteResult {
    Continue,
    Close,
    Action(MenuAction),
}

pub fn draw_command_palette(state: &CommandPaletteState, theme: &Theme, area: Rect, f: &mut Frame) {
    let popup_width = (area.width as f32 * 0.6) as u16;
    let popup_height = (area.height as f32 * 0.5) as u16;
    let popup_x = area.x + (area.width.saturating_sub(popup_width)) / 2;
    let popup_y = area.y + area.height / 6;

    let popup_area = Rect {
        x: popup_x,
        y: popup_y,
        width: popup_width.min(area.width),
        height: popup_height.min(area.height.saturating_sub(popup_y)),
    };

    f.render_widget(Clear, popup_area);

    let chunks = Layout::new(
        Direction::Vertical,
        [Constraint::Length(3), Constraint::Min(1)],
    )
    .split(popup_area);

    let input_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.palette_selected_bg))
        .style(Style::default().bg(theme.palette_bg).fg(theme.palette_fg))
        .title(" Command Palette ");

    f.render_widget(&input_block, chunks[0]);
    let input_inner = input_block.inner(chunks[0]);

    let input_text = if state.query.is_empty() {
        "Type to search...".to_string()
    } else {
        state.query.clone()
    };
    let input_style = if state.query.is_empty() {
        Style::default().fg(theme.border).bg(theme.palette_bg)
    } else {
        Style::default().fg(theme.palette_fg).bg(theme.palette_bg)
    };
    f.render_widget(Paragraph::new(input_text).style(input_style), input_inner);

    let cursor_x = input_inner.x + state.query.len() as u16;
    let cursor_y = input_inner.y;
    f.set_cursor_position((cursor_x, cursor_y));

    let list_block = Block::default()
        .borders(Borders::LEFT | Borders::RIGHT | Borders::BOTTOM)
        .border_style(Style::default().fg(theme.border))
        .style(Style::default().bg(theme.palette_bg).fg(theme.palette_fg));
    f.render_widget(&list_block, chunks[1]);
    let list_inner = list_block.inner(chunks[1]);

    let items: Vec<ListItem> = state
        .filtered
        .iter()
        .enumerate()
        .map(|(i, item)| {
            let style = if i == state.selected {
                Style::default()
                    .fg(theme.palette_selected_fg)
                    .bg(theme.palette_selected_bg)
            } else {
                Style::default().fg(theme.palette_fg).bg(theme.palette_bg)
            };
            ListItem::new(item.display()).style(style)
        })
        .collect();

    let max_visible = list_inner.height as usize;
    let scroll_offset = if state.selected >= max_visible {
        state.selected - max_visible + 1
    } else {
        0
    };

    let mut list_state = ListState::default();
    list_state.select(Some(state.selected.saturating_sub(scroll_offset)));

    f.render_stateful_widget(List::new(items), list_inner, &mut list_state);
}
