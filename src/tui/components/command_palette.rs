use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
};

use crate::{
    app::App,
    command_palette::{self, CommandItem},
    state::Overlay,
    tui::{components::modal, layout, theme::Theme},
};

const MODAL_SIZE: modal::ModalSize = modal::ModalSize::new(
    modal::ResponsiveSize::new(90, 80).with_margin(2),
    modal::ResponsiveSize::new(85, 22).with_margin(1),
);

pub fn draw(frame: &mut Frame, area: Rect, app: &mut App) {
    let selected = match app.overlay() {
        Overlay::CommandPalette(state) => state.selected,
        Overlay::None => return,
        _ => return,
    };
    if layout::terminal_is_too_small(area) {
        app.dismiss_overlay();
        return;
    }

    let theme = app.theme();
    let modal = modal::Modal::new(area, theme, modal::ModalConfig::new(MODAL_SIZE));
    modal.render(frame);

    let [title_area, list_area, footer_area] = Layout::vertical([
        Constraint::Length(2),
        Constraint::Min(0),
        Constraint::Length(2),
    ])
    .areas(modal.inner());

    modal::draw_title_bar(frame, title_area, "command palette", theme);
    draw_list(frame, list_area, selected, theme);
    draw_footer(frame, footer_area, theme);
}

fn draw_list(frame: &mut Frame, area: Rect, selected: usize, theme: Theme) {
    let groups = command_palette::command_groups();
    let key_width = groups
        .iter()
        .flat_map(|group| group.items)
        .map(|item| Line::raw(item.keybind).width())
        .max()
        .unwrap_or(0);

    let mut rows = Vec::new();
    let mut command_index = 0;
    let mut selected_row = None;

    for group in groups {
        rows.push(ListItem::new(Line::from(Span::styled(
            format!("  {}", group.section.label()),
            theme.muted().add_modifier(Modifier::BOLD),
        ))));

        for item in group.items {
            if command_index == selected {
                selected_row = Some(rows.len());
            }
            rows.push(ListItem::new(command_line(
                item,
                area.width,
                key_width,
                command_index == selected,
                theme,
            )));
            command_index += 1;
        }
    }

    let list = List::new(rows)
        .style(Style::default().fg(theme.fg).bg(theme.bg))
        .highlight_style(Style::default().bg(theme.select_hi));
    let mut state = ListState::default().with_selected(selected_row);

    frame.render_stateful_widget(list, area, &mut state);
}

fn command_line(
    item: &CommandItem,
    row_width: u16,
    key_width: usize,
    is_selected: bool,
    theme: Theme,
) -> Line<'static> {
    const LEADING_WIDTH: usize = 2;
    const TRAILING_WIDTH: usize = 1;

    let row_width = usize::from(row_width);
    let keycap_width = key_width.saturating_add(2);
    let available = row_width.saturating_sub(LEADING_WIDTH + TRAILING_WIDTH + keycap_width);
    let label_width = (row_width * 28 / 100).clamp(16, 26).min(available);
    let description_width = available.saturating_sub(label_width);
    let label_style = if is_selected {
        theme.text_primary().add_modifier(Modifier::BOLD)
    } else {
        theme.text_primary()
    };

    Line::from(vec![
        Span::raw(" ".repeat(LEADING_WIDTH.min(row_width))),
        Span::styled(fit_to_width(item.label, label_width), label_style),
        Span::styled(
            fit_to_width(item.description, description_width),
            theme.muted(),
        ),
        Span::styled(
            format!(" {:>width$} ", item.keybind, width = key_width),
            Style::default().fg(theme.fg).bg(theme.file_header_bg),
        ),
        Span::raw(" ".repeat(TRAILING_WIDTH)),
    ])
}

fn fit_to_width(text: &str, width: usize) -> String {
    if width == 0 {
        return String::new();
    }

    let mut fitted = text.to_string();
    if Line::raw(&fitted).width() > width {
        fitted.clear();
        let content_width = width.saturating_sub(1);

        for character in text.chars() {
            let candidate = format!("{fitted}{character}");
            if Line::raw(&candidate).width() > content_width {
                break;
            }
            fitted.push(character);
        }
        fitted.push('…');
    }

    let padding = width.saturating_sub(Line::raw(&fitted).width());
    fitted.push_str(&" ".repeat(padding));
    fitted
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
        Span::styled("select ", theme.muted()),
        Span::styled("  enter ", theme.accent()),
        Span::styled("run", theme.muted()),
    ];
    frame.render_widget(
        Paragraph::new(Line::from(left)).alignment(Alignment::Center),
        inner,
    );
}
