use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
};

use crate::{
    app::App,
    file_finder,
    state::{FileSlot, Overlay},
    tui::{components::modal, layout, theme::Theme},
};

const MODAL_SIZE: modal::ModalSize = modal::ModalSize::new(
    modal::ResponsiveSize::new(90, 80).with_margin(2),
    modal::ResponsiveSize::new(85, 14).with_margin(1),
);

pub fn draw(frame: &mut Frame, area: Rect, app: &mut App) {
    if layout::terminal_is_too_small(area) {
        app.dismiss_overlay();
        return;
    }

    let (query, selected) = match app.overlay() {
        Overlay::FileFinder(state) => (state.query.clone(), state.selected),
        _ => return,
    };

    let theme = app.theme();
    let modal = modal::Modal::new(area, theme, modal::ModalConfig::new(MODAL_SIZE));
    modal.render(frame);

    let [input_area, list_area, footer_area] = Layout::vertical([
        Constraint::Length(4),
        Constraint::Min(0),
        Constraint::Length(2),
    ])
    .areas(modal.inner());

    draw_input(frame, input_area, theme, &query);
    draw_file_list(frame, list_area, app.files(), &query, selected, theme);
    draw_footer(frame, footer_area, theme);
}

fn draw_input(frame: &mut Frame, area: Rect, theme: Theme, input: &str) {
    let block = Block::default()
        .borders(Borders::BOTTOM)
        .border_style(theme.panel_border())
        .style(Style::default().bg(theme.bg));
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let [_, input_row, _] = Layout::vertical([
        Constraint::Length(1),
        Constraint::Length(1),
        Constraint::Min(0),
    ])
    .areas(inner);
    let [input_area, _, close_area] = Layout::horizontal([
        Constraint::Min(0),
        Constraint::Length(2),
        Constraint::Length(5),
    ])
    .areas(input_row);
    let [prompt_area, query_area] =
        Layout::horizontal([Constraint::Length(4), Constraint::Min(0)]).areas(input_area);
    let query = Line::from(vec![
        Span::raw(input.to_string()),
        Span::styled("| ", theme.accent()),
    ]);
    let horizontal_scroll = query.width().saturating_sub(usize::from(query_area.width));

    frame.render_widget(
        Paragraph::new(Span::styled("  › ", theme.accent())),
        prompt_area,
    );
    frame.render_widget(
        Paragraph::new(query).scroll((0, u16::try_from(horizontal_scroll).unwrap_or(u16::MAX))),
        query_area,
    );
    frame.render_widget(
        Paragraph::new(Span::styled("esc  ", theme.accent())).alignment(Alignment::Right),
        close_area,
    );
}

fn draw_file_list(
    frame: &mut Frame,
    area: Rect,
    files: &[FileSlot],
    query: &str,
    selected: usize,
    theme: Theme,
) {
    let matching_files = file_finder::matching_files(files, query);
    if matching_files.is_empty() {
        let item = ListItem::new(Span::styled("   no matches", theme.muted()));
        frame.render_widget(List::new(vec![item]), area);
        return;
    }

    let items = matching_files
        .iter()
        .map(|file_match| {
            let path = &files[file_match.file_index].entry.path;
            let line = match &file_match.range {
                Some(range) => Line::from(vec![
                    Span::raw("   "),
                    Span::raw(path[..range.start].to_string()),
                    Span::styled(
                        path[range.clone()].to_string(),
                        theme.accent().add_modifier(Modifier::BOLD),
                    ),
                    Span::raw(path[range.end..].to_string()),
                ]),
                None => Line::from(vec![Span::raw("   "), Span::raw(path.clone())]),
            };
            ListItem::new(line)
        })
        .collect::<Vec<_>>();
    let list = List::new(items).highlight_style(Style::default().bg(theme.select_hi));
    let selected = selected.min(matching_files.len() - 1);
    let mut list_state = ListState::default().with_selected(Some(selected));

    frame.render_stateful_widget(list, area, &mut list_state);
}

fn draw_footer(frame: &mut Frame, area: Rect, theme: Theme) {
    let block = Block::default()
        .borders(Borders::TOP)
        .border_style(theme.panel_border())
        .style(Style::default().bg(theme.bg));
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let commands = vec![
        Span::styled(" ↑↓ ", theme.accent()),
        Span::styled("select", theme.muted()),
        Span::raw("   "),
        Span::styled(" enter ", theme.accent()),
        Span::styled("jump", theme.muted()),
        Span::raw("   "),
        Span::styled(" ctrl+u ", theme.accent()),
        Span::styled("clear", theme.muted()),
    ];
    frame.render_widget(
        Paragraph::new(Line::from(commands)).alignment(Alignment::Center),
        inner,
    );
}
