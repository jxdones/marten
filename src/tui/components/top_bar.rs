use crate::app::App;
use crate::git::repository::Head;
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::Style,
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
};

pub fn draw(frame: &mut Frame, area: Rect, app: &App) {
    let theme = app.theme();
    let block = Block::default()
        .borders(Borders::BOTTOM)
        .border_style(theme.panel_border())
        .style(Style::default().bg(theme.bg));
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let layout = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(1),
            Constraint::Percentage(45),
            Constraint::Percentage(55),
            Constraint::Length(1),
        ])
        .split(inner);

    let left = layout[1];
    let right = layout[2];

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

    let right_line = diff_summary(app);

    let bg_style = Style::default().bg(theme.bg);

    frame.render_widget(Paragraph::new(left_line).style(bg_style), left);
    frame.render_widget(
        Paragraph::new(right_line)
            .style(bg_style)
            .alignment(Alignment::Right),
        right,
    );
}

fn diff_summary(app: &App) -> Line<'static> {
    let theme = app.theme();
    let files = app.files();

    let insertions = files
        .iter()
        .map(|slot| slot.entry.insertions)
        .sum::<usize>();
    let deletions = files.iter().map(|slot| slot.entry.deletions).sum::<usize>();
    let file_label = if files.len() == 1 { "file" } else { "files" };

    Line::from(vec![
        Span::styled(format!("+{insertions}"), theme.success()),
        Span::styled(" ", theme.muted()),
        Span::styled(format!("-{deletions}"), theme.unstaged()),
        Span::styled("  ·  ", theme.muted()),
        Span::styled(files.len().to_string(), theme.text_primary()),
        Span::styled(format!(" {file_label}"), theme.muted()),
    ])
}
