use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Borders, List, ListItem, ListState};
use ratatui::{Frame, layout::Rect};

const BORDER_WIDTH: usize = 2;
const STATUS_LETTER_WIDTH: usize = 2;

use crate::app::App;
use crate::git::repository::FileStatus;
use crate::state::TreeRow;
use crate::tui::components::panel;

pub fn draw(frame: &mut Frame, area: Rect, app: &mut App, is_focused: bool) {
    let theme = app.theme();
    let block = panel::block(
        Line::from(vec![Span::styled(
            " files",
            Style::default()
                .fg(theme.accent)
                .add_modifier(Modifier::BOLD),
        )]),
        theme,
        Borders::NONE,
        theme.sidebar_bg,
        is_focused,
    );

    app.ensure_rows();
    let selected_index = app.files_state().selected;
    app.set_tree_row_count(app.cached_rows().len());
    let files = app.files();

    let collapsed = app.collapsed_files().clone();
    let selected_row;
    let mut items: Vec<ListItem> = Vec::new();
    {
        let rows = app.cached_rows();
        selected_row = get_selected_row(rows, selected_index);

        if files.is_empty() {
            items.push(ListItem::new(Line::from(Span::styled(
                "working tree clean",
                theme.muted(),
            ))));
        }

        for row in rows {
            match row {
                TreeRow::Dir(dir_name, depth) => {
                    let path_depth = "  ".repeat(*depth);
                    let symbol = if collapsed.contains(dir_name) {
                        "› "
                    } else {
                        "⌄ "
                    };
                    items.push(ListItem::new(Line::from(vec![
                        Span::raw(path_depth),
                        Span::styled(symbol, theme.muted()),
                        Span::styled(dir_name, theme.muted()),
                    ])));
                }
                TreeRow::File(idx, depth) => {
                    let entry = &files[*idx].entry;
                    let status_letter = match entry.status {
                        FileStatus::Staged => Span::styled("S ", theme.staged()),
                        FileStatus::Partial => Span::styled("P ", theme.partial()),
                        FileStatus::Conflicted => Span::styled("! ", theme.conflict()),
                        FileStatus::Unstaged => Span::styled("M ", theme.unstaged()),
                        FileStatus::Untracked => Span::styled("U ", theme.untracked()),
                    };

                    let path = entry.path.split('/').next_back().unwrap_or(&entry.path);
                    let path_depth = "  ".repeat(*depth);

                    let insertions = humanize_stat('+', entry.insertions);
                    let deletions = humanize_stat('-', entry.deletions);
                    let stats = format!("{insertions}{deletions}");

                    let fixed_width =
                        path_depth.len() + BORDER_WIDTH + STATUS_LETTER_WIDTH + stats.len();
                    let max_path_width = (area.width as usize).saturating_sub(fixed_width);

                    let display_path = if path.chars().count() > max_path_width {
                        let mut truncated = String::new();
                        for (count, ch) in path.chars().enumerate() {
                            if count + 1 >= max_path_width {
                                break;
                            }
                            truncated.push(ch);
                        }
                        truncated.push('…');
                        truncated
                    } else {
                        path.to_string()
                    };

                    let padding_width = (area.width as usize)
                        .saturating_sub(path_depth.len())
                        .saturating_sub(BORDER_WIDTH)
                        .saturating_sub(display_path.chars().count())
                        .saturating_sub(STATUS_LETTER_WIDTH)
                        .saturating_sub(stats.len());
                    let stats_padding = " ".repeat(padding_width);

                    items.push(ListItem::new(Line::from(vec![
                        Span::raw(path_depth),
                        status_letter,
                        Span::styled(display_path, theme.text_primary()),
                        Span::raw(stats_padding),
                        Span::styled(insertions, theme.staged()),
                        Span::styled(deletions, theme.unstaged()),
                    ])));
                }
            }
        }
    }

    let mut list_state = ListState::default();
    list_state.select(selected_row);

    let list = List::new(items)
        .block(block)
        .highlight_style(Style::default().bg(theme.select));

    frame.render_stateful_widget(list, area, &mut list_state);
}

const fn get_selected_row(_rows: &[TreeRow], selected: Option<usize>) -> Option<usize> {
    selected
}

fn humanize_stat(prefix: char, n: usize) -> String {
    if n >= 1000 {
        format!("{prefix}{}K ", n / 1000)
    } else {
        format!("{prefix}{n} ")
    }
}
