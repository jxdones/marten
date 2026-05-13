use crate::{app::App, state::Focus};
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::Style,
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
    let theme = app.theme();

    let current_branch = "main";
    let left_line = Line::from(vec![
        Span::styled("marten", theme.repo_name()),
        Span::styled("  ·  ", Style::default()),
        Span::styled(current_branch, theme.branch_name()),
        Span::styled("  ·  ", Style::default()),
        Span::styled("↑", theme.success()),
        Span::styled("3", theme.success()),
        Span::styled(" ↓", theme.danger()),
        Span::styled("1", theme.danger()),
    ]);

    let mode = top_bar_mode(app);
    let right_line = Line::from(Span::styled(mode, theme.text_primary()));

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
