use std::fs;
use std::path::PathBuf;

pub fn create_file(path: &PathBuf) -> Result<(), String> {
    fs::write(path, "").map_err(|e| e.to_string())
}

pub fn create_folder(path: &PathBuf) -> Result<(), String> {
    fs::create_dir(path).map_err(|e| e.to_string())
}

pub fn delete_path(path: &PathBuf) -> Result<(), String> {
    if path.is_dir() {
        fs::remove_dir(path).map_err(|e| e.to_string())
    } else {
        fs::remove_file(path).map_err(|e| e.to_string())
    }
}

pub fn rename_path(old: &PathBuf, new: &PathBuf) -> Result<(), String> {
    fs::rename(old, new).map_err(|e| e.to_string())
}

pub fn read_file(path: &PathBuf) -> Result<String, String> {
    fs::read_to_string(path).map_err(|e| e.to_string())
}

pub fn write_file(path: &PathBuf, content: &str) -> Result<(), String> {
    fs::write(path, content).map_err(|e| e.to_string())
}

pub fn get_dir_size(path: &PathBuf) -> u64 {
    let mut size = 0;
    if let Ok(entries) = fs::read_dir(path) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                size += get_dir_size(&path);
            } else if let Ok(metadata) = fs::metadata(path) {
                size += metadata.len();
            }
        }
    }
    size
}
