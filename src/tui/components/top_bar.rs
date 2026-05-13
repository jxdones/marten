use crate::{app::App, state::Focus};
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
};

pub fn draw(frame: &mut Frame, area: Rect, app: &App) {
    let layout = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(1),
            Constraint::Percentage(45),
            Constraint::Percentage(55),
            Constraint::Length(1),
        ])
        .split(area);

    let left = layout[1];
    let right = layout[2];

    let current_branch = "main";
    let left_line = Line::from(vec![
        Span::styled(
            "marten",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(" ", Style::default()),
        Span::styled(
            current_branch,
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled("  ↑", Style::default().fg(Color::Gray)),
        Span::styled("3", Style::default().fg(Color::Green)),
        Span::styled(" ↓", Style::default().fg(Color::Gray)),
        Span::styled("1", Style::default().fg(Color::Red)),
    ]);

    let mode = top_bar_mode(app);
    let right_line = Line::from(Span::styled(
        mode,
        Style::default().fg(Color::Rgb(230, 180, 80)),
    ));

    frame.render_widget(Paragraph::new(""), area);
    frame.render_widget(Paragraph::new(left_line), left);
    frame.render_widget(
        Paragraph::new(right_line).alignment(Alignment::Right),
        right,
    );
}

fn top_bar_mode(app: &App) -> &'static str {
    match app.focus() {
        Focus::Files => "files",
        Focus::Diff => "diff",
        Focus::History => "history",
        Focus::Details => "details",
    }
}
