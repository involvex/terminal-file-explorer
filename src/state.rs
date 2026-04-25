use std::fs;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

#[derive(Debug, Clone, Copy, PartialEq)]
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
    pub fn from_path(path: &Path) -> Self {
        let metadata = fs::metadata(path);
        let is_dir = path.is_dir();
        let size = metadata.as_ref().map(|m| m.len()).unwrap_or(0);
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
        if self.is_dir {
            return "<DIR>".to_string();
        }
        const KB: u64 = 1024;
        const MB: u64 = KB * 1024;
        const GB: u64 = MB * 1024;
        if self.size >= GB {
            format!("{:.1}G", self.size as f64 / GB as f64)
        } else if self.size >= MB {
            format!("{:.1}M", self.size as f64 / MB as f64)
        } else if self.size >= KB {
            format!("{:.1}K", self.size as f64 / KB as f64)
        } else {
            format!("{}B", self.size)
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
                let file_entry = FileEntry::from_path(&entry.path());
                self.entries.push(file_entry);
            }
        }
        self.sort_entries();
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
        let has_parent = self.cwd.parent().is_some();
        if has_parent && self.selected == 0 {
            return false;
        }
        let offset = if has_parent { 1 } else { 0 };
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
}
