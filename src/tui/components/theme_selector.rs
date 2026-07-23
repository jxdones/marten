use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
};

use crate::{
    app::App,
    state::Overlay,
    tui::{
        components::modal,
        layout,
        theme::{THEMES, Theme, ThemeEntry},
    },
};

const MODAL_SIZE: modal::ModalSize = modal::ModalSize::new(
    modal::ResponsiveSize::new(90, 54).with_margin(2),
    modal::ResponsiveSize::new(80, 15).with_margin(1),
);

pub fn draw(frame: &mut Frame, area: Rect, app: &mut App) {
    let (selected, original) = match app.overlay() {
        Overlay::ThemeSelector(state) => (state.selected, state.original),
        Overlay::None => return,
        Overlay::CommandPalette(_) => return,
    };

    if layout::terminal_is_too_small(area) {
        app.dismiss_overlay();
        return;
    }

    let active_theme = app.theme();
    let modal = modal::Modal::new(area, active_theme, modal::ModalConfig::new(MODAL_SIZE));
    modal.render(frame);

    let [title_area, header_area, list_area, footer_area] = Layout::vertical([
        Constraint::Length(2),
        Constraint::Length(2),
        Constraint::Min(1),
        Constraint::Length(2),
    ])
    .areas(modal.inner());

    modal::draw_title_bar(frame, title_area, "theme picker", active_theme);
    draw_header(frame, header_area, active_theme);
    draw_list(frame, list_area, selected, original, active_theme);
    draw_footer(frame, footer_area, active_theme);
}

fn draw_header(frame: &mut Frame, area: Rect, theme: Theme) {
    let header = Line::from(vec![
        Span::styled("  Theme                ", theme.muted()),
        Span::styled("Palette         ", theme.muted()),
        Span::styled("Mode", theme.muted()),
    ]);

    frame.render_widget(Paragraph::new(header), area);
}

fn draw_list(frame: &mut Frame, area: Rect, selected: usize, original: usize, active_theme: Theme) {
    let rows = THEMES
        .iter()
        .enumerate()
        .map(|(index, entry)| theme_row(entry, index == selected, index == original, active_theme))
        .collect::<Vec<_>>();

    let list = List::new(rows)
        .style(Style::default().fg(active_theme.fg).bg(active_theme.bg))
        .highlight_style(Style::default());

    let mut state = ListState::default().with_selected(Some(selected));
    frame.render_stateful_widget(list, area, &mut state);
}

fn theme_row(
    entry: &ThemeEntry,
    is_selected: bool,
    is_original: bool,
    active_theme: Theme,
) -> ListItem<'static> {
    let row_style = if is_selected {
        Style::default()
            .fg(active_theme.fg)
            .bg(active_theme.select_hi)
    } else {
        Style::default().fg(active_theme.fg).bg(active_theme.bg)
    };

    let label_style = if is_selected {
        row_style.add_modifier(Modifier::BOLD)
    } else {
        row_style
    };

    let marker = if is_selected { "› " } else { "  " };
    let appearance = match entry.appearance {
        "dark" => "Dark",
        "light" => "Light",
        other => other,
    };
    let current = if is_original { " ●" } else { "" };
    let sample = entry.theme;

    let line = Line::from(vec![
        Span::styled(marker, row_style),
        Span::styled(fit_to_width(entry.name, 20), label_style),
        Span::styled(" ", row_style),
        Span::styled(
            " Aa ",
            Style::default()
                .fg(sample.fg)
                .bg(sample.bg)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(" ", row_style),
        Span::styled(" + ", sample.diff_add()),
        Span::styled(" ", row_style),
        Span::styled(" - ", sample.diff_del()),
        Span::styled("    ", row_style),
        Span::styled(fit_to_width(appearance, 8), row_style),
        Span::styled(current, row_style.fg(active_theme.accent)),
    ]);

    ListItem::new(line).style(row_style)
}

fn fit_to_width(text: &str, width: usize) -> String {
    let mut output = text.chars().take(width).collect::<String>();
    let used = Line::raw(&output).width();

    output.push_str(&" ".repeat(width.saturating_sub(used)));
    output
}

fn draw_footer(frame: &mut Frame, area: Rect, theme: Theme) {
    let block = Block::default()
        .borders(Borders::TOP)
        .border_style(theme.panel_border())
        .style(Style::default().bg(theme.bg));
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let left = vec![
        Span::styled(" ↓↑/jk ", theme.accent()),
        Span::styled("preview ", theme.muted()),
        Span::styled("  enter ", theme.accent()),
        Span::styled("save", theme.muted()),
    ];
    frame.render_widget(
        Paragraph::new(Line::from(left)).alignment(Alignment::Center),
        inner,
    );
}
