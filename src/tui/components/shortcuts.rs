use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
};

use crate::app::App;
use crate::state::Focus;

pub fn draw(frame: &mut Frame, area: Rect, app: &App) {
    let theme = app.theme();
    let block = Block::default()
        .borders(Borders::TOP)
        .border_style(theme.panel_border());
    let inner = block.inner(area);
    frame.render_widget(block, area);

    if inner.is_empty() {
        return;
    }

    let left = shortcut_spans(shortcuts(app), app);
    let right = quit_spans(app);
    let right_width = spans_width(&right) as u16;
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Min(0),
            Constraint::Length(right_width.min(inner.width)),
        ])
        .split(inner);

    frame.render_widget(Paragraph::new(Line::from(left)), chunks[0]);
    frame.render_widget(Paragraph::new(Line::from(right)), chunks[1]);
}

fn shortcuts(app: &App) -> Vec<(&'static str, &'static str)> {
    let mut shortcuts = vec![("tab", "next"), ("shift+tab", "previous"), ("r", "reload")];

    match app.focus() {
        Focus::Files => {
            shortcuts.extend([("j/k", "navigate")]);
        }
        Focus::Diff => {
            let line_number_label = if app.diff_state().show_line_numbers {
                "hide lines"
            } else {
                "show lines"
            };
            shortcuts.extend([("j/k", "scroll"), ("[/]", "hunk"), ("l", line_number_label)]);
        }
        Focus::History | Focus::Details => {
            shortcuts.extend([("j/k", "navigate")]);
        }
    }

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
