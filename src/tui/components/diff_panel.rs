use ratatui::layout::Alignment;
use ratatui::style::{Modifier, Style, Stylize};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::{Frame, layout::Rect};

use crate::app::App;
use crate::git::repository::{DiffHunk, DiffLine, FileStatus};
use crate::state::LineIndex;
use crate::tui::theme::Theme;

pub fn draw(frame: &mut Frame, area: Rect, app: &App, is_focused: bool) {
    let theme = app.theme();
    let border_style = if is_focused {
        theme.focused_border()
    } else {
        theme.panel_border()
    };
    let block = Block::default()
        .title(Line::from(vec![Span::styled(
            "[0] diff",
            Style::default()
                .fg(theme.accent)
                .add_modifier(Modifier::BOLD),
        )]))
        .title(diff_title(app).alignment(Alignment::Right))
        .borders(Borders::ALL)
        .border_style(border_style);

    let diff_state = app.diff_state();
    let hunks = app.diff_hunks();

    let lines = if let Some(hunks) = hunks {
        if hunks.is_empty() {
            vec![Line::from(Span::styled("no changes", theme.muted()))]
        } else {
            render_diff_lines(
                diff_state.scroll_offset,
                area.height.saturating_sub(2) as usize,
                hunks,
                &diff_state.line_index,
                diff_state.selected_hunk,
                diff_state.show_line_numbers,
                theme,
                area.width as usize,
            )
        }
    } else {
        vec![Line::from(Span::styled(
            "no available diffs",
            theme.muted(),
        ))]
    };

    let paragraph = Paragraph::new(Text::from(lines)).block(block);
    frame.render_widget(paragraph, area);
}

fn diff_title(app: &App) -> Line<'static> {
    let theme = app.theme();
    let selected_file = app.selected_file();
    let diff_state = app.diff_state();
    let hunks = app.diff_hunks();
    let selected_hunk = diff_state.selected_hunk.map_or(0, |idx| idx + 1);
    let hunk_count = hunks.map_or(0, Vec::len);
    let selected_line = diff_state.selected_line;
    let line_count = hunks
        .and_then(|h| diff_state.selected_hunk.and_then(|idx| h.get(idx)))
        .map_or(0, |hunk| hunk.lines.len());

    let mut spans = Vec::new();
    if let Some(file) = selected_file {
        spans.push(Span::styled(format!(" {}", file.path), theme.muted()));
        spans.push(Span::styled(" · ", theme.muted()));
    }
    spans.push(Span::styled(
        format!("hunk {selected_hunk}/{hunk_count}"),
        theme.muted(),
    ));
    if line_count > 0 {
        spans.push(Span::styled(" · ", theme.muted()));
        spans.push(Span::styled(
            format!("line {}/{}", selected_line + 1, line_count),
            theme.muted(),
        ));
    }
    if let Some(file) = selected_file {
        spans.push(Span::styled(" · ", theme.muted()));
        spans.push(Span::styled(
            format!("+{}", file.insertions),
            theme.success(),
        ));
        spans.push(Span::styled(" ", theme.muted()));
        spans.push(Span::styled(
            format!("-{}", file.deletions),
            theme.unstaged(),
        ));
        spans.push(Span::styled(" · ", theme.muted()));

        let status_color = match file.status {
            FileStatus::Staged => theme.staged(),
            FileStatus::Partial => theme.partial(),
            FileStatus::Unstaged => theme.unstaged(),
            FileStatus::Untracked => theme.untracked(),
            FileStatus::Conflicted => theme.conflict(),
        };

        spans.push(
            Span::styled(file.status.label().to_lowercase(), status_color)
                .add_modifier(Modifier::BOLD),
        );
    }

    Line::from(spans)
}

fn hunk_header_line(
    width: usize,
    index: usize,
    total: usize,
    header: &str,
    stats: (usize, usize),
    is_selected: bool,
    theme: crate::tui::theme::Theme,
) -> Line<'static> {
    let style = if is_selected {
        Style::default()
            .fg(theme.accent)
            .add_modifier(Modifier::BOLD)
    } else {
        theme.muted()
    };
    let prefix = format!(" hunk {}/{} ", index + 1, total);
    let (insertions, deletions) = stats;
    let stat_text = format!(" +{insertions} -{deletions}");
    let padding = width.saturating_sub(prefix.len() + header.len() + stat_text.len());

    Line::from(vec![
        Span::styled(prefix, style),
        Span::styled(header.to_string(), style),
        Span::styled(" ".repeat(padding), style),
        Span::styled(format!("+{insertions}"), Style::default().fg(theme.add_fg)),
        Span::styled(" ", style),
        Span::styled(format!("-{deletions}"), Style::default().fg(theme.del_fg)),
    ])
}

fn render_diff_lines(
    scroll_offset: usize,
    viewport_height: usize,
    hunks: &[DiffHunk],
    line_index: &LineIndex,
    selected_hunk: Option<usize>,
    show_line_numbers: bool,
    theme: Theme,
    panel_width: usize,
) -> Vec<Line<'static>> {
    let row_width = panel_width.saturating_sub(2);
    let visible_rows = scroll_offset..scroll_offset + viewport_height;

    visible_rows
        .filter_map(|global_row| {
            let (hunk_idx, line_in_hunk) = line_index.lookup(global_row)?;
            let hunk = &hunks[hunk_idx];
            let is_selected = Some(hunk_idx) == selected_hunk;

            if line_in_hunk == 0 {
                Some(hunk_header_line(
                    row_width,
                    hunk_idx,
                    hunks.len(),
                    hunk.header.trim_end(),
                    (hunk.insertions, hunk.deletions),
                    is_selected,
                    theme,
                ))
            } else {
                Some(diff_line(
                    row_width,
                    &hunk.lines[line_in_hunk - 1],
                    is_selected,
                    show_line_numbers,
                    theme,
                ))
            }
        })
        .collect()
}

fn diff_line(
    width: usize,
    line: &DiffLine,
    is_selected: bool,
    show_line_numbers: bool,
    theme: crate::tui::theme::Theme,
) -> Line<'static> {
    let base = match line.origin {
        '+' => theme.diff_add(),
        '-' => theme.diff_del(),
        _ => theme.muted(),
    };
    let selected = match line.origin {
        '+' => Style::default().fg(theme.add_fg).bg(theme.select_hi),
        '-' => Style::default().fg(theme.del_fg).bg(theme.select_hi),
        _ => theme.muted(),
    };
    let style = if is_selected { selected } else { base };
    let content = line.content.trim_end().replace('\t', "    ");
    let text = if show_line_numbers {
        let old_lineno = line_number(line.old_lineno);
        let new_lineno = line_number(line.new_lineno);
        format!("{old_lineno} {new_lineno} {} {}", line.origin, content)
    } else {
        format!("{} {}", line.origin, content)
    };

    bordered_line(width, text, style, border_style(line, is_selected, theme))
}

fn line_number(line_number: Option<u32>) -> String {
    line_number.map_or_else(|| "    ".to_string(), |number| format!("{number:>4}"))
}

fn border_style(
    line: &DiffLine,
    is_selected: bool,
    theme: crate::tui::theme::Theme,
) -> Option<Style> {
    if !is_selected {
        return None;
    }

    match line.origin {
        '+' => Some(Style::default().fg(theme.add_gutter)),
        '-' => Some(Style::default().fg(theme.del_gutter)),
        _ => None,
    }
}

fn bordered_line(
    width: usize,
    text: String,
    style: Style,
    border_style: Option<Style>,
) -> Line<'static> {
    let content_width = width.saturating_sub(1);
    let padding = content_width.saturating_sub(text.len());
    let border = border_style.map_or_else(
        || Span::styled(" ", style),
        |border| Span::styled("▌", border),
    );

    Line::from(vec![
        border,
        Span::styled(text, style),
        Span::styled(" ".repeat(padding), style),
    ])
}
