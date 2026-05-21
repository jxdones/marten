use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders};
use ratatui::{Frame, layout::Rect};

use crate::app::App;

pub fn draw(frame: &mut Frame, area: Rect, app: &App, is_focused: bool) {
    let theme = app.theme();
    let border_style = if is_focused {
        theme.focused_border()
    } else {
        theme.panel_border()
    };
    let block = Block::default()
        .title(Line::from(vec![Span::styled(
            "[4] history",
            Style::default()
                .fg(theme.accent)
                .add_modifier(Modifier::BOLD),
        )]))
        .borders(Borders::ALL)
        .border_style(border_style);
    frame.render_widget(block, area);
}
