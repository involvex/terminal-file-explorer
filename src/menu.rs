use crate::theme::Theme;
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::Frame;

#[derive(Debug, Clone, PartialEq)]
pub enum MenuAction {
    NewFile,
    NewFolder,
    Rename,
    Delete,
    Quit,
    Undo,
    Redo,
    ToggleHidden,
    TogglePreview,
    CycleSort,
    CycleTheme,
    Refresh,
    OpenEditor,
    GoUp,
    ShowKeybindings,
    ShowAbout,
    OpenCommandPalette,
}

#[derive(Debug, Clone)]
pub struct MenuItem {
    pub label: String,
    pub shortcut: String,
    pub action: Option<MenuAction>,
    pub separator: bool,
}

impl MenuItem {
    pub fn action(label: &str, shortcut: &str, action: MenuAction) -> Self {
        Self {
            label: label.to_string(),
            shortcut: shortcut.to_string(),
            action: Some(action),
            separator: false,
        }
    }

    pub fn separator() -> Self {
        Self {
            label: String::new(),
            shortcut: String::new(),
            action: None,
            separator: true,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Menu {
    pub name: String,
    pub items: Vec<MenuItem>,
}

#[derive(Debug, Clone)]
pub struct MenuBarState {
    pub menus: Vec<Menu>,
    pub open_index: Option<usize>,
    pub selected_item: Option<usize>,
}

impl MenuBarState {
    pub fn new() -> Self {
        let file_menu = Menu {
            name: "File".to_string(),
            items: vec![
                MenuItem::action("New File", "Ctrl+N", MenuAction::NewFile),
                MenuItem::action("New Folder", "", MenuAction::NewFolder),
                MenuItem::separator(),
                MenuItem::action("Rename", "Ctrl+R", MenuAction::Rename),
                MenuItem::action("Delete", "Del", MenuAction::Delete),
                MenuItem::separator(),
                MenuItem::action("Quit", "Ctrl+Q", MenuAction::Quit),
            ],
        };

        let edit_menu = Menu {
            name: "Edit".to_string(),
            items: vec![
                MenuItem::action("Open in Editor", "e", MenuAction::OpenEditor),
                MenuItem::separator(),
                MenuItem::action("Undo", "Ctrl+Z", MenuAction::Undo),
                MenuItem::action("Redo", "Ctrl+Y", MenuAction::Redo),
            ],
        };

        let view_menu = Menu {
            name: "View".to_string(),
            items: vec![
                MenuItem::action("Toggle Hidden Files", "Ctrl+H", MenuAction::ToggleHidden),
                MenuItem::action("Toggle Preview", "Tab", MenuAction::TogglePreview),
                MenuItem::action("Cycle Sort Order", "Ctrl+O", MenuAction::CycleSort),
                MenuItem::action("Cycle Theme", "Ctrl+T", MenuAction::CycleTheme),
                MenuItem::separator(),
                MenuItem::action("Refresh", "F5", MenuAction::Refresh),
                MenuItem::action("Go Up", "\u{2190}", MenuAction::GoUp),
            ],
        };

        let help_menu = Menu {
            name: "Help".to_string(),
            items: vec![
                MenuItem::action("Keybindings", "", MenuAction::ShowKeybindings),
                MenuItem::action("About", "", MenuAction::ShowAbout),
            ],
        };

        Self {
            menus: vec![file_menu, edit_menu, view_menu, help_menu],
            open_index: None,
            selected_item: None,
        }
    }

    pub fn open(&mut self, index: usize) {
        self.open_index = Some(index);
        self.selected_item = Some(0);
    }

    pub fn close(&mut self) {
        self.open_index = None;
        self.selected_item = None;
    }

    pub fn is_open(&self) -> bool {
        self.open_index.is_some()
    }

    pub fn move_left(&mut self) {
        if let Some(idx) = self.open_index {
            if idx == 0 {
                self.open_index = Some(self.menus.len() - 1);
            } else {
                self.open_index = Some(idx - 1);
            }
            self.selected_item = Some(0);
        }
    }

    pub fn move_right(&mut self) {
        if let Some(idx) = self.open_index {
            self.open_index = Some((idx + 1) % self.menus.len());
            self.selected_item = Some(0);
        }
    }

    pub fn move_up(&mut self) {
        if let Some(idx) = self.open_index {
            if let Some(sel) = self.selected_item {
                let menu = &self.menus[idx];
                let new_sel = if sel == 0 {
                    menu.items.len() - 1
                } else {
                    sel - 1
                };
                self.selected_item = Some(new_sel);
            }
        }
    }

    pub fn move_down(&mut self) {
        if let Some(idx) = self.open_index {
            if let Some(sel) = self.selected_item {
                let menu = &self.menus[idx];
                let new_sel = if sel >= menu.items.len() - 1 {
                    0
                } else {
                    sel + 1
                };
                self.selected_item = Some(new_sel);
            }
        }
    }

    pub fn select_item(&self) -> Option<MenuAction> {
        if let Some(idx) = self.open_index {
            if let Some(sel) = self.selected_item {
                let menu = &self.menus[idx];
                if let Some(item) = menu.items.get(sel) {
                    return item.action.clone();
                }
            }
        }
        None
    }

    pub fn menu_bar_column_offset(&self, index: usize) -> u16 {
        let mut offset: u16 = 0;
        for (i, menu) in self.menus.iter().enumerate() {
            if i == index {
                return offset;
            }
            offset += menu.name.len() as u16 + 3;
        }
        offset
    }

    pub fn handle_dropdown_click(&mut self, row: u16, col: u16) -> Option<MenuAction> {
        let open_idx = self.open_index?;
        let menu = &self.menus[open_idx];
        let menu_x = self.menu_bar_column_offset(open_idx);

        let max_label_len: usize = menu
            .items
            .iter()
            .map(|i| {
                if i.separator {
                    0
                } else {
                    i.label.len() + i.shortcut.len() + 4
                }
            })
            .max()
            .unwrap_or(10);
        let dropdown_width = (max_label_len + 4).max(14) as u16;

        if col < menu_x || col >= menu_x + dropdown_width {
            self.close();
            return None;
        }

        let dropdown_top: u16 = 1;
        let inner_left: u16 = menu_x + 1;
        let inner_right: u16 = menu_x + dropdown_width - 1;

        if row <= dropdown_top || col < inner_left || col >= inner_right {
            return None;
        }

        let item_row = row - dropdown_top - 1;
        if item_row as usize >= menu.items.len() {
            return None;
        }

        let item = &menu.items[item_row as usize];
        if item.separator {
            return None;
        }

        let action = item.action.clone();
        self.close();
        action
    }

    pub fn handle_key_event(&mut self, key: crossterm::event::KeyEvent) -> Option<MenuAction> {
        use crossterm::event::{KeyCode, KeyModifiers};

        match key.code {
            KeyCode::Esc => {
                self.close();
                None
            }
            KeyCode::Left => {
                self.move_left();
                None
            }
            KeyCode::Right => {
                self.move_right();
                None
            }
            KeyCode::Up => {
                self.move_up();
                None
            }
            KeyCode::Down => {
                self.move_down();
                None
            }
            KeyCode::Enter => {
                let action = self.select_item();
                self.close();
                action
            }
            KeyCode::Char('p') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.close();
                Some(MenuAction::OpenCommandPalette)
            }
            KeyCode::F(10) => {
                self.close();
                None
            }
            _ => None,
        }
    }
}

pub fn draw_menu_bar(state: &MenuBarState, theme: &Theme, area: Rect, f: &mut Frame) {
    let mut menu_positions: Vec<(u16, u16)> = Vec::new();

    let mut spans: Vec<Span> = Vec::new();
    for (i, menu) in state.menus.iter().enumerate() {
        let is_open = state.open_index == Some(i);
        let name_style = if is_open {
            Style::default()
                .fg(theme.menu_selected_fg)
                .bg(theme.menu_selected_bg)
        } else {
            Style::default().fg(theme.menu_fg).bg(theme.menu_bg)
        };
        let hotkey_style = if is_open {
            Style::default()
                .fg(theme.menu_selected_fg)
                .bg(theme.menu_selected_bg)
                .add_modifier(ratatui::style::Modifier::UNDERLINED)
        } else {
            Style::default().fg(theme.menu_hotkey_fg).bg(theme.menu_bg)
        };

        let start_x = area.x
            + spans
                .iter()
                .map(|s: &Span| s.content.len() as u16)
                .sum::<u16>();
        let spacer = Span::raw(" ");
        let first_char = Span::styled(
            menu.name.chars().next().unwrap_or('_').to_string(),
            hotkey_style,
        );
        let rest_chars = Span::styled(menu.name.chars().skip(1).collect::<String>(), name_style);
        let trailing = Span::styled(" ", name_style);

        let section_start = start_x;
        spans.push(spacer);
        spans.push(first_char);
        spans.push(rest_chars);
        spans.push(trailing);
        let section_end = area.x
            + spans
                .iter()
                .map(|s: &Span| s.content.len() as u16)
                .sum::<u16>();
        menu_positions.push((section_start, section_end));

        if i < state.menus.len() - 1 {
            spans.push(Span::styled(
                " ",
                Style::default().fg(theme.menu_fg).bg(theme.menu_bg),
            ));
        }
    }

    let line = Line::from(spans);
    f.render_widget(Paragraph::new(line), area);

    if let Some(open_idx) = state.open_index {
        let (start, _end) = menu_positions[open_idx];
        let menu = &state.menus[open_idx];

        let max_label_len: usize = menu
            .items
            .iter()
            .map(|i| {
                if i.separator {
                    0
                } else {
                    i.label.len() + i.shortcut.len() + 4
                }
            })
            .max()
            .unwrap_or(10);
        let dropdown_width = (max_label_len + 4).max(14) as u16;
        let dropdown_height = (menu.items.len() + 2) as u16;

        let dropdown_x = start;
        let dropdown_y = area.y + 1;

        let dropdown_area = Rect {
            x: dropdown_x,
            y: dropdown_y,
            width: dropdown_width.min(f.area().width.saturating_sub(dropdown_x)),
            height: dropdown_height.min(f.area().height.saturating_sub(dropdown_y)),
        };

        let dropdown_block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(theme.border))
            .style(Style::default().bg(theme.menu_bg).fg(theme.menu_fg));
        f.render_widget(ratatui::widgets::Clear, dropdown_area);
        f.render_widget(&dropdown_block, dropdown_area);

        let inner = dropdown_block.inner(dropdown_area);

        let mut y_offset = 0u16;
        for (i, item) in menu.items.iter().enumerate() {
            if y_offset as usize >= inner.height as usize {
                break;
            }

            if item.separator {
                let sep_area = Rect {
                    x: inner.x,
                    y: inner.y + y_offset,
                    width: inner.width,
                    height: 1,
                };
                f.render_widget(
                    Paragraph::new("\u{2500}".repeat(inner.width as usize))
                        .style(Style::default().fg(theme.separator_fg)),
                    sep_area,
                );
                y_offset += 1;
                continue;
            }

            let is_selected = state.selected_item == Some(i);
            let row_style = if is_selected {
                Style::default()
                    .fg(theme.menu_selected_fg)
                    .bg(theme.menu_selected_bg)
            } else {
                Style::default().fg(theme.menu_fg).bg(theme.menu_bg)
            };

            let label_padded = format!(
                " {:<width$}{} ",
                item.label,
                item.shortcut,
                width = (inner.width as usize).saturating_sub(item.shortcut.len() + 3)
            );

            let item_area = Rect {
                x: inner.x,
                y: inner.y + y_offset,
                width: inner.width,
                height: 1,
            };
            f.render_widget(Paragraph::new(label_padded).style(row_style), item_area);
            y_offset += 1;
        }
    }
}
