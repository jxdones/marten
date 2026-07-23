use std::{str::FromStr, sync::OnceLock};

use ratatui::{
    style::{Color, Style},
    text::Span,
};
use syntect::{
    easy::HighlightLines,
    highlighting::{
        Color as SyntectColor, ScopeSelectors, Style as SyntectStyle, StyleModifier,
        Theme as SyntectTheme, ThemeItem, ThemeSet, ThemeSettings,
    },
    parsing::SyntaxSet,
    util::LinesWithEndings,
};

use crate::tui::theme::{SyntaxPalette, THEMES as APP_THEMES};

static SYNTAX_SET: OnceLock<SyntaxSet> = OnceLock::new();
static THEMES: OnceLock<ThemeSet> = OnceLock::new();

pub fn highlight_line(
    path: &str,
    content: &str,
    base_style: Style,
    theme_name: &str,
) -> Option<Vec<Span<'static>>> {
    let syntax_set = SYNTAX_SET.get_or_init(SyntaxSet::load_defaults_newlines);
    let syntax = syntax_set
        .find_syntax_for_file(path)
        .ok()
        .flatten()
        .unwrap_or_else(|| syntax_set.find_syntax_plain_text());

    let themes = THEMES.get_or_init(load_themes);
    let theme = themes.themes.get(theme_name)?;

    let mut highlighter = HighlightLines::new(syntax, theme);
    let mut spans = Vec::new();

    for line in LinesWithEndings::from(content) {
        let highlighted = highlighter.highlight_line(line, syntax_set).ok()?;
        spans.extend(
            highlighted
                .into_iter()
                .map(|(style, text)| Span::styled(text.to_string(), span_style(base_style, style))),
        );
    }

    Some(spans)
}

fn load_themes() -> ThemeSet {
    let mut themes = ThemeSet::load_defaults();

    for entry in APP_THEMES {
        if let Some(palette) = entry.theme.syntax_palette {
            themes.themes.insert(
                entry.theme.syntax_theme.to_string(),
                custom_theme(entry.name, entry.theme.bg, entry.theme.fg, palette),
            );
        }
    }

    themes
}

fn custom_theme(name: &str, bg: Color, fg: Color, palette: SyntaxPalette) -> SyntectTheme {
    SyntectTheme {
        name: Some(name.to_string()),
        settings: ThemeSettings {
            background: Some(syntect_color(bg)),
            foreground: Some(syntect_color(fg)),
            ..ThemeSettings::default()
        },
        scopes: vec![
            theme_item("comment", palette.comment),
            theme_item("keyword, storage", palette.keyword),
            theme_item("entity.name.function, support.function", palette.function),
            theme_item("variable", palette.variable),
            theme_item("string", palette.string),
            theme_item("constant.numeric, constant.language", palette.number),
            theme_item(
                "entity.name.type, entity.name.class, support.type, storage.type",
                palette.type_name,
            ),
            theme_item("keyword.operator", palette.operator),
            theme_item("punctuation", palette.punctuation),
        ],
        ..SyntectTheme::default()
    }
}

fn theme_item(scope: &str, color: Color) -> ThemeItem {
    ThemeItem {
        scope: ScopeSelectors::from_str(scope).expect("static scope selector should be valid"),
        style: StyleModifier {
            foreground: Some(syntect_color(color)),
            ..StyleModifier::default()
        },
    }
}

fn syntect_color(color: Color) -> SyntectColor {
    let Color::Rgb(r, g, b) = color else {
        unreachable!("theme colors used for syntax highlighting must be RGB")
    };

    SyntectColor { r, g, b, a: 255 }
}

fn span_style(base_style: Style, style: SyntectStyle) -> Style {
    base_style.fg(Color::Rgb(
        style.foreground.r,
        style.foreground.g,
        style.foreground.b,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_configured_syntax_theme_is_available() {
        let themes = load_themes();

        for entry in APP_THEMES {
            assert!(
                themes.themes.contains_key(entry.theme.syntax_theme),
                "missing syntax theme for {}",
                entry.id
            );
        }
    }
}
