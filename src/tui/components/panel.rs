use ratatui::{
    style::{Color, Style},
    widgets::{Block, Borders},
};

pub fn block(title: &'static str, is_focused: bool) -> Block<'static> {
    let border_style = if is_focused {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default().fg(Color::DarkGray)
    };

    Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(border_style)
}
