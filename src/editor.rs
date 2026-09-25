#[derive(Debug, Clone)]
pub struct EditorState {
    pub content: Vec<String>,
    pub path: Option<std::path::PathBuf>,
    pub modified: bool,
    pub active: bool,
    pub cursor_line: usize,
    pub cursor_col: usize,
    pub scroll_offset: usize,
    undo_stack: Vec<Vec<String>>,
    redo_stack: Vec<Vec<String>>,
}

impl EditorState {
    pub fn new() -> Self {
        Self {
            content: Vec::new(),
            path: None,
            modified: false,
            active: false,
            cursor_line: 0,
            cursor_col: 0,
            scroll_offset: 0,
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
        }
    }

    pub fn open(&mut self, content: Vec<String>, path: std::path::PathBuf) {
        self.content = content;
        self.path = Some(path);
        self.modified = false;
        self.active = true;
        self.cursor_line = 0;
        self.cursor_col = 0;
        self.scroll_offset = 0;
        self.undo_stack.clear();
        self.redo_stack.clear();
    }

    pub fn save(&mut self) -> bool {
        if let Some(path) = &self.path {
            let content = self.content.join("\n");
            if crate::fs::write_file(path, &content).is_ok() {
                self.modified = false;
                return true;
            }
        }
        false
    }

    pub fn close(&mut self) {
        self.active = false;
        self.path = None;
        self.content.clear();
        self.modified = false;
        self.cursor_line = 0;
        self.cursor_col = 0;
        self.scroll_offset = 0;
        self.undo_stack.clear();
        self.redo_stack.clear();
    }

    pub fn push_undo(&mut self) {
        self.undo_stack.push(self.content.clone());
        self.redo_stack.clear();
    }

    pub fn undo(&mut self) {
        if let Some(prev) = self.undo_stack.pop() {
            self.redo_stack.push(self.content.clone());
            self.content = prev;
            self.modified = true;
        }
    }

    pub fn redo(&mut self) {
        if let Some(next) = self.redo_stack.pop() {
            self.undo_stack.push(self.content.clone());
            self.content = next;
            self.modified = true;
        }
    }

    pub fn insert_char(&mut self, c: char) {
        self.push_undo();
        if self.cursor_line >= self.content.len() {
            self.content.push(String::new());
        }
        self.content[self.cursor_line].insert(self.cursor_col, c);
        self.cursor_col += 1;
        self.modified = true;
    }

    pub fn backspace(&mut self) {
        if self.cursor_col > 0 {
            self.push_undo();
            self.content[self.cursor_line].remove(self.cursor_col - 1);
            self.cursor_col -= 1;
            self.modified = true;
        } else if self.cursor_line > 0 {
            self.push_undo();
            let current = self.content.remove(self.cursor_line);
            self.cursor_line -= 1;
            self.cursor_col = self.content[self.cursor_line].len();
            self.content[self.cursor_line].push_str(&current);
            self.modified = true;
        }
    }

    pub fn delete_char(&mut self) {
        let line_len = self
            .content
            .get(self.cursor_line)
            .map(|l| l.len())
            .unwrap_or(0);
        if self.cursor_col < line_len {
            self.push_undo();
            self.content[self.cursor_line].remove(self.cursor_col);
            self.modified = true;
        } else if self.cursor_line < self.content.len().saturating_sub(1) {
            self.push_undo();
            let next_line = self.content.remove(self.cursor_line + 1);
            self.content[self.cursor_line].push_str(&next_line);
            self.modified = true;
        }
    }

    pub fn newline(&mut self) {
        self.push_undo();
        if self.cursor_line >= self.content.len() {
            self.content.push(String::new());
        }
        let current = self.content[self.cursor_line].clone();
        let new_line = if self.cursor_col >= current.len() {
            String::new()
        } else {
            current[self.cursor_col..].to_string()
        };
        self.content[self.cursor_line] = if self.cursor_col >= current.len() {
            current
        } else {
            current[..self.cursor_col].to_string()
        };
        self.content.insert(self.cursor_line + 1, new_line);
        self.cursor_line += 1;
        self.cursor_col = 0;
        self.modified = true;
    }

    pub fn insert_tab(&mut self) {
        self.push_undo();
        let indent = "    ";
        if self.cursor_line >= self.content.len() {
            self.content.push(String::new());
        }
        self.content[self.cursor_line].insert_str(self.cursor_col, indent);
        self.cursor_col += indent.len();
        self.modified = true;
    }
}
