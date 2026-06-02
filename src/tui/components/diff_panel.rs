use ratatui::layout::Alignment;
use ratatui::style::{Modifier, Style, Stylize};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::{Frame, layout::Rect};

use crate::app::App;
use crate::git::repository::{
    DIFF_LINE_THRESHOLD, DiffHunk, DiffLine, DiffSectionKind, FileEntry, FileStatus,
};
use crate::inline_diff::{self, Range};
use crate::state::line_index::IndexRow;
use crate::state::review::RenderedRow;
use crate::state::{DiffLoadState, LineIndex, ReviewDoc, ViewMode};
use crate::syntax;
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
            " diff",
            Style::default()
                .fg(theme.accent)
                .add_modifier(Modifier::BOLD),
        )]))
        .title(diff_title(app).alignment(Alignment::Right))
        .borders(Borders::ALL)
        .border_style(border_style)
        .style(Style::default().bg(theme.bg));

    let viewport_height = area.height.saturating_sub(2) as usize;
    let panel_width = area.width as usize;

    let lines = match app.review_state().mode {
        ViewMode::SingleFile => render_single_file(app, theme, viewport_height, panel_width),
        ViewMode::Continuous => {
            let scroll = app.review_state().continuous_scroll;
            let review_doc = app.review_doc();
            let selected_hunk = match review_doc.lookup_row(scroll) {
                Some(RenderedRow::HunkHeader { file_idx, hunk_idx }) => Some((file_idx, hunk_idx)),
                Some(RenderedRow::DiffLine {
                    file_idx, hunk_idx, ..
                }) => Some((file_idx, hunk_idx)),
                _ => None,
            };
            render_review_doc(
                scroll,
                viewport_height,
                review_doc,
                selected_hunk,
                app.diff_state().show_line_numbers,
                theme,
                panel_width,
            )
        }
    };

    let paragraph = Paragraph::new(Text::from(lines)).block(block);
    frame.render_widget(paragraph, area);
}

fn diff_title(app: &App) -> Line<'static> {
    let theme = app.theme();

    let (file_path, selected_hunk, hunk_count, selected_line, line_count) =
        match app.review_state().mode {
            ViewMode::Continuous => {
                let review_doc = app.review_doc();
                let scroll = app.review_state().continuous_scroll;
                match review_doc.lookup_row(scroll) {
                    Some(RenderedRow::HunkHeader { file_idx, hunk_idx }) => {
                        let path = review_doc.files[file_idx].entry.path.clone();
                        let (total, line_count) = match &review_doc.files[file_idx].load {
                            DiffLoadState::Loaded { hunks, .. } => {
                                (hunks.len(), hunks[hunk_idx].lines.len())
                            }
                            _ => (0, 0),
                        };
                        (Some(path), hunk_idx + 1, total, 0, line_count)
                    }
                    Some(RenderedRow::DiffLine {
                        file_idx,
                        hunk_idx,
                        line_idx,
                    }) => {
                        let path = review_doc.files[file_idx].entry.path.clone();
                        let (total, line_count) = match &review_doc.files[file_idx].load {
                            DiffLoadState::Loaded { hunks, .. } => {
                                (hunks.len(), hunks[hunk_idx].lines.len())
                            }
                            _ => (0, 0),
                        };
                        (Some(path), hunk_idx + 1, total, line_idx, line_count)
                    }
                    Some(RenderedRow::FileHeader { file_idx })
                    | Some(RenderedRow::Binary { file_idx }) => {
                        let path = review_doc.files[file_idx].entry.path.clone();
                        (Some(path), 0, 0, 0, 0)
                    }
                    _ => (None, 0, 0, 0, 0),
                }
            }
            ViewMode::SingleFile => {
                let diff_state = app.diff_state();
                let hunks = app.diff_hunks();
                let selected_hunk = diff_state.selected_hunk.map_or(0, |idx| idx + 1);
                let hunk_count = hunks.map_or(0, Vec::len);
                let line_count = hunks
                    .and_then(|h| diff_state.selected_hunk.and_then(|idx| h.get(idx)))
                    .map_or(0, |hunk| hunk.lines.len());
                let path = app.selected_file().map(|f| f.path.clone());
                (
                    path,
                    selected_hunk,
                    hunk_count,
                    diff_state.selected_line,
                    line_count,
                )
            }
        };

    let mut spans = Vec::new();
    if let Some(path) = file_path {
        spans.push(Span::styled(format!(" {path}"), theme.muted()));
        spans.push(Span::styled(" · ", theme.muted()));
    }
    if hunk_count > 0 {
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
    }
    Line::from(spans)
}

fn section_header_line(width: usize, kind: DiffSectionKind, theme: Theme) -> Line<'static> {
    let (label, label_style) = match kind {
        DiffSectionKind::Staged => (" staged ", theme.staged().bg(theme.hunk_header_bg)),
        DiffSectionKind::Unstaged => (" unstaged ", theme.unstaged().bg(theme.hunk_header_bg)),
    };
    let dash_style = label_style;
    let dashes = "─".repeat(width.saturating_sub(label.len()));
    Line::from(vec![
        Span::styled(label, label_style.add_modifier(Modifier::BOLD)),
        Span::styled(dashes, dash_style),
    ])
    .style(Style::default().bg(theme.hunk_header_bg))
}

fn hunk_header_line(
    width: usize,
    index: usize,
    total: usize,
    header: &str,
    stats: (usize, usize),
    is_selected: bool,
    theme: Theme,
) -> Line<'static> {
    let style = if is_selected {
        Style::default()
            .fg(theme.accent)
            .bg(theme.hunk_header_bg)
            .add_modifier(Modifier::BOLD)
    } else {
        theme.hunk_header()
    };
    let prefix = format!(" hunk {}/{} ", index + 1, total);
    let (insertions, deletions) = stats;
    let stat_text = format!(" +{insertions} -{deletions}");
    let padding =
        width.saturating_sub(text_width(&prefix) + text_width(header) + text_width(&stat_text));

    Line::from(vec![
        Span::styled(prefix, style),
        Span::styled(header.to_string(), style),
        Span::styled(" ".repeat(padding), style),
        Span::styled(
            format!("+{insertions}"),
            Style::default().fg(theme.add_fg).bg(theme.hunk_header_bg),
        ),
        Span::styled(" ", style),
        Span::styled(
            format!("-{deletions}"),
            Style::default().fg(theme.del_fg).bg(theme.hunk_header_bg),
        ),
    ])
    .style(Style::default().bg(theme.hunk_header_bg))
}

fn render_single_file(
    app: &App,
    theme: Theme,
    viewport_height: usize,
    panel_width: usize,
) -> Vec<Line<'static>> {
    let diff_state = app.diff_state();
    let hunks = app.diff_hunks();
    let path = app.selected_file().map(|file| file.path.as_str());

    if diff_state.is_binary {
        vec![Line::from(Span::styled(
            "Binary file — skipped",
            theme.muted(),
        ))]
    } else if let Some(line_count) = diff_state.too_large {
        vec![
            Line::from(Span::styled(
                "File too large to render automatically.",
                theme.muted(),
            )),
            Line::from(Span::styled(
                format!("{line_count} lines (threshold: {DIFF_LINE_THRESHOLD})"),
                theme.muted(),
            )),
            Line::from(Span::styled("Press Enter to view.", theme.muted())),
        ]
    } else if let Some(hunks) = hunks {
        if hunks.is_empty() {
            vec![Line::from(Span::styled("no changes", theme.muted()))]
        } else {
            render_diff_lines(
                diff_state.scroll_offset,
                viewport_height,
                hunks,
                path,
                &diff_state.line_index,
                diff_state.selected_hunk,
                diff_state.show_line_numbers,
                theme,
                panel_width,
            )
        }
    } else {
        vec![Line::from(Span::styled(
            "no available diffs",
            theme.muted(),
        ))]
    }
}

#[allow(clippy::too_many_arguments)]
fn render_diff_lines(
    scroll_offset: usize,
    viewport_height: usize,
    hunks: &[DiffHunk],
    path: Option<&str>,
    line_index: &LineIndex,
    selected_hunk: Option<usize>,
    show_line_numbers: bool,
    theme: Theme,
    panel_width: usize,
) -> Vec<Line<'static>> {
    let row_width = panel_width.saturating_sub(2);
    let visible_rows = scroll_offset..scroll_offset + viewport_height;

    visible_rows
        .filter_map(|global_row| match line_index.lookup(global_row)? {
            IndexRow::SectionHeader(section_idx) => {
                let kind = line_index.section_header_rows[section_idx].1;
                Some(section_header_line(row_width, kind, theme))
            }
            IndexRow::HunkHeader(hunk_idx) => {
                let hunk = &hunks[hunk_idx];
                let is_selected = Some(hunk_idx) == selected_hunk;
                Some(hunk_header_line(
                    row_width,
                    hunk_idx,
                    hunks.len(),
                    hunk.header.trim_end(),
                    (hunk.insertions, hunk.deletions),
                    is_selected,
                    theme,
                ))
            }
            IndexRow::DiffLine(hunk_idx, line_idx) => {
                let hunk = &hunks[hunk_idx];
                let is_selected = Some(hunk_idx) == selected_hunk;
                let line = &hunk.lines[line_idx];
                let ranges = inline_ranges(hunk, line_idx, line.origin);
                Some(diff_line(
                    row_width,
                    line,
                    path,
                    &ranges,
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
    path: Option<&str>,
    inline_ranges: &[Range],
    is_selected: bool,
    show_line_numbers: bool,
    theme: Theme,
) -> Line<'static> {
    let base = match line.origin {
        '+' => theme.diff_add(),
        '-' => theme.diff_del(),
        _ => theme.muted().bg(theme.bg),
    };
    let line_style = base;
    let content_style = base;
    let content = line.content.trim_end().replace('\t', "    ");
    let prefix = if show_line_numbers {
        let old_lineno = line_number(line.old_lineno);
        let new_lineno = line_number(line.new_lineno);
        format!("{old_lineno} {new_lineno} {} ", line.origin)
    } else {
        format!("{} ", line.origin)
    };

    let mut spans = vec![Span::styled(prefix, line_style)];
    if let Some(path) = path {
        if let Some(highlighted) = syntax::highlight_line(path, &content, content_style) {
            spans.extend(style_content_spans(
                highlighted,
                inline_ranges,
                line.origin,
                false,
                theme,
            ));
        } else {
            spans.extend(style_content_spans(
                vec![Span::styled(content, content_style)],
                inline_ranges,
                line.origin,
                false,
                theme,
            ));
        }
    } else {
        spans.extend(style_content_spans(
            vec![Span::styled(content, content_style)],
            inline_ranges,
            line.origin,
            false,
            theme,
        ));
    }

    bordered_line(
        width,
        spans,
        line_style,
        border_style(line, is_selected, theme),
    )
}

fn inline_ranges(hunk: &DiffHunk, line_idx: usize, origin: char) -> Vec<Range> {
    match origin {
        '-' => {
            let Some(next_line) = hunk.lines.get(line_idx + 1) else {
                return Vec::new();
            };
            if next_line.origin != '+' {
                return Vec::new();
            }

            let (old_ranges, _) = inline_diff::changed_ranges(
                hunk.lines[line_idx].content.trim_end(),
                next_line.content.trim_end(),
            );
            old_ranges
        }
        '+' => {
            if line_idx == 0 {
                return Vec::new();
            }

            let Some(previous_line) = hunk.lines.get(line_idx - 1) else {
                return Vec::new();
            };
            if previous_line.origin != '-' {
                return Vec::new();
            }

            let (_, new_ranges) = inline_diff::changed_ranges(
                previous_line.content.trim_end(),
                hunk.lines[line_idx].content.trim_end(),
            );
            new_ranges
        }
        _ => Vec::new(),
    }
}

fn style_content_spans(
    spans: Vec<Span<'static>>,
    ranges: &[Range],
    origin: char,
    is_selected: bool,
    theme: Theme,
) -> Vec<Span<'static>> {
    if ranges.is_empty() && !is_selected {
        return spans;
    }

    let mut highlighted = Vec::new();
    let mut offset = 0;
    let inline_bg = match origin {
        '+' => theme.add_inline_bg,
        '-' => theme.del_inline_bg,
        _ => theme.select_hi,
    };
    let inline_overlay = Style::default().bg(inline_bg).add_modifier(Modifier::BOLD);
    let selected_overlay = Style::default().bg(theme.select_hi);

    for span in spans {
        let mut segment = String::new();
        let mut segment_changed = false;
        let mut segment_started = false;

        for (local_idx, ch) in span.content.chars().enumerate() {
            let changed = in_ranges(offset + local_idx, ranges);
            if segment_started && changed != segment_changed {
                let style = if segment_changed {
                    span.style.patch(inline_overlay)
                } else if is_selected {
                    span.style.patch(selected_overlay)
                } else {
                    span.style
                };
                highlighted.push(Span::styled(std::mem::take(&mut segment), style));
            }

            segment_started = true;
            segment_changed = changed;
            segment.push(ch);
        }

        if !segment.is_empty() {
            let style = if segment_changed {
                span.style.patch(inline_overlay)
            } else if is_selected {
                span.style.patch(selected_overlay)
            } else {
                span.style
            };
            highlighted.push(Span::styled(segment, style));
        }

        offset += span.content.chars().count();
    }

    highlighted
}

fn in_ranges(index: usize, ranges: &[Range]) -> bool {
    ranges
        .iter()
        .any(|(start, end)| *start <= index && index < *end)
}

fn line_number(line_number: Option<u32>) -> String {
    line_number.map_or_else(|| "    ".to_string(), |number| format!("{number:>4}"))
}

fn border_style(line: &DiffLine, is_selected: bool, theme: Theme) -> Option<Style> {
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
    spans: Vec<Span<'static>>,
    style: Style,
    border_style: Option<Style>,
) -> Line<'static> {
    let content_width = width.saturating_sub(1);
    let used_width = spans
        .iter()
        .map(|span| text_width(span.content.as_ref()))
        .sum::<usize>();
    let padding = content_width.saturating_sub(used_width);
    let border = border_style.map_or_else(
        || Span::styled(" ", style),
        |border| Span::styled("▌", border),
    );

    let mut line_spans = Vec::with_capacity(spans.len() + 2);
    line_spans.push(border);
    line_spans.extend(spans);
    line_spans.push(Span::styled(" ".repeat(padding), style));

    Line::from(line_spans).style(style)
}

fn file_status_color(status: FileStatus, theme: Theme) -> Style {
    match status {
        FileStatus::Staged => theme.staged(),
        FileStatus::Partial => theme.partial(),
        FileStatus::Unstaged => theme.unstaged(),
        FileStatus::Untracked => theme.untracked(),
        FileStatus::Conflicted => theme.conflict(),
    }
}

fn file_status_symbol(status: FileStatus) -> &'static str {
    match status {
        FileStatus::Staged => "S",
        FileStatus::Partial => "P",
        FileStatus::Unstaged => "M",
        FileStatus::Untracked => "U",
        FileStatus::Conflicted => "C",
    }
}

fn render_file_header(width: usize, file: &FileEntry, theme: Theme) -> Line<'static> {
    let collapse_symbol = "▼ ";
    let bg = Style::default().bg(theme.file_header_bg);
    let mut spans = vec![Span::styled(collapse_symbol, theme.muted().patch(bg))];

    let status_color = file_status_color(file.status, theme);
    let status_symbol = file_status_symbol(file.status);
    let path = format!(" {}", file.path);
    let stats = format!(
        "+{} -{} {}",
        file.insertions,
        file.deletions,
        file.status.label().to_lowercase()
    );
    let padding = width.saturating_sub(
        text_width(collapse_symbol)
            + text_width(status_symbol)
            + text_width(&path)
            + text_width(&stats),
    );

    spans.push(Span::styled(status_symbol, status_color.patch(bg)));
    spans.push(Span::styled(path, theme.muted().patch(bg)));
    spans.push(Span::styled((" ").repeat(padding), bg));
    spans.push(Span::styled(
        format!("+{}", file.insertions),
        theme.success().patch(bg),
    ));
    spans.push(Span::styled(" ", theme.muted().patch(bg)));
    spans.push(Span::styled(
        format!("-{}", file.deletions),
        theme.unstaged().patch(bg),
    ));
    spans.push(Span::styled(" ", theme.muted().patch(bg)));
    spans.push(
        Span::styled(file.status.label().to_lowercase(), status_color.patch(bg))
            .add_modifier(Modifier::BOLD),
    );

    Line::from(spans).style(bg)
}

fn render_binary_header(width: usize, file: &FileEntry, theme: Theme) -> Line<'static> {
    let bg = Style::default().bg(theme.file_header_bg);
    let status_color = file_status_color(file.status, theme);
    let status_symbol = file_status_symbol(file.status);
    let prefix = "  ";
    let path = format!(" {}", file.path);
    let tag = " binary";
    let padding = width.saturating_sub(
        text_width(prefix) + text_width(status_symbol) + text_width(&path) + text_width(tag),
    );
    Line::from(vec![
        Span::styled(prefix, theme.muted().patch(bg)),
        Span::styled(status_symbol, status_color.patch(bg)),
        Span::styled(path, theme.muted().patch(bg)),
        Span::styled(" ".repeat(padding), bg),
        Span::styled(tag, theme.muted().patch(bg)),
    ])
    .style(bg)
}

fn text_width(text: &str) -> usize {
    text.chars().count()
}

fn render_review_doc(
    scroll_offset: usize,
    viewport_height: usize,
    review_doc: &ReviewDoc,
    selected_hunk: Option<(usize, usize)>,
    show_line_numbers: bool,
    theme: Theme,
    panel_width: usize,
) -> Vec<Line<'static>> {
    let row_width = panel_width.saturating_sub(2);
    let visible_rows = scroll_offset..scroll_offset + viewport_height;

    let pinned_file_idx = review_doc.index.file_at_row(scroll_offset).map(|(i, _)| i);
    let mut lines: Vec<Line<'static>> = if let Some(file_idx) = pinned_file_idx {
        let slot = &review_doc.files[file_idx];
        if matches!(slot.load, DiffLoadState::Binary) {
            vec![]
        } else {
            vec![render_file_header(row_width, &slot.entry, theme)]
        }
    } else {
        vec![]
    };

    lines.extend(
        visible_rows
            .flat_map(|global_row| match review_doc.lookup_row(global_row) {
                Some(RenderedRow::FileHeader { file_idx }) => {
                    if Some(file_idx) == pinned_file_idx {
                        vec![]
                    } else {
                        let entry = &review_doc.files[file_idx].entry;
                        vec![render_file_header(row_width, entry, theme)]
                    }
                }
                Some(RenderedRow::SectionHeader { kind, .. }) => {
                    vec![section_header_line(row_width, kind, theme)]
                }
                Some(RenderedRow::HunkHeader { file_idx, hunk_idx }) => {
                    let state = &review_doc.files[file_idx].load;
                    match state {
                        DiffLoadState::Loaded { hunks, .. } => {
                            let hunk = &hunks[hunk_idx];
                            let is_selected = selected_hunk == Some((file_idx, hunk_idx));
                            let header = hunk_header_line(
                                row_width,
                                hunk_idx,
                                hunks.len(),
                                hunk.header.trim_end(),
                                (hunk.insertions, hunk.deletions),
                                is_selected,
                                theme,
                            );
                            vec![header]
                        }
                        _ => vec![],
                    }
                }
                Some(RenderedRow::DiffLine {
                    file_idx,
                    hunk_idx,
                    line_idx,
                }) => {
                    let state = &review_doc.files[file_idx].load;
                    match state {
                        DiffLoadState::Loaded { hunks, .. } => {
                            let line = &hunks[hunk_idx].lines[line_idx];
                            let path = &review_doc.files[file_idx].entry.path;
                            let is_selected = selected_hunk == Some((file_idx, hunk_idx));
                            let ranges = inline_ranges(&hunks[hunk_idx], line_idx, line.origin);
                            let diff = diff_line(
                                row_width,
                                line,
                                Some(path),
                                &ranges,
                                is_selected,
                                show_line_numbers,
                                theme,
                            );
                            vec![diff]
                        }
                        _ => vec![],
                    }
                }
                Some(RenderedRow::Loading) => {
                    vec![Line::from(Span::styled(" Loading..", theme.muted()))]
                }
                Some(RenderedRow::Binary { file_idx }) => {
                    let entry = &review_doc.files[file_idx].entry;
                    vec![render_binary_header(row_width, entry, theme)]
                }
                Some(RenderedRow::TooLarge { lines, .. }) => vec![Line::from(Span::styled(
                    format!("  File too large ({lines} lines) — press Enter to load"),
                    theme.muted(),
                ))],
                Some(RenderedRow::Error { msg, .. }) => vec![Line::from(Span::styled(
                    format!(" Error: {msg}"),
                    theme.muted(),
                ))],
                None => vec![],
            })
            .collect::<Vec<_>>(),
    );

    lines
}
