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

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
pub struct UiConfig {
    pub theme: BuiltInTheme,
    pub show_sidebar: Option<bool>,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum BuiltInTheme {
    #[default]
    Marten,
}

#[derive(Debug)]
pub enum ConfigError {
    Read {
        path: PathBuf,
        source: std::io::Error,
    },
    Parse {
        path: PathBuf,
        source: toml::de::Error,
    },
}

impl UiConfig {
    pub const fn theme(&self) -> Theme {
        self.theme.resolve()
    }

    pub fn show_sidebar(&self, terminal_width: u16) -> bool {
        self.show_sidebar.unwrap_or(terminal_width > 120)
    }
}

impl BuiltInTheme {
    const fn resolve(self) -> Theme {
        match self {
            Self::Marten => theme::DEFAULT,
        }
    }
}

pub fn load() -> Result<Config, ConfigError> {
    let Some(home) = env::var_os("HOME") else {
        return Ok(Config::default());
    };
    let path = PathBuf::from(home)
        .join(CONFIG_DIR)
        .join(APP_DIR)
        .join(CONFIG_FILE);

    load_from(path)
}

fn load_from(path: PathBuf) -> Result<Config, ConfigError> {
    let contents = match fs::read_to_string(&path) {
        Ok(contents) => contents,
        Err(source) if source.kind() == ErrorKind::NotFound => return Ok(Config::default()),
        Err(source) => return Err(ConfigError::Read { path, source }),
    };

    toml::from_str(&contents).map_err(|source| ConfigError::Parse { path, source })
}

impl std::fmt::Display for ConfigError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Read { path, source } => {
                write!(formatter, "could not read {}: {source}", path.display())
            }
            Self::Parse { path, source } => {
                write!(formatter, "could not parse {}: {source}", path.display())
            }
        }
    }
}

impl std::error::Error for ConfigError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Read { source, .. } => Some(source),
            Self::Parse { source, .. } => Some(source),
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

        assert_eq!(config.ui.theme, BuiltInTheme::Marten);
        assert_eq!(config.ui.show_sidebar, None);
    }

    #[test]
    fn empty_config_uses_defaults() {
        let config: Config = toml::from_str("").unwrap();

        assert_eq!(config.ui.theme, BuiltInTheme::Marten);
        assert_eq!(config.ui.show_sidebar, None);
    }

    #[test]
    fn empty_theme_is_invalid() {
        let error = toml::from_str::<Config>("[ui]\ntheme = ''").unwrap_err();

        assert!(error.to_string().contains("unknown variant"));
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
        let error = toml::from_str::<Config>("[ui]\ntheme = 'unknown'").unwrap_err();
        let message = error.to_string();

        assert!(message.contains("unknown variant `unknown`"));
        assert!(message.contains("expected `marten`"));
    }

    #[test]
    fn malformed_file_error_includes_its_path() {
        let directory = tempdir().unwrap();
        let path = directory.path().join("config.toml");
        fs::write(&path, "[ui\n").unwrap();

        let error = load_from(path.clone()).unwrap_err();

        assert!(error.to_string().contains(&path.display().to_string()));
    }
}
