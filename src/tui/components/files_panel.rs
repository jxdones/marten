use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::{List, ListItem};
use ratatui::{Frame, layout::Rect};

use crate::app::App;
use crate::git::repository::FileStatus;
use crate::state::{FilePanelRow, file_panel_rows};
use crate::tui::components::panel;

pub fn draw(frame: &mut Frame, area: Rect, app: &mut App, is_focused: bool) {
    let theme = app.theme();
    let block = panel::block("files", theme, is_focused);

    let mut items: Vec<ListItem> = Vec::new();
    let Some(files) = app.files().cloned() else {
        items.push(ListItem::new(Line::from(Span::styled(
            "unable to read git status",
            theme.muted(),
        ))));

        let list = List::new(items)
            .block(block)
            .highlight_style(Style::default().bg(theme.select));

        frame.render_stateful_widget(list, area, &mut app.files_state_mut().list);
        return;
    };

    if files.is_empty() {
        items.push(ListItem::new(Line::from(Span::styled(
            "working tree clean",
            theme.muted(),
        ))));
    }

    for row in file_panel_rows(&files) {
        match row {
            FilePanelRow::Header { status, count } => {
                items.push(ListItem::new(Line::from(Span::styled(
                    format!("{} {}", status.label(), count),
                    theme.muted(),
                ))));
            }
            FilePanelRow::File { entry, stats_width } => {
                let usable = (area.width as usize).saturating_sub(2);
                let path_width = usable.saturating_sub(2 + stats_width);

                let status_letter = match entry.status {
                    FileStatus::Staged => Span::styled("S ", theme.staged()),
                    FileStatus::Partial => Span::styled("P ", theme.unstaged()),
                    FileStatus::Conflicted => Span::styled("! ", theme.conflict()),
                    FileStatus::Unstaged => Span::styled("M ", theme.unstaged()),
                    FileStatus::Untracked => Span::styled("U ", theme.untracked()),
                };

                let path = if path_width == 0 {
                    String::new()
                } else if entry.path.len() > path_width {
                    let segments: Vec<&str> = entry.path.split('/').collect();
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
                    format!("{:<path_width$}", entry.path, path_width = path_width)
                };

                let insertions = humanize_stat('+', entry.insertions);
                let deletions = humanize_stat('-', entry.deletions);
                let stats = format!("{insertions}{deletions}");
                let stats_padding = " ".repeat(stats_width.saturating_sub(stats.len()));

                items.push(ListItem::new(Line::from(vec![
                    status_letter,
                    // TODO: truncate long paths to filename only
                    Span::styled(path, theme.text_primary()),
                    Span::raw(stats_padding),
                    Span::styled(insertions, theme.staged()),
                    Span::styled(deletions, theme.unstaged()),
                ])));
            }
        }
    }
    let list = List::new(items)
        .block(block)
        .highlight_style(Style::default().bg(theme.select));

    frame.render_stateful_widget(list, area, &mut app.files_state_mut().list);
}

fn humanize_stat(prefix: char, n: usize) -> String {
    if n >= 1000 {
        format!("{prefix}{}K ", n / 1000)
    } else {
        format!("{prefix}{n} ")
    }
}

// TODO: Add tests for stat formatting and file row rendering.
