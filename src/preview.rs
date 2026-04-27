use crate::git::GitStatus;
use ratatui::style::{Color as RatatuiColor, Style};
use ratatui::text::{Line, Span, Text};
use std::fs;
use std::path::Path;
use syntect::easy::HighlightLines;
use syntect::highlighting::ThemeSet;
use syntect::parsing::SyntaxSet;

pub fn get_preview_text<'a>(
    path: &Path,
    name: &str,
    max_width: usize,
    max_height: usize,
    syntax_set: &'a SyntaxSet,
    theme_set: &'a ThemeSet,
) -> Text<'a> {
    let extension = name.split('.').next_back().unwrap_or("").to_lowercase();

    if path.is_dir() {
        return Text::from(format!("[DIR] {}", name));
    }

    match extension.as_str() {
        "txt" | "md" | "rs" | "js" | "ts" | "py" | "go" | "java" | "c" | "cpp" | "h" | "hpp"
        | "toml" | "yaml" | "yml" | "json" | "xml" | "html" | "css" | "sh" | "bash" | "zsh" => {
            preview_highlighted(path, max_width, max_height, syntax_set, theme_set)
        }
        "png" | "jpg" | "jpeg" | "gif" | "bmp" | "ico" | "webp" => Text::from(preview_image(path)),
        "mp3" | "flac" | "wav" | "ogg" | "m4a" | "aac" => Text::from(preview_audio(path)),
        "pdf" => Text::from(preview_pdf(path)),
        _ => preview_highlighted(path, max_width, max_height, syntax_set, theme_set),
    }
}

fn preview_highlighted<'a>(
    path: &Path,
    max_width: usize,
    max_height: usize,
    syntax_set: &'a SyntaxSet,
    theme_set: &'a ThemeSet,
) -> Text<'a> {
    if let Ok(content) = fs::read_to_string(path) {
        let syntax = syntax_set
            .find_syntax_by_extension(path.extension().and_then(|e| e.to_str()).unwrap_or(""))
            .unwrap_or_else(|| syntax_set.find_syntax_plain_text());

        let theme = &theme_set.themes["base16-ocean.dark"];
        let mut h = HighlightLines::new(syntax, theme);

        let mut lines = Vec::new();
        for line_str in content.lines().take(max_height) {
            let ranges: Vec<(syntect::highlighting::Style, &str)> =
                h.highlight_line(line_str, syntax_set).unwrap_or_default();
            let mut spans = Vec::new();
            let mut current_width = 0;

            for (style, text) in ranges {
                if current_width >= max_width {
                    break;
                }

                let available = max_width - current_width;
                let (display_text, truncated) = if text.len() > available {
                    (&text[..available], true)
                } else {
                    (text, false)
                };

                let fg =
                    RatatuiColor::Rgb(style.foreground.r, style.foreground.g, style.foreground.b);
                spans.push(Span::styled(
                    display_text.to_string(),
                    Style::default().fg(fg),
                ));
                current_width += display_text.len();

                if truncated {
                    break;
                }
            }
            lines.push(Line::from(spans));
        }
        Text::from(lines)
    } else {
        Text::from("[Cannot read file]")
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
