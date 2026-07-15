use ratatui::style::Style;
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
    let block = panel::block(None, theme, Borders::NONE, theme.sidebar_bg, is_focused);

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

        for (i, row) in rows.iter().enumerate() {
            match row {
                TreeRow::Dir(dir_name, depth) => {
                    let path_depth = "  ".repeat(*depth);
                    let symbol = if collapsed.contains(dir_name) {
                        "› "
                    } else {
                        "⌄ "
                    };
                    let segs: Vec<&str> = dir_name.split('/').collect();
                    let label = if segs.len() > *depth {
                        let next_is_child_dir = rows
                            .get(i + 1)
                            .is_some_and(|r| matches!(r, TreeRow::Dir(_, d) if *d == depth + 1));
                        if next_is_child_dir {
                            segs[depth.saturating_sub(1)].to_string()
                        } else {
                            segs[depth.saturating_sub(1)..].join("/")
                        }
                    } else {
                        segs.last().copied().unwrap_or(dir_name).to_string()
                    };
                    let dir_style = if is_focused {
                        theme.accent()
                    } else {
                        theme.muted()
                    };
                    items.push(ListItem::new(Line::from(vec![
                        Span::raw(path_depth),
                        Span::styled(symbol, dir_style),
                        Span::styled(label, dir_style),
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

                    let fixed_width = path_depth.len() + BORDER_WIDTH + STATUS_LETTER_WIDTH;
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

                    items.push(ListItem::new(Line::from(vec![
                        Span::raw(path_depth),
                        status_letter,
                        Span::styled(display_path, theme.text_primary()),
                    ])));
                }
            }
        }
    }

    let mut list_state = ListState::default();
    list_state.select(selected_row);

    let select_bg = if is_focused {
        theme.select_hi
    } else {
        theme.select
    };
    let list = List::new(items)
        .block(block)
        .highlight_style(Style::default().bg(select_bg));

    frame.render_stateful_widget(list, area, &mut list_state);
}

fn get_selected_row(rows: &[TreeRow], selected: Option<usize>) -> Option<usize> {
    selected.filter(|index| *index < rows.len())
}

#[cfg(test)]
mod tests {
    use super::get_selected_row;
    use crate::state::TreeRow;

    #[test]
    fn empty_tree_does_not_select_the_status_message() {
        assert_eq!(get_selected_row(&[], Some(0)), None);
    }

    #[test]
    fn selection_is_limited_to_tree_rows() {
        let rows = [TreeRow::File(0, 0)];

        assert_eq!(get_selected_row(&rows, Some(0)), Some(0));
        assert_eq!(get_selected_row(&rows, Some(1)), None);
    }
}
