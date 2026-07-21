use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::Style,
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
};

use crate::app::App;
use crate::state::Focus;

pub fn draw(frame: &mut Frame, area: Rect, app: &App) {
    let theme = app.theme();
    let bg_style = Style::default().bg(theme.bg);
    let block = Block::default()
        .borders(Borders::TOP)
        .border_style(theme.panel_border())
        .style(bg_style);
    let inner = block.inner(area);
    frame.render_widget(block, area);

    if inner.is_empty() {
        return;
    }

    let left = shortcut_spans(shortcuts(app), app);
    let right = quit_spans(app);
    let right_width = u16::try_from(spans_width(&right)).expect("terminal width exceeded u16::MAX");
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Min(0),
            Constraint::Length(right_width.min(inner.width)),
        ])
        .split(inner);

    frame.render_widget(Paragraph::new(Line::from(left)).style(bg_style), chunks[0]);
    frame.render_widget(Paragraph::new(Line::from(right)).style(bg_style), chunks[1]);
}

fn shortcuts(app: &App) -> Vec<(&'static str, &'static str)> {
    let mut shortcuts = vec![("tab", "focus")];

    match app.focus() {
        Focus::Files => {
            shortcuts.push(("j/k", "navigate"));
        }
        Focus::Diff => {
            shortcuts.extend([("h/j/k/l", "scroll"), ("[/]", "hunk")]);
        }
    }

    shortcuts.push(("?", "commands"));

    shortcuts
}

fn shortcut_spans(shortcuts: Vec<(&'static str, &'static str)>, app: &App) -> Vec<Span<'static>> {
    let theme = app.theme();
    let mut spans = Vec::new();

    for (idx, (key, label)) in shortcuts.into_iter().enumerate() {
        if idx > 0 {
            spans.push(Span::raw(" "));
        }
        spans.push(Span::styled(format!(" {key} "), theme.accent()));
        spans.push(Span::styled(format!("{label} "), theme.muted()));
    }

    spans
}

fn quit_spans(app: &App) -> Vec<Span<'static>> {
    let theme = app.theme();
    vec![
        Span::styled(" q ", theme.danger()),
        Span::styled("quit ", theme.muted()),
    ]
}

fn spans_width(spans: &[Span]) -> usize {
    spans.iter().map(|span| span.content.chars().count()).sum()
}
