use ratatui::widgets::{Block, Borders};

use crate::tui::theme::Theme;

pub fn block(title: &'static str, theme: Theme, is_focused: bool) -> Block<'static> {
    let border_style = if is_focused {
        theme.focused_border()
    } else {
        theme.panel_border()
    };

    Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(border_style)
}
