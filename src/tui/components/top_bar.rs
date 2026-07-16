use crate::app::App;
use crate::git::repository::{DiffSource, Head, RepositoryStatus, RevisionData};
use crate::tui::theme::Theme;
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Layout, Rect},
    style::Style,
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
};

const HORIZONTAL_PADDING: u16 = 1;
const SUMMARY_GAP: u16 = 2;

pub fn draw(frame: &mut Frame, area: Rect, app: &App) {
    let theme = app.theme();
    let block = Block::default()
        .borders(Borders::BOTTOM)
        .border_style(theme.panel_border())
        .style(Style::default().bg(theme.bg));
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let right_line = diff_summary(app);
    let right_width = u16::try_from(right_line.width()).unwrap_or(u16::MAX);
    let (left, right) = content_areas(inner, right_width);

    let left_line = match app.diff_source() {
        DiffSource::Worktree => worktree_summary(app.repository_status(), theme),
        DiffSource::Revision(revision) => revision_summary(
            app.repository_status(),
            revision,
            theme,
            usize::from(left.width),
        ),
    };

    let bg_style = Style::default().bg(theme.bg);

    frame.render_widget(Paragraph::new(left_line).style(bg_style), left);
    frame.render_widget(
        Paragraph::new(right_line)
            .style(bg_style)
            .alignment(Alignment::Right),
        right,
    );
}

fn content_areas(area: Rect, right_width: u16) -> (Rect, Rect) {
    let layout = Layout::horizontal([
        Constraint::Length(HORIZONTAL_PADDING),
        Constraint::Fill(1),
        Constraint::Length(SUMMARY_GAP),
        Constraint::Length(right_width),
        Constraint::Length(HORIZONTAL_PADDING),
    ])
    .split(area);

    (layout[1], layout[3])
}

fn worktree_summary(status: Option<&RepositoryStatus>, theme: Theme) -> Line<'static> {
    let Some(status) = status else {
        return Line::from(Span::styled("no repository", theme.repo_name()));
    };

    let (branch_label, branch_style) = match &status.head {
        Head::Branch(name) => (name.clone(), theme.branch_name()),
        Head::Detached(commit) => (format!("{commit} (detached)"), theme.danger()),
        Head::Unknown => ("unknown".to_string(), theme.muted()),
    };

    let mut spans = vec![
        Span::styled(status.name.clone(), theme.repo_name()),
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
}

fn revision_summary(
    status: Option<&RepositoryStatus>,
    revision: &RevisionData,
    theme: Theme,
    max_width: usize,
) -> Line<'static> {
    let repository_name =
        status.map_or_else(|| "no repository".to_string(), |status| status.name.clone());
    let subject = if revision.subject.is_empty() {
        "(no subject)"
    } else {
        &revision.subject
    };

    let prefix = vec![
        Span::styled(repository_name, theme.repo_name()),
        Span::styled("  ·  ", Style::default()),
        Span::styled(revision.short_oid.clone(), theme.branch_name()),
        Span::styled("  ·  ", Style::default()),
    ];
    let subject_width = max_width.saturating_sub(Line::from(prefix.clone()).width());
    let subject = truncate_with_ellipsis(subject, subject_width);

    Line::from(
        prefix
            .into_iter()
            .chain([Span::styled(subject, theme.muted())])
            .collect::<Vec<_>>(),
    )
}

fn truncate_with_ellipsis(text: &str, max_width: usize) -> String {
    if Line::raw(text).width() <= max_width {
        return text.to_string();
    }
    if max_width == 0 {
        return String::new();
    }

    let ellipsis = '…';
    let content_width = max_width.saturating_sub(Line::raw(ellipsis.to_string()).width());
    let mut truncated = String::new();

    for character in text.chars() {
        truncated.push(character);
        if Line::raw(&truncated).width() > content_width {
            truncated.pop();
            break;
        }
    }

    truncated.push(ellipsis);
    truncated
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
