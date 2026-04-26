use std::fs;
use std::path::Path;
use crate::git::GitStatus;

pub fn get_preview_content(path: &Path, name: &str, max_width: usize, max_height: usize) -> String {
    let extension = name.split('.').last().unwrap_or("").to_lowercase();

    if path.is_dir() {
        return format!("[DIR] {}", name);
    }

    match extension.as_str() {
        "txt" | "md" | "rs" | "js" | "ts" | "py" | "go" | "java" | "c" | "cpp" | "h" | "hpp"
        | "toml" | "yaml" | "yml" | "json" | "xml" | "html" | "css" | "sh" | "bash" | "zsh" => {
            preview_text(path, max_width, max_height)
        }
        "png" | "jpg" | "jpeg" | "gif" | "bmp" | "ico" | "webp" => preview_image(path),
        "mp3" | "flac" | "wav" | "ogg" | "m4a" | "aac" => preview_audio(path),
        "pdf" => preview_pdf(path),
        _ => preview_text(path, max_width, max_height),
    }
}

fn preview_text(path: &Path, max_width: usize, max_height: usize) -> String {
    if let Ok(content) = fs::read_to_string(path) {
        let lines: Vec<&str> = content.lines().take(max_height).collect();
        let mut result = String::new();
        for line in lines {
            let truncated = if line.len() > max_width {
                &line[..max_width]
            } else {
                line
            };
            result.push_str(truncated);
            result.push('\n');
        }
        result
    } else {
        "[Cannot read file]".to_string()
    }
}

fn preview_image(path: &Path) -> String {
    format!("[Image: {}]", path.display())
}

fn preview_audio(path: &Path) -> String {
    use id3::TagLike;
    if let Ok(tag) = id3::Tag::read_from_path(path) {
        let title = tag.title().unwrap_or("Unknown");
        let artist = tag.artist().unwrap_or("Unknown");
        let album = tag.album().unwrap_or("Unknown");
        let duration = tag.duration().unwrap_or(0);
        let minutes = duration / 60;
        let seconds = duration % 60;
        format!(
            "[Audio]\nTitle: {}\nArtist: {}\nAlbum: {}\nDuration: {}:{:02}",
            title, artist, album, minutes, seconds
        )
    } else {
        format!("[Audio: {}]", path.display())
    }
}

fn preview_pdf(path: &Path) -> String {
    format!("[PDF: {}]", path.display())
}

pub fn get_git_diff_preview(path: &Path, name: &str) -> String {
    if let Some(diff) = GitStatus::get_file_diff(path, name) {
        if diff.is_empty() {
            "No changes.".to_string()
        } else {
            diff
        }
    } else {
        "[No Git diff available]".to_string()
    }
}
