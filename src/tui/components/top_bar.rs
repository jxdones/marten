use crate::{app::App, git::repository::Head, state::Focus};
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

    let left_line = app.repository_status().map_or_else(
        || Line::from(vec![Span::styled("no repository", theme.repo_name())]),
        |status| {
            let (branch_label, branch_style) = match &status.head {
                Head::Branch(name) => (name.clone(), theme.branch_name()),
                Head::Detached(commit) => (format!("{commit} (detached)"), theme.danger()),
                Head::Unknown => ("unknown".to_string(), theme.muted()),
            };

            let mut spans = vec![
                Span::styled(status.name.as_str(), theme.repo_name()),
                Span::styled("  ·  ", Style::default()),
                Span::styled(branch_label, branch_style),
            ];

            if status.changes.staged > 0 {
                spans.push(Span::styled(
                    format!(" +{}", status.changes.staged),
                    theme.staged(),
                ));
            }

            if status.changes.unstaged > 0 {
                spans.push(Span::styled(
                    format!(" ~{}", status.changes.unstaged),
                    theme.unstaged(),
                ));
            }

            if status.changes.untracked > 0 {
                spans.push(Span::styled(
                    format!(" ?{}", status.changes.untracked),
                    theme.untracked(),
                ));
            }

            if status.changes.conflicted > 0 {
                spans.push(Span::styled(
                    format!(" !{}", status.changes.conflicted),
                    theme.conflict(),
                ));
            }

            let ahead_style = if status.ahead > 0 {
                theme.success()
            } else {
                theme.muted()
            };

            let behind_style = if status.behind > 0 {
                theme.danger()
            } else {
                theme.muted()
            };

            spans.extend([
                Span::styled("  ·  ", Style::default()),
                Span::styled(format!("↑{}", status.ahead), ahead_style),
                Span::styled(format!(" ↓{}", status.behind), behind_style),
            ]);

            Line::from(spans)
        },
    );

    let mode = top_bar_mode(app);
    let right_line = Line::from(Span::styled(mode, theme.text_primary()));

    frame.render_widget(Paragraph::new(""), area);
    frame.render_widget(Paragraph::new(left_line), left);
    frame.render_widget(
        Paragraph::new(right_line).alignment(Alignment::Right),
        right,
    );
}

const fn top_bar_mode(app: &App) -> &'static str {
    match app.focus() {
        Focus::Files => "files",
        Focus::Branches => "branches",
        Focus::Stash => "stash",
        Focus::Diff => "diff",
        Focus::History => "history",
        Focus::Details => "details",
    }
}
