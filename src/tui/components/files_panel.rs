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
    let block = panel::block("[1] files", theme, is_focused);

    let Some(files) = app.files().cloned() else {
        let list_state = app.files_list_state();
        let mut items = Vec::new();
        items.push(ListItem::new(Line::from(Span::styled(
            "unable to read git status",
            theme.muted(),
        ))));

        let list = List::new(items)
            .block(block)
            .highlight_style(Style::default().bg(theme.select));

        frame.render_stateful_widget(list, area, list_state);
        return;
    };

    let selected_index = app.files_state().selected;
    let list_state = app.files_list_state();

    let mut items: Vec<ListItem> = Vec::new();
    if files.is_empty() {
        items.push(ListItem::new(Line::from(Span::styled(
            "working tree clean",
            theme.muted(),
        ))));
    }

    let rows = file_panel_rows(&files);
    let selected_row = get_selected_row(&rows, selected_index);
    list_state.select(selected_row);

    let stats_width = rows
        .iter()
        .filter_map(|row| match row {
            FilePanelRow::File { entry } => Some(
                humanize_stat('+', entry.insertions).len()
                    + humanize_stat('-', entry.deletions).len(),
            ),
            _ => None,
        })
        .max()
        .unwrap_or(0);

    for row in rows {
        match row {
            FilePanelRow::Header { status, count } => {
                items.push(ListItem::new(Line::from(Span::styled(
                    format!("{} {}", status.label(), count),
                    theme.muted(),
                ))));
            }
            FilePanelRow::File { entry } => {
                let usable = (area.width as usize).saturating_sub(2);
                let path_width = usable.saturating_sub(2 + stats_width);

                let status_letter = match entry.status {
                    FileStatus::Staged => Span::styled("S ", theme.staged()),
                    FileStatus::Partial => Span::styled("P ", theme.partial()),
                    FileStatus::Conflicted => Span::styled("! ", theme.conflict()),
                    FileStatus::Unstaged => Span::styled("M ", theme.unstaged()),
                    FileStatus::Untracked => Span::styled("U ", theme.untracked()),
                };

                let path = format_path(&entry.path, path_width);

                let insertions = humanize_stat('+', entry.insertions);
                let deletions = humanize_stat('-', entry.deletions);
                let stats = format!("{insertions}{deletions}");
                let stats_padding = " ".repeat(stats_width.saturating_sub(stats.len()));

                items.push(ListItem::new(Line::from(vec![
                    status_letter,
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

    frame.render_stateful_widget(list, area, list_state);
}

fn get_selected_row(rows: &[FilePanelRow<'_>], selected: Option<usize>) -> Option<usize> {
    let mut file_count = 0;
    for (row_index, row) in rows.iter().enumerate() {
        if let FilePanelRow::File { .. } = row {
            if Some(file_count) == selected {
                return Some(row_index);
            }
            file_count += 1;
        }
    }
    None
}

fn humanize_stat(prefix: char, n: usize) -> String {
    if n >= 1000 {
        format!("{prefix}{}K ", n / 1000)
    } else {
        format!("{prefix}{n} ")
    }
}

fn format_path(path: &str, width: usize) -> String {
    if width == 0 {
        return String::new();
    }

    if path.len() <= width {
        return format!("{path:<width$}");
    }

    let segments: Vec<&str> = path.split('/').collect();
    let filename = segments.last().copied().unwrap_or(path);
    let parent = segments
        .len()
        .checked_sub(2)
        .and_then(|index| segments.get(index))
        .copied();

    let short = if let Some(parent) = parent {
        let prefix = format!("…/{parent}/");
        if prefix.len() < width {
            format!(
                "{prefix}{}",
                truncate_middle(filename, width - prefix.len())
            )
        } else {
            truncate_middle(filename, width)
        }
    } else {
        truncate_middle(filename, width)
    };

    format!("{short:<width$}")
}

fn truncate_middle(value: &str, width: usize) -> String {
    let len = value.chars().count();
    if len <= width {
        return value.to_string();
    }

    if width == 1 {
        return "…".to_string();
    }

    let left_width = (width - 1) / 2;
    let right_width = width - 1 - left_width;
    let left: String = value.chars().take(left_width).collect();
    let right: String = value
        .chars()
        .skip(len.saturating_sub(right_width))
        .collect();

    format!("{left}…{right}")
}

// TODO: Add tests for stat formatting, path truncation, and file row rendering.
