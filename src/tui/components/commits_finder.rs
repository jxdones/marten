use std::time::{SystemTime, UNIX_EPOCH};

use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Layout, Rect},
    style::Style,
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
};

use crate::{
    app::App,
    commits_finder as commit_search,
    git::repository::CommitInfo,
    state::{CommitsFinderFocus, Overlay},
    tui::{components::modal, layout, theme::Theme},
};

const MODAL_SIZE: modal::ModalSize = modal::ModalSize::new(
    modal::ResponsiveSize::new(90, 80).with_margin(2),
    modal::ResponsiveSize::new(85, 13).with_margin(1),
);

pub fn draw(frame: &mut Frame, area: Rect, app: &mut App) {
    let (query, selected, focus) = match app.overlay() {
        Overlay::CommitsFinder(state) => (state.query.clone(), state.selected, state.focus),
        _ => return,
    };

    if layout::terminal_is_too_small(area) {
        app.dismiss_overlay();
        return;
    }

    let theme = app.theme();
    let modal = modal::Modal::new(area, theme, modal::ModalConfig::new(MODAL_SIZE));
    modal.render(frame);

    let [title_area, input_area, list_area, footer_area] = Layout::vertical([
        Constraint::Length(2),
        Constraint::Length(2),
        Constraint::Min(0),
        Constraint::Length(2),
    ])
    .areas(modal.inner());

    let matching_commits = commit_search::matching_commits(app.commits(), &query);
    modal::draw_title_bar(frame, title_area, "commits", theme);
    draw_input(
        frame,
        input_area,
        theme,
        &query,
        focus == CommitsFinderFocus::Search,
    );
    draw_commit_list(
        frame,
        list_area,
        app.commits(),
        &matching_commits,
        selected,
        theme,
    );
    draw_footer(frame, footer_area, theme, focus);
}

fn draw_input(frame: &mut Frame, area: Rect, theme: Theme, input: &str, focused: bool) {
    let block = Block::default()
        .borders(Borders::BOTTOM)
        .border_style(theme.panel_border())
        .style(Style::default().bg(theme.bg));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let line = if input.is_empty() && !focused {
        Line::from(vec![
            Span::styled("  / ", theme.muted()),
            Span::styled("search commits", theme.muted()),
        ])
    } else {
        let mut spans = vec![
            Span::styled(
                "  / ",
                if focused {
                    theme.accent()
                } else {
                    theme.muted()
                },
            ),
            Span::styled(input.to_string(), theme.fg),
        ];
        if focused {
            spans.push(Span::styled("|", theme.accent()));
        }
        Line::from(spans)
    };

    frame.render_widget(Paragraph::new(line), inner);
}

fn draw_commit_list(
    frame: &mut Frame,
    area: Rect,
    commits: &[CommitInfo],
    matching_commits: &[usize],
    selected: usize,
    theme: Theme,
) {
    const LEFT_PADDING: usize = 2;
    const HASH_WIDTH: usize = 7;
    const HASH_GAP: usize = 2;
    const AGE_GAP: usize = 2;
    const RIGHT_PADDING: usize = 2;

    let selected = selected.min(matching_commits.len().saturating_sub(1));
    let now = unix_timestamp();
    let ages = matching_commits
        .iter()
        .filter_map(|index| commits.get(*index))
        .map(|commit| relative_age(commit.committed_at, now))
        .collect::<Vec<_>>();
    let age_width = ages
        .iter()
        .map(|age| Line::raw(age).width())
        .max()
        .unwrap_or(0)
        .max(3);
    let fixed_width = LEFT_PADDING + HASH_WIDTH + HASH_GAP + AGE_GAP + age_width + RIGHT_PADDING;
    let subject_width = usize::from(area.width).saturating_sub(fixed_width);

    if matching_commits.is_empty() {
        let message = if commits.is_empty() {
            "  no commits"
        } else {
            "  no matching commits"
        };
        frame.render_widget(
            List::new(vec![ListItem::new(Span::styled(message, theme.muted()))]),
            area,
        );
        return;
    }

    let items = matching_commits
        .iter()
        .filter_map(|index| commits.get(*index))
        .zip(ages)
        .enumerate()
        .map(|(index, (commit, age))| {
            let short_oid = commit.oid.to_string().chars().take(7).collect::<String>();
            let oid_style = if index == selected {
                theme.accent()
            } else {
                theme.muted()
            };
            let line = Line::from(vec![
                Span::raw(" ".repeat(LEFT_PADDING)),
                Span::styled(short_oid, oid_style),
                Span::raw(" ".repeat(HASH_GAP)),
                Span::styled(
                    fit_to_width(&commit.subject, subject_width),
                    theme.text_primary(),
                ),
                Span::raw(" ".repeat(AGE_GAP)),
                Span::styled(format!("{age:>age_width$}"), theme.muted()),
                Span::raw(" ".repeat(RIGHT_PADDING)),
            ]);

            ListItem::new(line)
        })
        .collect::<Vec<_>>();

    let list = List::new(items).highlight_style(Style::default().bg(theme.select_hi).bold());
    let mut state = ListState::default().with_selected(Some(selected));
    frame.render_stateful_widget(list, area, &mut state);
}

fn unix_timestamp() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .ok()
        .and_then(|duration| i64::try_from(duration.as_secs()).ok())
        .unwrap_or_default()
}

fn relative_age(committed_at: i64, now: i64) -> String {
    let seconds = now.saturating_sub(committed_at);

    match seconds {
        0..60 => format!("{seconds}s"),
        60..3_600 => format!("{}m", seconds / 60),
        3_600..86_400 => format!("{}h", seconds / 3_600),
        86_400..2_592_000 => format!("{}d", seconds / 86_400),
        2_592_000..31_536_000 => format!("{}mo", seconds / 2_592_000),
        _ => format!("{}y", seconds / 31_536_000),
    }
}

fn fit_to_width(text: &str, width: usize) -> String {
    if width == 0 {
        return String::new();
    }

    let text_width = Line::raw(text).width();
    let content_width = if text_width > width {
        width.saturating_sub(1)
    } else {
        width
    };
    let mut output = String::new();

    for character in text.chars() {
        let candidate = format!("{output}{character}");
        if Line::raw(&candidate).width() > content_width {
            break;
        }
        output.push(character);
    }

    if text_width > width {
        output.push('…');
    }

    let padding = width.saturating_sub(Line::raw(&output).width());
    output.push_str(&" ".repeat(padding));
    output
}

fn draw_footer(frame: &mut Frame, area: Rect, theme: Theme, focus: CommitsFinderFocus) {
    let block = Block::default()
        .borders(Borders::TOP)
        .border_style(theme.panel_border())
        .style(Style::default().bg(theme.bg));
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let commands = Line::from(vec![
        Span::styled(" ↑↓/jk ", theme.accent()),
        Span::styled("navigate", theme.muted()),
        Span::raw("   "),
        Span::styled(" enter ", theme.accent()),
        Span::styled("open diff", theme.muted()),
        Span::raw("   "),
        Span::styled(" ctrl+u ", theme.accent()),
        Span::styled("clear", theme.muted()),
        Span::raw("   "),
        Span::styled(" r ", theme.accent()),
        Span::styled("reset", theme.muted()),
        Span::raw("   "),
        Span::styled(" / ", theme.accent()),
        Span::styled(
            if focus == CommitsFinderFocus::Search {
                "list"
            } else {
                "search"
            },
            theme.muted(),
        ),
    ]);

    frame.render_widget(Paragraph::new(commands).alignment(Alignment::Center), inner)
}
