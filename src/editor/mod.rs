use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct EditorState {
    pub content: Vec<String>,
    pub cursor_line: usize,
    pub cursor_col: usize,
    pub scroll_offset: usize,
    pub modified: bool,
}

impl Default for EditorState {
    fn default() -> Self {
        Self {
            content: Vec::new(),
            cursor_line: 0,
            cursor_col: 0,
            scroll_offset: 0,
            modified: false,
        }
    }
}

impl EditorState {
    pub fn new(content: String) -> Self {
        let content: Vec<String> = content.lines().map(|s| s.to_string()).collect();
        Self {
            content,
            cursor_line: 0,
            cursor_col: 0,
            scroll_offset: 0,
            modified: false,
        }
    }

    pub fn to_string(&self) -> String {
        self.content.join("\n")
    }

    pub fn insert_char(&mut self, ch: char) {
        if self.cursor_line >= self.content.len() {
            self.content.push(String::new());
        }
        let line = &mut self.content[self.cursor_line];
        if self.cursor_col > line.len() {
            self.cursor_col = line.len();
        }
        line.insert(self.cursor_col, ch);
        self.cursor_col += 1;
        self.modified = true;
    }

    pub fn insert_newline(&mut self) {
        if self.cursor_line >= self.content.len() {
            self.content.push(String::new());
        }
        let current_line = self.content[self.cursor_line].clone();
        let new_line = if self.cursor_col >= current_line.len() {
            String::new()
        } else {
            current_line[self.cursor_col..].to_string()
        };
        self.content[self.cursor_line] =
            current_line[..self.cursor_col.min(current_line.len())].to_string();
        self.content.insert(self.cursor_line + 1, new_line);
        self.cursor_line += 1;
        self.cursor_col = 0;
        self.modified = true;
    }

    pub fn delete_char(&mut self) {
        if self.cursor_line >= self.content.len() {
            return;
        }
        let line = &mut self.content[self.cursor_line];
        if self.cursor_col == 0 {
            if self.cursor_line > 0 {
                let prev_line_len = self.content[self.cursor_line - 1].len();
                let current_line_text = self.content.remove(self.cursor_line);
                self.content[self.cursor_line - 1].push_str(&current_line_text);
                self.cursor_line -= 1;
                self.cursor_col = prev_line_len;
                self.modified = true;
            }
        } else if self.cursor_col <= line.len() {
            line.remove(self.cursor_col - 1);
            self.cursor_col -= 1;
            self.modified = true;
        }
    }

    pub fn move_up(&mut self) {
        if self.cursor_line > 0 {
            self.cursor_line -= 1;
            if self.cursor_col > self.content[self.cursor_line].len() {
                self.cursor_col = self.content[self.cursor_line].len();
            }
        }
    }

    pub fn move_down(&mut self) {
        if self.cursor_line < self.content.len().saturating_sub(1) {
            self.cursor_line += 1;
            if self.cursor_col > self.content[self.cursor_line].len() {
                self.cursor_col = self.content[self.cursor_line].len();
            }
        }
    }

    pub fn move_left(&mut self) {
        if self.cursor_col > 0 {
            self.cursor_col -= 1;
        } else if self.cursor_line > 0 {
            self.cursor_line -= 1;
            self.cursor_col = self.content[self.cursor_line].len();
        }
    }

    pub fn move_right(&mut self) {
        if self.cursor_line < self.content.len() {
            let line_len = self.content[self.cursor_line].len();
            if self.cursor_col < line_len {
                self.cursor_col += 1;
            } else if self.cursor_line < self.content.len() - 1 {
                self.cursor_line += 1;
                self.cursor_col = 0;
            }
        }
    }

    pub fn move_to_start(&mut self) {
        self.cursor_col = 0;
    }

    pub fn move_to_end(&mut self) {
        if self.cursor_line < self.content.len() {
            self.cursor_col = self.content[self.cursor_line].len();
        }
    }

    pub fn update_scroll(&mut self, visible_lines: usize) {
        if self.cursor_line < self.scroll_offset {
            self.scroll_offset = self.cursor_line;
        } else if self.cursor_line >= self.scroll_offset + visible_lines {
            self.scroll_offset = self.cursor_line - visible_lines + 1;
        }
    }
}

pub fn render_editor(
    state: &EditorState,
    theme: &crate::theme::Theme,
    width: usize,
    height: usize,
) -> String {
    let visible_lines = height.saturating_sub(2);
    let scroll = state.scroll_offset;

    let mut output = String::new();

    for i in 0..visible_lines {
        let line_idx = scroll + i;
        if line_idx >= state.content.len() {
            output.push_str(&format!("~\n"));
        } else {
            let line = &state.content[line_idx];
            let display_line = if line.len() > width - 2 {
                format!("{}{}", &line[..width - 2], "<")
            } else {
                line.clone()
            };

            let cursor_marker = if line_idx == state.cursor_line {
                let mut marker = String::new();
                for j in 0..width {
                    if j == state.cursor_col {
                        marker.push('|');
                    } else if j < display_line.len() {
                        marker.push(display_line.chars().nth(j).unwrap_or(' '));
                    } else {
                        marker.push(' ');
                    }
                }
                marker
            } else {
                format!(
                    "{}{}",
                    display_line,
                    " ".repeat(width.saturating_sub(display_line.len()))
                )
            };

            output.push_str(&cursor_marker);
            output.push('\n');
        }
    }

    output
}
