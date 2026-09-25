use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum SortOrder {
    Name,
    Size,
    Modified,
}

impl SortOrder {
    pub fn next(&self) -> Self {
        match self {
            SortOrder::Name => SortOrder::Size,
            SortOrder::Size => SortOrder::Modified,
            SortOrder::Modified => SortOrder::Name,
        }
    }
}

#[derive(Debug, Clone)]
pub struct FileEntry {
    pub name: String,
    pub path: PathBuf,
    pub is_dir: bool,
    pub size: u64,
    pub modified: Option<SystemTime>,
}

impl FileEntry {
    pub fn from_path(path: &Path, calculate_dir_size: bool) -> Self {
        let metadata = fs::metadata(path);
        let is_dir = path.is_dir();
        let mut size = metadata.as_ref().map(|m| m.len()).unwrap_or(0);
        if is_dir && calculate_dir_size {
            size = crate::fs::get_dir_size(&path.to_path_buf());
        }
        let modified = metadata.as_ref().ok().and_then(|m| m.modified().ok());

        Self {
            name: path
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string(),
            path: path.to_path_buf(),
            is_dir,
            size,
            modified,
        }
    }

    pub fn size_formatted(&self) -> String {
        // Convert size to megabytes
        let mb = self.size as f64 / (1024.0 * 1024.0);
        if mb >= 1.0 {
            if mb >= 1024.0 {
                format!("{:.1}G", mb / 1024.0)
            } else {
                format!("{:.1}M", mb)
            }
        } else {
            "<1M".to_string()
        }
    }

    pub fn modified_formatted(&self) -> String {
        if let Some(time) = self.modified {
            let datetime: chrono::DateTime<chrono::Local> = time.into();
            datetime.format("%Y-%m-%d %H:%M").to_string()
        } else {
            "--".to_string()
        }
    }
}

#[derive(Debug, Clone)]
pub struct AppState {
    pub cwd: PathBuf,
    pub entries: Vec<FileEntry>,
    pub selected: usize,
    pub sort_order: SortOrder,
    pub show_hidden: bool,
    pub preview_open: bool,
    pub show_git_diff: bool,
    pub calculate_dir_size: bool,
    pub selected_paths: HashSet<PathBuf>,
    pub file_filter: String,
    pub filter_active: bool,
}

impl AppState {
    pub fn new(cwd: PathBuf) -> Self {
        Self {
            cwd,
            entries: Vec::new(),
            selected: 0,
            sort_order: SortOrder::Name,
            show_hidden: false,
            preview_open: false,
            show_git_diff: false,
            calculate_dir_size: false,
            selected_paths: HashSet::new(),
            file_filter: String::new(),
            filter_active: false,
        }
    }

    pub fn refresh(&mut self) {
        self.load_dir();
    }

    pub fn load_dir(&mut self) {
        self.entries.clear();
        let path = &self.cwd;
        if let Ok(entries) = fs::read_dir(path) {
            for entry in entries.flatten() {
                let file_name = entry.file_name().to_string_lossy().to_string();
                if !self.show_hidden && file_name.starts_with('.') {
                    continue;
                }
                let file_entry = FileEntry::from_path(&entry.path(), self.calculate_dir_size);
                self.entries.push(file_entry);
            }
        }
        self.sort_entries();
        self.apply_filter();
        self.selected = self.selected.min(self.entries.len().saturating_sub(1));
    }

    pub fn apply_filter(&mut self) {
        if self.file_filter.is_empty() {
            return;
        }
        let filter_lower = self.file_filter.to_lowercase();
        self.entries
            .retain(|e| e.name.to_lowercase().contains(&filter_lower));
    }

    pub fn start_filter(&mut self) {
        self.filter_active = true;
        self.file_filter.clear();
    }

    pub fn cancel_filter(&mut self) {
        self.filter_active = false;
        self.file_filter.clear();
        self.load_dir();
    }

    pub fn push_filter_char(&mut self, c: char) {
        self.file_filter.push(c);
        self.apply_filter();
        self.selected = self.selected.min(self.entries.len().saturating_sub(1));
    }

    pub fn pop_filter_char(&mut self) {
        self.file_filter.pop();
        self.apply_filter();
        self.selected = self.selected.min(self.entries.len().saturating_sub(1));
    }

    pub fn sort_entries(&mut self) {
        match self.sort_order {
            SortOrder::Name => {
                self.entries.sort_by(|a, b| {
                    if a.is_dir != b.is_dir {
                        if a.is_dir {
                            return std::cmp::Ordering::Less;
                        } else {
                            return std::cmp::Ordering::Greater;
                        }
                    }
                    a.name.to_lowercase().cmp(&b.name.to_lowercase())
                });
            }
            SortOrder::Size => {
                self.entries.sort_by(|a, b| {
                    if a.is_dir != b.is_dir {
                        if a.is_dir {
                            return std::cmp::Ordering::Less;
                        } else {
                            return std::cmp::Ordering::Greater;
                        }
                    }
                    b.size.cmp(&a.size)
                });
            }
            SortOrder::Modified => {
                self.entries.sort_by(|a, b| {
                    if a.is_dir != b.is_dir {
                        if a.is_dir {
                            return std::cmp::Ordering::Less;
                        } else {
                            return std::cmp::Ordering::Greater;
                        }
                    }
                    b.modified.cmp(&a.modified)
                });
            }
        }
    }

    pub fn selected_entry(&self) -> Option<&FileEntry> {
        let has_parent = self.cwd.parent().is_some();
        let offset = if has_parent { 1 } else { 0 };
        let adjusted_idx = self.selected.saturating_sub(offset);
        self.entries.get(adjusted_idx)
    }

    pub fn cd_into(&mut self) -> bool {
        let has_parent_before = self.cwd.parent().is_some();
        let offset = if has_parent_before { 1 } else { 0 };
        let entry_idx = self.selected.saturating_sub(offset);
        if let Some(entry) = self.entries.get(entry_idx) {
            if entry.is_dir {
                self.cwd = entry.path.clone();
                self.load_dir();
                self.selected = 0;
                return true;
            }
        }
        false
    }

    pub fn cd_parent(&mut self) {
        if let Some(parent) = self.cwd.parent() {
            self.cwd = parent.to_path_buf();
            self.load_dir();
            self.selected = 0;
        }
    }

    pub fn is_parent_selected(&self) -> bool {
        self.cwd.parent().is_some() && self.selected == 0
    }

    pub fn toggle_selection(&mut self) {
        if self.is_parent_selected() {
            return;
        }
        let path = self.selected_entry().map(|e| e.path.clone());
        if let Some(path) = path {
            if self.selected_paths.contains(&path) {
                self.selected_paths.remove(&path);
            } else {
                self.selected_paths.insert(path);
            }
        }
    }

    pub fn clear_selection(&mut self) {
        self.selected_paths.clear();
    }

    pub fn select_by_path(&mut self, path: &Path) {
        let has_parent = self.cwd.parent().is_some();
        let base_offset = if has_parent { 1 } else { 0 };

        if let Some(pos) = self.entries.iter().position(|e| e.path == path) {
            self.selected = pos + base_offset;
        }
    }

    pub fn select_by_path_and_line(&mut self, path: &Path, _line: usize) {
        // For now, just select the file.
        // In the future, we could pass the line number to the editor if it opens.
        self.select_by_path(path);
    }
}
