use ratatui::style::{Color, Style};
use ratatui::text::Line;
use ratatui::widgets::{Block, Borders};

use crate::tui::theme::Theme;

pub fn block(
    title: Option<Line<'static>>,
    theme: Theme,
    borders: Borders,
    bg: Color,
    is_focused: bool,
) -> Block<'static> {
    let border_style = if is_focused {
        theme.focused_border()
    } else {
        theme.panel_border()
    };

    let mut block = Block::default()
        .borders(borders)
        .border_style(border_style)
        .style(Style::default().bg(bg));
    if let Some(title) = title {
        block = block.title(title);
    }
    block
}
