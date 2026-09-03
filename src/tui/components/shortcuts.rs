use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::Style,
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
};

use crate::app::App;
use crate::state::Focus;

const GAP: &str = "   ";

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

    let right = right_spans(app);
    let right_width = u16::try_from(spans_width(&right)).expect("terminal width exceeded u16::MAX");
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Min(0),
            Constraint::Length(right_width.min(inner.width)),
        ])
        .split(inner);

    let left = shortcut_spans(&shortcuts(app), chunks[0].width as usize, app);

    frame.render_widget(Paragraph::new(Line::from(left)).style(bg_style), chunks[0]);
    frame.render_widget(Paragraph::new(Line::from(right)).style(bg_style), chunks[1]);
}

fn shortcuts(app: &App) -> Vec<(&'static str, &'static str)> {
    match app.focus() {
        Focus::Files => vec![
            ("j/k", "navigate"),
            ("z", "collapse"),
            ("m", "review"),
        ],
        Focus::Diff => vec![
            ("[/]", "hunk"),
            ("z", "collapse"),
            ("m", "review"),
            ("e", "edit"),
        ],
    }
}

fn shortcut_spans(
    shortcuts: &[(&'static str, &'static str)],
    max_width: usize,
    app: &App,
) -> Vec<Span<'static>> {
    let theme = app.theme();
    let mut spans = Vec::new();
    let mut current_width = 0;

    for (idx, &(key, label)) in shortcuts.iter().enumerate() {
        let needed = if idx == 0 {
            1 + item_width(key, label)
        } else {
            GAP.len() + item_width(key, label)
        };

        if current_width + needed > max_width {
            break;
        }

        if idx == 0 {
            spans.push(Span::raw(" "));
            current_width += 1;
        } else {
            spans.push(Span::raw(GAP));
            current_width += GAP.len();
        }

        for ch in key.chars() {
            let style = if ch == '/' {
                theme.muted()
            } else {
                theme.accent()
            };
            spans.push(Span::styled(ch.to_string(), style));
        }
        spans.push(Span::raw(" "));
        spans.push(Span::styled(label, theme.muted()));
        current_width += item_width(key, label);
    }

    spans
}

fn item_width(key: &str, label: &str) -> usize {
    key.chars().count() + 1 + label.chars().count()
}


fn right_spans(app: &App) -> Vec<Span<'static>> {
    let theme = app.theme();
    vec![
        Span::styled("?", theme.accent()),
        Span::raw(" "),
        Span::styled("commands", theme.muted()),
        Span::raw("   "),
        Span::styled("q", theme.danger()),
        Span::raw(" "),
        Span::styled("quit", theme.muted()),
        Span::raw(" "),
    ]
}

fn spans_width(spans: &[Span]) -> usize {
    spans.iter().map(|span| span.content.chars().count()).sum()
}
