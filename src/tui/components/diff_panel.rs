use ratatui::layout::{Alignment, Constraint, Layout};
use ratatui::style::{Modifier, Style, Stylize};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::{Frame, layout::Rect};
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

use crate::app::App;
use crate::git::repository::{DiffHunk, DiffLine, DiffSectionKind, FileEntry, FileStatus};
use crate::inline_diff::{self, Range};
use crate::state::review::RenderedRow;
use crate::state::{ContinuousDiff, DiffLayout, DiffLoadState};
use crate::syntax;
use crate::tui::theme::Theme;

#[derive(Clone, Copy)]
struct HunkPosition {
    file_idx: Option<usize>,
    selected_hunk: usize,
    hunk_count: usize,
    selected_line: usize,
    line_count: usize,
}

impl HunkPosition {
    fn active(self) -> Option<Self> {
        (self.hunk_count > 0).then_some(self)
    }

    // Position for a file in continuous mode
    fn for_file(self, file_idx: usize) -> Option<Self> {
        match self.file_idx {
            Some(idx) if idx != file_idx => None,
            _ => self.active(),
        }
    }
}

fn hunk_position(app: &App) -> HunkPosition {
    let continuous_diff = app.continuous_diff();
    let layout = continuous_diff.layout;
    let scroll = app.review_state().continuous_scroll;
    match continuous_diff.lookup_row(scroll) {
        Some(RenderedRow::HunkHeader { file_idx, hunk_idx }) => {
            let (total, line_count) = match &continuous_diff.files[file_idx].load {
                DiffLoadState::Loaded { hunks, .. } => (
                    hunks.len(),
                    match layout {
                        DiffLayout::Unified => hunks[hunk_idx].lines.len(),
                        DiffLayout::SideBySide => hunks[hunk_idx].comparison_rows.len(),
                    },
                ),
                _ => (0, 0),
            };
            HunkPosition {
                file_idx: Some(file_idx),
                selected_hunk: hunk_idx + 1,
                hunk_count: total,
                selected_line: 0,
                line_count,
            }
        }
        Some(RenderedRow::DiffRow {
            file_idx,
            hunk_idx,
            row_idx,
        }) => {
            let (total, line_count) = match &continuous_diff.files[file_idx].load {
                DiffLoadState::Loaded { hunks, .. } => (
                    hunks.len(),
                    match layout {
                        DiffLayout::Unified => hunks[hunk_idx].lines.len(),
                        DiffLayout::SideBySide => hunks[hunk_idx].comparison_rows.len(),
                    },
                ),
                _ => (0, 0),
            };
            HunkPosition {
                file_idx: Some(file_idx),
                selected_hunk: hunk_idx + 1,
                hunk_count: total,
                selected_line: row_idx,
                line_count,
            }
        }
        _ => HunkPosition {
            file_idx: None,
            selected_hunk: 0,
            hunk_count: 0,
            selected_line: 0,
            line_count: 0,
        },
    }
}

pub fn draw(frame: &mut Frame, area: Rect, app: &App, is_focused: bool, has_sidebar: bool) {
    let theme = app.theme();
    let borders = if has_sidebar {
        Borders::LEFT
    } else {
        Borders::NONE
    };
    let block = Block::default()
        .borders(borders)
        .border_style(theme.panel_border())
        .style(Style::default().bg(theme.bg));

    let viewport_height = area.height as usize;
    let border_width = usize::from(has_sidebar);
    let panel_width = (area.width as usize).saturating_sub(border_width);
    let position = hunk_position(app);

    let scroll = app.review_state().continuous_scroll;
    let continuous_diff = app.continuous_diff();
    let selected_hunk = match continuous_diff.lookup_row(scroll) {
        Some(RenderedRow::HunkHeader { file_idx, hunk_idx }) => Some((file_idx, hunk_idx)),
        Some(RenderedRow::DiffRow {
            file_idx, hunk_idx, ..
        }) => Some((file_idx, hunk_idx)),
        _ => None,
    };
    let lines = render_continuous_diff(
        scroll,
        viewport_height,
        continuous_diff,
        selected_hunk,
        app.diff_state().show_line_numbers,
        app.diff_state().horizontal_scroll,
        theme,
        panel_width,
        position,
        is_focused,
    );

    if lines.is_empty() {
        let inner = block.inner(area);
        frame.render_widget(block, area);
        draw_empty_diff(frame, inner, theme);
        return;
    }

    let paragraph = Paragraph::new(Text::from(lines)).block(block);
    frame.render_widget(paragraph, area);
}

#[allow(clippy::too_many_arguments)]
fn render_continuous_diff(
    scroll_offset: usize,
    viewport_height: usize,
    continuous_diff: &ContinuousDiff,
    selected_hunk: Option<(usize, usize)>,
    show_line_numbers: bool,
    horizontal_scroll: usize,
    theme: Theme,
    panel_width: usize,
    position: HunkPosition,
    is_focused: bool,
) -> Vec<Line<'static>> {
    let row_width = panel_width;
    let visible_rows = scroll_offset..scroll_offset + viewport_height;

    let pinned_file_idx = continuous_diff
        .index
        .file_at_row(scroll_offset)
        .map(|(i, _)| i);
    let mut lines: Vec<Line<'static>> = if let Some(file_idx) = pinned_file_idx {
        let slot = &continuous_diff.files[file_idx];
        if matches!(slot.load, DiffLoadState::Binary) {
            vec![]
        } else {
            vec![render_file_header(
                row_width,
                &slot.entry,
                theme,
                position.for_file(file_idx),
                is_focused,
            )]
        }
    } else {
        vec![]
    };

    lines.extend(
        visible_rows
            .flat_map(|global_row| match continuous_diff.lookup_row(global_row) {
                Some(RenderedRow::FileHeader { file_idx }) => {
                    if Some(file_idx) == pinned_file_idx {
                        vec![]
                    } else {
                        let entry = &continuous_diff.files[file_idx].entry;
                        vec![render_file_header(
                            row_width,
                            entry,
                            theme,
                            position.for_file(file_idx),
                            is_focused,
                        )]
                    }
                }
                Some(RenderedRow::SectionHeader { kind, .. }) => {
                    vec![section_header_line(row_width, kind, theme)]
                }
                Some(RenderedRow::HunkHeader { file_idx, hunk_idx }) => {
                    let state = &continuous_diff.files[file_idx].load;
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
                Some(RenderedRow::DiffRow {
                    file_idx,
                    hunk_idx,
                    row_idx,
                }) => {
                    let state = &continuous_diff.files[file_idx].load;
                    match state {
                        DiffLoadState::Loaded { hunks, .. } => {
                            let hunk = &hunks[hunk_idx];
                            let path = &continuous_diff.files[file_idx].entry.path;
                            let is_selected = selected_hunk == Some((file_idx, hunk_idx));
                            let diff = match continuous_diff.layout {
                                DiffLayout::Unified => {
                                    let line = &hunk.lines[row_idx];
                                    let ranges = inline_ranges(hunk, row_idx);
                                    diff_line(
                                        row_width,
                                        line,
                                        path,
                                        &ranges,
                                        is_selected,
                                        show_line_numbers,
                                        horizontal_scroll,
                                        theme,
                                    )
                                }
                                DiffLayout::SideBySide => side_by_side_diff_line(
                                    row_width,
                                    hunk,
                                    row_idx,
                                    path,
                                    is_selected,
                                    show_line_numbers,
                                    horizontal_scroll,
                                    theme,
                                ),
                            };
                            vec![diff]
                        }
                        _ => vec![],
                    }
                }
                Some(RenderedRow::Loading) => {
                    vec![Line::from(Span::styled(" Loading..", theme.muted()))]
                }
                Some(RenderedRow::Binary { file_idx }) => {
                    let entry = &continuous_diff.files[file_idx].entry;
                    vec![render_binary_header(row_width, entry, theme)]
                }
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

fn render_file_header(
    width: usize,
    file: &FileEntry,
    theme: Theme,
    position: Option<HunkPosition>,
    is_focused: bool,
) -> Line<'static> {
    let collapse_symbol = "▼ ";
    let bg = Style::default().bg(theme.file_header_bg);
    let mut spans = vec![Span::styled(collapse_symbol, theme.muted().patch(bg))];

    let status_color = file_status_color(file.status, theme);
    let status_symbol = file_status_symbol(file.status);
    let path = format!(" {}", file.path);
    let position_spans = position_spans(position, theme, bg, is_focused);
    let position_width: usize = position_spans
        .iter()
        .map(|span| text_width(&span.content))
        .sum();
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
            + position_width
            + text_width(&stats),
    );

    let mut path_style = if is_focused {
        theme.accent().patch(bg)
    } else {
        theme.muted().patch(bg)
    };
    if position.is_some() {
        path_style = path_style.add_modifier(Modifier::BOLD);
    }

    spans.push(Span::styled(status_symbol, status_color.patch(bg)));
    spans.push(Span::styled(path, path_style));
    spans.extend(position_spans);
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

fn position_spans(
    position: Option<HunkPosition>,
    theme: Theme,
    bg: Style,
    is_focused: bool,
) -> Vec<Span<'static>> {
    let Some(position) = position else {
        return Vec::new();
    };

    let bold = if is_focused {
        Modifier::BOLD
    } else {
        Modifier::empty()
    };
    let muted = theme.muted().patch(bg).add_modifier(bold);
    let label = if is_focused {
        theme.accent().patch(bg).add_modifier(bold)
    } else {
        muted
    };

    let mut spans = counter_spans(
        "hunk ",
        position.selected_hunk,
        position.hunk_count,
        label,
        muted,
    );
    if position.line_count > 0 {
        spans.extend(counter_spans(
            "line ",
            position.selected_line + 1,
            position.line_count,
            label,
            muted,
        ));
    }
    spans
}

/// Renders " · {label}{current}/{total}", e.g. " · hunk 1/3".
fn counter_spans(
    label: &'static str,
    current: usize,
    total: usize,
    label_style: Style,
    muted_style: Style,
) -> Vec<Span<'static>> {
    vec![
        Span::styled(" · ", muted_style),
        Span::styled(label, label_style),
        Span::styled(current.to_string(), label_style),
        Span::styled("/", muted_style),
        Span::styled(total.to_string(), muted_style),
    ]
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

#[allow(clippy::too_many_arguments)]
fn diff_line(
    width: usize,
    line: &DiffLine,
    path: &str,
    inline_ranges: &[Range],
    is_selected: bool,
    show_line_numbers: bool,
    horizontal_scroll: usize,
    theme: Theme,
) -> Line<'static> {
    let base = match line.origin {
        '+' => theme.diff_add(),
        '-' => theme.diff_del(),
        _ => theme.muted().bg(theme.bg),
    };
    let content = display_content(&line.content);
    let prefix = if show_line_numbers {
        let old_lineno = line_number(line.old_lineno);
        let new_lineno = line_number(line.new_lineno);
        format!("{old_lineno} {new_lineno} {} ", line.origin)
    } else {
        format!("{} ", line.origin)
    };

    let highlighted = syntax::highlight_line(path, &content, base, theme.syntax_theme)
        .unwrap_or_else(|| vec![Span::styled(content, base)]);
    let content_spans = style_content_spans(highlighted, inline_ranges, line.origin, false, theme);
    let mut spans = vec![Span::styled(prefix, base)];
    spans.extend(skip_spans(content_spans, horizontal_scroll));

    bordered_line(width, spans, base, border_style(line, is_selected, theme))
}

#[allow(clippy::too_many_arguments)]
fn side_by_side_diff_line(
    width: usize,
    hunk: &DiffHunk,
    row_idx: usize,
    path: &str,
    is_selected: bool,
    show_line_numbers: bool,
    horizontal_scroll: usize,
    theme: Theme,
) -> Line<'static> {
    let row = &hunk.comparison_rows[row_idx];
    let old_line = row.old_line_idx.map(|idx| &hunk.lines[idx]);
    let new_line = row.new_line_idx.map(|idx| &hunk.lines[idx]);
    let (old_ranges, new_ranges) = comparison_inline_ranges(old_line, new_line);

    let left_width = width / 2;
    let right_width = width.saturating_sub(left_width);
    let mut spans = vec![change_gutter(
        old_line,
        '-',
        is_selected,
        theme.del_gutter,
        " ",
        Style::default().bg(theme.bg),
    )];
    spans.extend(comparison_side_spans(
        old_line,
        true,
        path,
        &old_ranges,
        show_line_numbers,
        horizontal_scroll,
        left_width.saturating_sub(1),
        theme,
    ));
    spans.push(change_gutter(
        new_line,
        '+',
        is_selected,
        theme.add_gutter,
        "│",
        Style::default().fg(theme.hunk_header_bg),
    ));
    spans.extend(comparison_side_spans(
        new_line,
        false,
        path,
        &new_ranges,
        show_line_numbers,
        horizontal_scroll,
        right_width.saturating_sub(1),
        theme,
    ));

    Line::from(spans)
}

fn change_gutter(
    line: Option<&DiffLine>,
    origin: char,
    is_selected: bool,
    color: ratatui::style::Color,
    fallback: &'static str,
    fallback_style: Style,
) -> Span<'static> {
    if is_selected && line.is_some_and(|line| line.origin == origin) {
        Span::styled("▌", Style::default().fg(color))
    } else {
        Span::styled(fallback, fallback_style)
    }
}

#[allow(clippy::too_many_arguments)]
fn comparison_side_spans(
    line: Option<&DiffLine>,
    old_side: bool,
    path: &str,
    inline_ranges: &[Range],
    show_line_numbers: bool,
    horizontal_scroll: usize,
    width: usize,
    theme: Theme,
) -> Vec<Span<'static>> {
    let Some(line) = line else {
        return alignment_gap_spans(width, theme);
    };
    let style = match line.origin {
        '+' => theme.diff_add(),
        '-' => theme.diff_del(),
        _ => theme.muted().bg(theme.bg),
    };
    let number = if old_side {
        line.old_lineno
    } else {
        line.new_lineno
    };
    let prefix = if show_line_numbers {
        format!("{} {} ", line_number(number), line.origin)
    } else {
        format!("{} ", line.origin)
    };
    let content = display_content(&line.content);
    let highlighted = syntax::highlight_line(path, &content, style, theme.syntax_theme)
        .unwrap_or_else(|| vec![Span::styled(content, style)]);
    let content_spans = style_content_spans(highlighted, inline_ranges, line.origin, false, theme);
    let mut spans = vec![Span::styled(prefix, style)];
    spans.extend(skip_spans(content_spans, horizontal_scroll));
    fit_spans(spans, width, style)
}

fn alignment_gap_spans(width: usize, theme: Theme) -> Vec<Span<'static>> {
    vec![Span::styled(
        "╱".repeat(width),
        Style::default().fg(theme.line).bg(theme.bg),
    )]
}

fn skip_spans(spans: Vec<Span<'static>>, mut amount: usize) -> Vec<Span<'static>> {
    let mut scrolled = Vec::new();

    for span in spans {
        let span_width = text_width(&span.content);
        if amount >= span_width {
            amount -= span_width;
            continue;
        }

        let mut content = String::new();
        for ch in span.content.chars() {
            let width = ch.width().unwrap_or(0);
            if amount > 0 {
                if amount >= width {
                    amount -= width;
                } else {
                    amount = 0;
                }
                continue;
            }
            content.push(ch);
        }
        if !content.is_empty() {
            scrolled.push(Span::styled(content, span.style));
        }
    }

    scrolled
}

fn comparison_inline_ranges(
    old_line: Option<&DiffLine>,
    new_line: Option<&DiffLine>,
) -> (Vec<Range>, Vec<Range>) {
    match (old_line, new_line) {
        (Some(old_line), Some(new_line)) if old_line.origin == '-' && new_line.origin == '+' => {
            inline_diff::changed_ranges(
                &display_content(&old_line.content),
                &display_content(&new_line.content),
            )
        }
        _ => (Vec::new(), Vec::new()),
    }
}

fn fit_spans(spans: Vec<Span<'static>>, width: usize, fill_style: Style) -> Vec<Span<'static>> {
    let mut fitted = Vec::new();
    let mut remaining = width;

    for span in spans {
        if remaining == 0 {
            break;
        }
        let mut content = String::new();
        let mut used = 0;
        for ch in span.content.chars() {
            let width = ch.width().unwrap_or(0);
            if used + width > remaining {
                break;
            }
            content.push(ch);
            used += width;
        }
        if used > 0 {
            fitted.push(Span::styled(content, span.style));
            remaining = remaining.saturating_sub(used);
        }
    }

    if remaining > 0 {
        fitted.push(Span::styled(" ".repeat(remaining), fill_style));
    }
    fitted
}

fn inline_ranges(hunk: &DiffHunk, line_idx: usize) -> Vec<Range> {
    if !matches!(hunk.lines[line_idx].origin, '+' | '-') {
        return Vec::new();
    }

    let Some((old_line_idx, new_line_idx)) = hunk.comparison_rows.iter().find_map(|row| {
        let (Some(old_line_idx), Some(new_line_idx)) = (row.old_line_idx, row.new_line_idx) else {
            return None;
        };

        if (old_line_idx == line_idx || new_line_idx == line_idx)
            && hunk.lines[old_line_idx].origin == '-'
            && hunk.lines[new_line_idx].origin == '+'
        {
            Some((old_line_idx, new_line_idx))
        } else {
            None
        }
    }) else {
        return Vec::new();
    };

    let (old_ranges, new_ranges) = inline_diff::changed_ranges(
        &display_content(&hunk.lines[old_line_idx].content),
        &display_content(&hunk.lines[new_line_idx].content),
    );
    if line_idx == old_line_idx {
        old_ranges
    } else {
        new_ranges
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

fn text_width(text: &str) -> usize {
    UnicodeWidthStr::width(text)
}

fn display_content(content: &str) -> String {
    content.trim_end().replace('\t', "    ")
}

fn draw_empty_diff(frame: &mut Frame, area: Rect, theme: Theme) {
    let lines = vec![
        Line::from(Span::styled("No changes yet", theme.muted())),
        Line::from(Span::styled(
            "Edit files in this repository to start a diff.",
            theme.muted(),
        )),
    ];

    let lines_max_width = lines.iter().map(|l| l.width()).max().unwrap_or(0) as u16;
    let widget = Paragraph::new(lines).alignment(Alignment::Center);

    let vertical = Layout::vertical([
        Constraint::Fill(1),
        Constraint::Length(2),
        Constraint::Fill(1),
    ])
    .split(area);

    let horizontal = Layout::horizontal([
        Constraint::Fill(1),
        Constraint::Length(lines_max_width),
        Constraint::Fill(1),
    ])
    .split(vertical[1]);

    frame.render_widget(widget, horizontal[1]);
}
