use crate::state::SortOrder;
use std::fs;
use std::path::PathBuf;

pub struct Config {
    pub theme: String,
    pub show_hidden: bool,
    pub preview_enabled: bool,
    pub calculate_dir_size: bool,
    pub sort_order: SortOrder,
    pub bookmarks: Vec<PathBuf>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            theme: "tokyo-night".to_string(),
            show_hidden: false,
            preview_enabled: true,
            calculate_dir_size: false,
            sort_order: SortOrder::Name,
            bookmarks: Vec::new(),
        }
    }
}

impl Config {
    pub fn load() -> Self {
        if let Some(proj_dirs) =
            directories::ProjectDirs::from("com", "tfe", "terminal-file-explorer")
        {
            let config_path = proj_dirs.config_dir().join("config.toml");
            if config_path.exists() {
                if let Ok(content) = fs::read_to_string(&config_path) {
                    if let Ok(config) = toml::from_str::<TomlConfig>(&content) {
                        return Self {
                            theme: config.theme.unwrap_or_else(|| "tokyo-night".to_string()),
                            show_hidden: config.show_hidden.unwrap_or(false),
                            preview_enabled: config.preview_enabled.unwrap_or(true),
                            calculate_dir_size: config.calculate_dir_size.unwrap_or(false),
                            sort_order: config.sort_order.unwrap_or(SortOrder::Name),
                            bookmarks: config.bookmarks.unwrap_or_default(),
                        };
                    }
                }
            }
        }
        Self::default()
    }
}

#[derive(serde::Deserialize, serde::Serialize)]
struct TomlConfig {
    theme: Option<String>,
    show_hidden: Option<bool>,
    preview_enabled: Option<bool>,
    calculate_dir_size: Option<bool>,
    sort_order: Option<SortOrder>,
    bookmarks: Option<Vec<PathBuf>>,
}

pub fn get_config_dir() -> Option<PathBuf> {
    directories::ProjectDirs::from("com", "tfe", "terminal-file-explorer")
        .map(|d| d.config_dir().to_path_buf())
}

pub fn ensure_config_dir() -> Option<PathBuf> {
    let dir = get_config_dir()?;
    if !dir.exists() {
        fs::create_dir_all(&dir).ok()?;
    }
    Some(dir)
}

pub fn save_config(config: &Config) -> Result<(), String> {
    let dir = ensure_config_dir().ok_or("Failed to get config directory")?;
    let config_path = dir.join("config.toml");
    let toml_config = TomlConfig {
        theme: Some(config.theme.clone()),
        show_hidden: Some(config.show_hidden),
        preview_enabled: Some(config.preview_enabled),
        calculate_dir_size: Some(config.calculate_dir_size),
        sort_order: Some(config.sort_order),
        bookmarks: Some(config.bookmarks.clone()),
    };
    let content = toml::to_string(&toml_config).map_err(|e| e.to_string())?;
    fs::write(config_path, content).map_err(|e| e.to_string())?;
    Ok(())
}
