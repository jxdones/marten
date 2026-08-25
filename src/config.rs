use std::{
    env, fs,
    io::{ErrorKind, Write},
    path::{Path, PathBuf},
};

use serde::Deserialize;

use crate::state::DiffLayout;
use crate::tui::theme::{self, Theme};

const CONFIG_DIR: &str = ".config";
const APP_DIR: &str = "marten";
const CONFIG_FILE: &str = "config.toml";
pub const DEFAULT_TAB_WIDTH: usize = 4;

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
pub struct Config {
    pub ui: UI,
    pub review: Review,
    pub diff: Diff,
}

#[derive(Debug, Deserialize)]
#[serde(default)]
pub struct UI {
    pub theme: String,
    pub show_sidebar: Option<bool>,
    pub transparent_background: bool,
    pub nerd_fonts: bool,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
pub struct Review {
    pub ignore: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[serde(default)]
pub struct Diff {
    pub ignore_whitespace: bool,
    pub tab_width: usize,
    pub layout: DiffLayoutSetting,
    pub show_line_numbers: bool,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DiffLayoutSetting {
    #[default]
    Auto,
    Split,
    Unified,
}

impl DiffLayoutSetting {
    pub const fn as_override(self) -> Option<DiffLayout> {
        match self {
            Self::Auto => None,
            Self::Split => Some(DiffLayout::SideBySide),
            Self::Unified => Some(DiffLayout::Unified),
        }
    }

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Auto => "auto",
            Self::Split => "split",
            Self::Unified => "unified",
        }
    }
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
    ParseDocument {
        path: PathBuf,
        source: toml_edit::TomlError,
    },
    Invalid {
        path: PathBuf,
        message: String,
    },
    CreateDirectory {
        path: PathBuf,
        source: std::io::Error,
    },
    Write {
        path: PathBuf,
        source: std::io::Error,
    },
}

impl UI {
    pub fn theme(&self) -> Theme {
        theme::entry_by_id(&self.theme)
            .unwrap_or_else(theme::default_entry)
            .theme
    }

    pub fn show_sidebar(&self, terminal_width: u16) -> bool {
        self.show_sidebar.unwrap_or(terminal_width > 120)
    }
}

impl Default for UI {
    fn default() -> Self {
        Self {
            theme: theme::default_entry().id.into(),
            show_sidebar: None,
            transparent_background: false,
            nerd_fonts: true,
        }
    }
}

impl Default for Diff {
    fn default() -> Self {
        Self {
            ignore_whitespace: false,
            tab_width: DEFAULT_TAB_WIDTH,
            layout: DiffLayoutSetting::Auto,
            show_line_numbers: true,
        }
    }
}

pub fn load() -> Result<Config, ConfigError> {
    let Some(path) = config_path() else {
        return Ok(Config::default());
    };

    let _ = ensure_default_file(&path);
    load_from(path)
}

fn ensure_default_file(path: &Path) -> Result<bool, ConfigError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|source| ConfigError::CreateDirectory {
            path: parent.to_path_buf(),
            source,
        })?;
    }

    let file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path);

    let mut file = match file {
        Ok(file) => file,
        Err(source) if source.kind() == ErrorKind::AlreadyExists => return Ok(false),
        Err(source) => {
            return Err(ConfigError::Write {
                path: path.to_path_buf(),
                source,
            });
        }
    };

    file.write_all(default_template().as_bytes())
        .map_err(|source| ConfigError::Write {
            path: path.to_path_buf(),
            source,
        })?;

    Ok(true)
}

fn default_template() -> String {
    let ui = UI::default();
    let diff = Diff::default();

    format!(
        r#"# marten configuration
#
# Every setting below is commented out and shown with its default value.
# Uncomment a line and change its value to override the default.
# This file is safe to delete -- marten regenerates it with defaults on the next run.

[ui]
# theme = "{theme}"
# show_sidebar = true          # default: shown automatically above 120 terminal columns
# transparent_background = {transparent_background}
# nerd_fonts = {nerd_fonts}    # default: true

[review]
# ignore = ["*.lock", "generated/**"]

[diff]
# ignore_whitespace = {ignore_whitespace}
# tab_width = {tab_width}
# layout = "{layout}"          # auto | split | unified
# show_line_numbers = {show_line_numbers}
"#,
        theme = ui.theme,
        transparent_background = ui.transparent_background,
        nerd_fonts = ui.nerd_fonts,
        ignore_whitespace = diff.ignore_whitespace,
        tab_width = diff.tab_width,
        layout = diff.layout.as_str(),
        show_line_numbers = diff.show_line_numbers,
    )
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

    if config.diff.tab_width == 0 {
        return Err(ConfigError::Invalid {
            path,
            message: "`diff.tab_width` must be greater than zero".into(),
        });
    }

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

    let mut document = contents
        .parse::<toml_edit::DocumentMut>()
        .map_err(|source| ConfigError::ParseDocument {
            path: path.clone(),
            source,
        })?;

    let ui = document["ui"]
        .or_insert(toml_edit::table())
        .as_table_like_mut()
        .ok_or_else(|| ConfigError::Invalid {
            path: path.clone(),
            message: "`ui` must be a table".into(),
        })?;
    ui.insert("theme", toml_edit::value(theme_id));

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|source| ConfigError::CreateDirectory {
            path: parent.to_path_buf(),
            source,
        })?;
    }
    fs::write(&path, document.to_string()).map_err(|source| ConfigError::Write { path, source })
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
            Self::ParseDocument { path, source } => {
                write!(formatter, "could not parse {}: {source}", path.display())
            }
            Self::Invalid { path, message } => {
                write!(formatter, "invalid config at {}: {message}", path.display())
            }
            Self::CreateDirectory { path, source } => {
                write!(formatter, "could not create {}: {source}", path.display())
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
            Self::ParseDocument { source, .. } => Some(source),
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
    fn transparent_background_defaults_to_false() {
        let config = Config::default();
        assert!(!config.ui.transparent_background);
    }

    #[test]
    fn set_transparent_background_to_true() {
        let config: Config = toml::from_str("[ui]\ntransparent_background = true").unwrap();
        assert!(config.ui.transparent_background);
    }

    #[test]
    fn nerd_fonts_defaults_to_true_and_can_be_disabled() {
        let config = Config::default();
        assert!(config.ui.nerd_fonts);

        let config: Config = toml::from_str("[ui]\nnerd_fonts = false").unwrap();
        assert!(!config.ui.nerd_fonts);
    }

    #[test]
    fn diff_ignore_whitespace_defaults_to_false() {
        let config = Config::default();
        assert!(!config.diff.ignore_whitespace);
    }

    #[test]
    fn set_diff_ignore_whitespace_to_true() {
        let config: Config = toml::from_str("[diff]\n ignore_whitespace = true").unwrap();
        assert!(config.diff.ignore_whitespace);
    }

    #[test]
    fn diff_tab_width_defaults_to_four() {
        let config = Config::default();
        assert!(config.diff.tab_width == DEFAULT_TAB_WIDTH)
    }

    #[test]
    fn set_tab_width_to_two() {
        let config: Config = toml::from_str("[diff]\n tab_width = 2").unwrap();
        assert!(config.diff.tab_width == 2);
    }

    #[test]
    fn diff_layout_defaults_to_auto() {
        let config = Config::default();
        assert_eq!(config.diff.layout, DiffLayoutSetting::Auto);
        assert_eq!(config.diff.layout.as_override(), None);
    }

    #[test]
    fn set_diff_layout_to_split() {
        let config: Config = toml::from_str("[diff]\n layout = 'split'").unwrap();
        assert_eq!(config.diff.layout, DiffLayoutSetting::Split);
        assert_eq!(
            config.diff.layout.as_override(),
            Some(DiffLayout::SideBySide)
        );
    }

    #[test]
    fn set_diff_layout_to_unified() {
        let config: Config = toml::from_str("[diff]\n layout = 'unified'").unwrap();
        assert_eq!(config.diff.layout, DiffLayoutSetting::Unified);
        assert_eq!(config.diff.layout.as_override(), Some(DiffLayout::Unified));
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
    fn saving_theme_preserves_comments_in_the_default_template() {
        let directory = tempdir().unwrap();
        let path = directory.path().join("config.toml");
        fs::write(&path, default_template()).unwrap();

        save_theme_to(path.clone(), "ermine").unwrap();

        let saved = fs::read_to_string(&path).unwrap();
        assert!(saved.contains("# Every setting below is commented out"));
        assert!(saved.contains("# tab_width = 4"));

        let config = load_from(path).unwrap();
        assert_eq!(config.ui.theme, "ermine");
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
    fn first_run_creates_a_commented_default_file() {
        let directory = tempdir().unwrap();
        let path = directory.path().join("config.toml");

        let created = ensure_default_file(&path).unwrap();
        assert!(created);
        assert!(path.exists());

        let contents = fs::read_to_string(&path).unwrap();
        assert!(contents.contains("# theme ="));
        assert!(contents.contains("# tab_width = 4"));
    }

    #[test]
    fn default_template_parses_to_defaults() {
        let config: Config = toml::from_str(&default_template()).unwrap();

        assert_eq!(config.ui.theme, Config::default().ui.theme);
        assert_eq!(config.diff.tab_width, DEFAULT_TAB_WIDTH);
        assert_eq!(config.diff.layout, DiffLayoutSetting::Auto);
    }

    #[test]
    fn ensure_default_file_does_not_overwrite_an_existing_file() {
        let directory = tempdir().unwrap();
        let path = directory.path().join("config.toml");
        fs::write(&path, "[ui]\ntheme = 'ermine'\n").unwrap();

        let created = ensure_default_file(&path).unwrap();
        assert!(!created);

        let config = load_from(path).unwrap();
        assert_eq!(config.ui.theme, "ermine");
    }

    #[test]
    fn ensure_default_file_creates_missing_parent_directories() {
        let directory = tempdir().unwrap();
        let path = directory.path().join("nested").join("config.toml");

        ensure_default_file(&path).unwrap();

        assert!(path.exists());
    }

    #[test]
    fn theme_ids_are_unique() {
        let mut ids = std::collections::HashSet::new();

        for entry in theme::THEMES {
            assert!(ids.insert(entry.id), "duplicate theme id: {}", entry.id);
        }
    }

    #[test]
    fn review_ignore_patterns_load_from_config() {
        let config: Config =
            toml::from_str("[review]\nignore = [\"*.lock\", \"generated/**\"]\n").unwrap();

        assert_eq!(config.review.ignore, vec!["*.lock", "generated/**"]);
        assert!(crate::glob::matches_any(
            &config.review.ignore,
            "Cargo.lock"
        ));
        assert!(crate::glob::matches_any(
            &config.review.ignore,
            "generated/out.rs"
        ));
        assert!(crate::glob::matches_any(
            &config.review.ignore,
            "generated/sub/out.rs"
        ));
        assert!(!crate::glob::matches_any(
            &config.review.ignore,
            "src/generated/out.rs"
        ));
        assert!(!crate::glob::matches_any(
            &config.review.ignore,
            "src/main.rs"
        ));
    }

    #[test]
    fn missing_review_section_uses_empty_defaults() {
        let config: Config = toml::from_str("[ui]\ntheme = 'ermine'\n").unwrap();

        assert!(config.review.ignore.is_empty());
        assert!(!crate::glob::matches_any(
            &config.review.ignore,
            "Cargo.lock"
        ));
    }

    #[test]
    fn empty_review_section_ignores_nothing() {
        let config: Config = toml::from_str("[review]\n").unwrap();

        assert!(config.review.ignore.is_empty());
        assert!(!crate::glob::matches_any(
            &config.review.ignore,
            "anything/at/all.rs"
        ));
    }

    #[test]
    fn saving_theme_preserves_review_settings() {
        let directory = tempdir().unwrap();
        let path = directory.path().join("config.toml");
        fs::write(
            &path,
            "[review]\nignore = [\"*.lock\"]\n\n[ui]\ntheme = 'marten'\n",
        )
        .unwrap();

        save_theme_to(path.clone(), "ermine").unwrap();

        let config = load_from(path).unwrap();
        assert_eq!(config.ui.theme, "ermine");
        assert_eq!(config.review.ignore, vec!["*.lock"]);
    }
}
