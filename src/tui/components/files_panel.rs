use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::{List, ListItem};
use ratatui::{Frame, layout::Rect};

const BORDER_WIDTH: usize = 2;
const STATUS_LETTER_WIDTH: usize = 2;

use crate::app::App;
use crate::git::repository::FileStatus;
use crate::state::{TreeRow, tree_rows};
use crate::tui::components::panel;

pub fn draw(frame: &mut Frame, area: Rect, app: &mut App, is_focused: bool) {
    let theme = app.theme();
    let block = panel::block("[1] files", theme, is_focused);

    let Some(files) = app.files().cloned() else {
        let list_state = app.files_list_state();
        let items = vec![ListItem::new(Line::from(Span::styled(
            "unable to read git status",
            theme.muted(),
        )))];

        let list = List::new(items)
            .block(block)
            .highlight_style(Style::default().bg(theme.select));

        frame.render_stateful_widget(list, area, list_state);
        return;
    };

    let rows = tree_rows(&files);
    app.set_tree_row_count(rows.len());

    let selected_index = app.files_state().selected;
    let list_state = app.files_list_state();

    let mut items: Vec<ListItem> = Vec::new();
    if files.is_empty() {
        items.push(ListItem::new(Line::from(Span::styled(
            "working tree clean",
            theme.muted(),
        ))));
    }

    let selected_row = get_selected_row(&rows, selected_index);
    list_state.select(selected_row);

    for row in rows {
        match row {
            TreeRow::Dir(dir_name, depth) => {
                let path_depth = "  ".repeat(depth);
                items.push(ListItem::new(Line::from(vec![
                    Span::raw(path_depth),
                    Span::styled(dir_name, theme.text_primary()),
                ])));
            }
            TreeRow::File(entry, depth) => {
                let status_letter = match entry.status {
                    FileStatus::Staged => Span::styled("S ", theme.staged()),
                    FileStatus::Partial => Span::styled("P ", theme.partial()),
                    FileStatus::Conflicted => Span::styled("! ", theme.conflict()),
                    FileStatus::Unstaged => Span::styled("M ", theme.unstaged()),
                    FileStatus::Untracked => Span::styled("U ", theme.untracked()),
                };

                let path = entry.path.split('/').next_back().unwrap_or(&entry.path);
                let path_depth = "  ".repeat(depth);

                let insertions = humanize_stat('+', entry.insertions);
                let deletions = humanize_stat('-', entry.deletions);
                let stats = format!("{insertions}{deletions}");
                let padding_width = area.width as usize
                    - path_depth.len()
                    - BORDER_WIDTH
                    - path.len()
                    - STATUS_LETTER_WIDTH
                    - stats.len();
                let stats_padding = " ".repeat(padding_width);

                items.push(ListItem::new(Line::from(vec![
                    Span::raw(path_depth),
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

fn get_selected_row(_rows: &[TreeRow<'_>], selected: Option<usize>) -> Option<usize> {
    selected
}

fn humanize_stat(prefix: char, n: usize) -> String {
    if n >= 1000 {
        format!("{prefix}{}K ", n / 1000)
    } else {
        format!("{prefix}{n} ")
    }
}
