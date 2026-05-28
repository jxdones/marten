use std::sync::OnceLock;

use ratatui::{
    style::{Color, Style},
    text::Span,
};
use syntect::{
    easy::HighlightLines,
    highlighting::{Style as SyntectStyle, Theme, ThemeSet},
    parsing::SyntaxSet,
    util::LinesWithEndings,
};

static SYNTAX_SET: OnceLock<SyntaxSet> = OnceLock::new();
static THEME: OnceLock<Theme> = OnceLock::new();

pub fn highlight_line(path: &str, content: &str, base_style: Style) -> Option<Vec<Span<'static>>> {
    let syntax_set = SYNTAX_SET.get_or_init(SyntaxSet::load_defaults_newlines);
    let syntax = syntax_set
        .find_syntax_for_file(path)
        .ok()
        .flatten()
        .unwrap_or_else(|| syntax_set.find_syntax_plain_text());

    let theme = THEME.get_or_init(|| {
        ThemeSet::load_defaults()
            .themes
            .remove("base16-ocean.dark")
            .unwrap_or_default()
    });

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

fn span_style(base_style: Style, style: SyntectStyle) -> Style {
    base_style.fg(Color::Rgb(
        style.foreground.r,
        style.foreground.g,
        style.foreground.b,
    ))
}
