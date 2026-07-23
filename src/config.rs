use std::{env, fs, io::ErrorKind, path::PathBuf};

use serde::Deserialize;

use crate::tui::theme::{self, Theme};

const CONFIG_DIR: &str = ".config";
const APP_DIR: &str = "marten";
const CONFIG_FILE: &str = "config.toml";

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
pub struct Config {
    pub ui: UiConfig,
}

#[derive(Debug, Deserialize)]
#[serde(default)]
pub struct UiConfig {
    pub theme: String,
    pub show_sidebar: Option<bool>,
}

#[derive(Debug)]
pub enum ConfigError {
    HomeDirectoryUnavailable,
    Read {
        path: PathBuf,
        source: std::io::Error,
    },
    Parse {
        path: PathBuf,
        source: toml::de::Error,
    },
    Invalid {
        path: PathBuf,
        message: String,
    },
    CreateDirectory {
        path: PathBuf,
        source: std::io::Error,
    },
    Serialize {
        path: PathBuf,
        source: toml::ser::Error,
    },
    Write {
        path: PathBuf,
        source: std::io::Error,
    },
}

impl UiConfig {
    pub fn theme(&self) -> Theme {
        theme::entry_by_id(&self.theme)
            .unwrap_or_else(theme::default_entry)
            .theme
    }

    pub fn show_sidebar(&self, terminal_width: u16) -> bool {
        self.show_sidebar.unwrap_or(terminal_width > 120)
    }
}

impl Default for UiConfig {
    fn default() -> Self {
        Self {
            theme: theme::default_entry().id.into(),
            show_sidebar: None,
        }
    }
}

pub fn load() -> Result<Config, ConfigError> {
    let Some(path) = config_path() else {
        return Ok(Config::default());
    };

    load_from(path)
}

pub fn save_theme(entry: &theme::ThemeEntry) -> Result<(), ConfigError> {
    let Some(path) = config_path() else {
        return Err(ConfigError::HomeDirectoryUnavailable);
    };

    save_theme_to(path, entry.id)
}

fn config_path() -> Option<PathBuf> {
    env::var_os("HOME").map(|home| {
        PathBuf::from(home)
            .join(CONFIG_DIR)
            .join(APP_DIR)
            .join(CONFIG_FILE)
    })
}

fn load_from(path: PathBuf) -> Result<Config, ConfigError> {
    let contents = match fs::read_to_string(&path) {
        Ok(contents) => contents,
        Err(source) if source.kind() == ErrorKind::NotFound => return Ok(Config::default()),
        Err(source) => return Err(ConfigError::Read { path, source }),
    };

    let config: Config = toml::from_str(&contents).map_err(|source| ConfigError::Parse {
        path: path.clone(),
        source,
    })?;

    if theme::entry_by_id(&config.ui.theme).is_none() {
        let expected = theme::THEMES
            .iter()
            .map(|entry| format!("`{}`", entry.id))
            .collect::<Vec<_>>()
            .join(", ");
        return Err(ConfigError::Invalid {
            path,
            message: format!(
                "unknown theme `{}` (expected one of: {expected})",
                config.ui.theme
            ),
        });
    }

    Ok(config)
}

fn save_theme_to(path: PathBuf, theme_id: &str) -> Result<(), ConfigError> {
    let contents = match fs::read_to_string(&path) {
        Ok(contents) => contents,
        Err(source) if source.kind() == ErrorKind::NotFound => String::new(),
        Err(source) => return Err(ConfigError::Read { path, source }),
    };
    let mut document =
        toml::from_str::<toml::Table>(&contents).map_err(|source| ConfigError::Parse {
            path: path.clone(),
            source,
        })?;
    let ui = document
        .entry("ui")
        .or_insert_with(|| toml::Value::Table(toml::Table::new()))
        .as_table_mut()
        .ok_or_else(|| ConfigError::Invalid {
            path: path.clone(),
            message: "`ui` must be a table".into(),
        })?;
    ui.insert("theme".into(), toml::Value::String(theme_id.into()));

    let serialized =
        toml::to_string_pretty(&document).map_err(|source| ConfigError::Serialize {
            path: path.clone(),
            source,
        })?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|source| ConfigError::CreateDirectory {
            path: parent.to_path_buf(),
            source,
        })?;
    }
    fs::write(&path, serialized).map_err(|source| ConfigError::Write { path, source })
}

impl std::fmt::Display for ConfigError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::HomeDirectoryUnavailable => {
                write!(
                    formatter,
                    "could not locate the config file because HOME is not set"
                )
            }
            Self::Read { path, source } => {
                write!(formatter, "could not read {}: {source}", path.display())
            }
            Self::Parse { path, source } => {
                write!(formatter, "could not parse {}: {source}", path.display())
            }
            Self::Invalid { path, message } => {
                write!(formatter, "invalid config at {}: {message}", path.display())
            }
            Self::CreateDirectory { path, source } => {
                write!(formatter, "could not create {}: {source}", path.display())
            }
            Self::Serialize { path, source } => {
                write!(
                    formatter,
                    "could not serialize {}: {source}",
                    path.display()
                )
            }
            Self::Write { path, source } => {
                write!(formatter, "could not write {}: {source}", path.display())
            }
        }
    }
}

impl std::error::Error for ConfigError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::HomeDirectoryUnavailable => None,
            Self::Read { source, .. }
            | Self::CreateDirectory { source, .. }
            | Self::Write { source, .. } => Some(source),
            Self::Parse { source, .. } => Some(source),
            Self::Serialize { source, .. } => Some(source),
            Self::Invalid { .. } => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn missing_config_uses_defaults() {
        let directory = tempdir().unwrap();
        let config = load_from(directory.path().join("config.toml")).unwrap();

        assert_eq!(config.ui.theme, "marten");
        assert_eq!(config.ui.show_sidebar, None);
    }

    #[test]
    fn empty_config_uses_defaults() {
        let config: Config = toml::from_str("").unwrap();

        assert_eq!(config.ui.theme, "marten");
        assert_eq!(config.ui.show_sidebar, None);
    }

    #[test]
    fn empty_theme_is_invalid() {
        let directory = tempdir().unwrap();
        let path = directory.path().join("config.toml");
        fs::write(&path, "[ui]\ntheme = ''").unwrap();

        let error = load_from(path).unwrap_err();

        assert!(error.to_string().contains("unknown theme ``"));
    }

    #[test]
    fn sidebar_setting_overrides_terminal_width() {
        let shown: Config = toml::from_str("[ui]\nshow_sidebar = true").unwrap();
        let hidden: Config = toml::from_str("[ui]\nshow_sidebar = false").unwrap();

        assert!(shown.ui.show_sidebar(80));
        assert!(!hidden.ui.show_sidebar(160));
    }

    #[test]
    fn sidebar_defaults_to_terminal_width() {
        let config = Config::default();

        assert!(!config.ui.show_sidebar(120));
        assert!(config.ui.show_sidebar(121));
    }

    #[test]
    fn unknown_theme_has_an_actionable_error() {
        let directory = tempdir().unwrap();
        let path = directory.path().join("config.toml");
        fs::write(&path, "[ui]\ntheme = 'unknown'").unwrap();

        let error = load_from(path).unwrap_err();
        let message = error.to_string();

        assert!(message.contains("unknown theme `unknown`"));
        assert!(message.contains("`marten`"));
        assert!(message.contains("`ermine`"));
    }

    #[test]
    fn malformed_file_error_includes_its_path() {
        let directory = tempdir().unwrap();
        let path = directory.path().join("config.toml");
        fs::write(&path, "[ui\n").unwrap();

        let error = load_from(path.clone()).unwrap_err();

        assert!(error.to_string().contains(&path.display().to_string()));
    }

    #[test]
    fn saving_theme_creates_config_and_can_be_loaded() {
        let directory = tempdir().unwrap();
        let path = directory.path().join("nested").join("config.toml");

        save_theme_to(path.clone(), "ermine").unwrap();

        let config = load_from(path).unwrap();
        assert_eq!(config.ui.theme, "ermine");
    }

    #[test]
    fn saving_theme_preserves_other_settings() {
        let directory = tempdir().unwrap();
        let path = directory.path().join("config.toml");
        fs::write(
            &path,
            "custom = 'kept'\n\n[ui]\nshow_sidebar = true\ntheme = 'marten'\n",
        )
        .unwrap();

        save_theme_to(path.clone(), "ermine").unwrap();

        let saved = fs::read_to_string(path).unwrap();
        let document: toml::Table = toml::from_str(&saved).unwrap();
        assert_eq!(document["custom"].as_str(), Some("kept"));
        assert_eq!(document["ui"]["show_sidebar"].as_bool(), Some(true));
        assert_eq!(document["ui"]["theme"].as_str(), Some("ermine"));
    }

    #[test]
    fn theme_ids_are_unique() {
        let mut ids = std::collections::HashSet::new();

        for entry in theme::THEMES {
            assert!(ids.insert(entry.id), "duplicate theme id: {}", entry.id);
        }
    }
}
