use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::{List, ListItem};
use ratatui::{Frame, layout::Rect};

use crate::app::App;
use crate::git::repository::FileStatus;
use crate::tui::components::panel;

pub fn draw(frame: &mut Frame, area: Rect, app: &mut App, is_focused: bool) {
    let theme = app.theme();
    let block = panel::block("files", theme, is_focused);

    let files = app.files().cloned().unwrap_or_default();
    let mut items: Vec<ListItem> = Vec::new();

    let groups = [
        ("STAGED", FileStatus::Staged),
        ("PARTIAL", FileStatus::Partial),
        ("CONFLICTED", FileStatus::Conflicted),
        ("UNSTAGED", FileStatus::Unstaged),
        ("UNTRACKED", FileStatus::Untracked),
    ];

    for (label, status) in &groups {
        let matching: Vec<_> = files.iter().filter(|f| matches!(&f.status, s if std::mem::discriminant(s) == std::mem::discriminant(status))).collect();
        if matching.is_empty() {
            continue;
        }

        items.push(ListItem::new(Line::from(Span::styled(
            format!("{} {}", label, matching.len()),
            theme.muted(),
        ))));

        let max_stats_width = matching
            .iter()
            .map(|f| format!("+{} -{}", f.insertions, f.deletions).len())
            .max()
            .unwrap_or(0);

        let usable = area.width as usize - 2;
        let path_width = usable - 2 - max_stats_width;

        for file in matching {
            let status_letter = match file.status {
                FileStatus::Staged => Span::styled("S ", theme.staged()),
                FileStatus::Partial => Span::styled("P ", theme.unstaged()),
                FileStatus::Conflicted => Span::styled("! ", theme.conflict()),
                FileStatus::Unstaged => Span::styled("M ", theme.unstaged()),
                FileStatus::Untracked => Span::styled("U ", theme.untracked()),
            };

            let path = if file.path.len() > path_width {
                let segments: Vec<&str> = file.path.split('/').collect();
                let short = if segments.len() >= 2 {
                    format!(
                        "…/{}/{}",
                        segments[segments.len() - 2],
                        segments[segments.len() - 1]
                    )
                } else {
                    format!("…{}", &segments[segments.len() - 1])
                };
                format!("{:<path_width$}", short, path_width = path_width)
            } else {
                format!("{:<path_width$}", file.path, path_width = path_width)
            };

            let stats_str = format!(
                "{:>width$}",
                format!("+{} -{}", file.insertions, file.deletions),
                width = max_stats_width
            );

            items.push(ListItem::new(Line::from(vec![
                status_letter,
                // TODO: truncate long paths to filename only
                Span::styled(path, theme.text_primary()),
                Span::styled(stats_str, theme.muted()),
            ])));
        }
    }
    let list = List::new(items)
        .block(block)
        .highlight_style(Style::default().bg(theme.select));

    frame.render_stateful_widget(list, area, &mut app.files_state_mut().list);
}
