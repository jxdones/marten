use ratatui::{
    Frame,
    layout::Rect,
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
};

use crate::app::App;

pub fn draw(frame: &mut Frame, area: Rect, app: &App) {
    let shortcuts = [("q", "quit"), ("tab", "next"), ("shift+tab", "previous")];
    let theme = app.theme();

    let spans = shortcuts
        .into_iter()
        .flat_map(|(key, label)| {
            [
                Span::styled(format!(" {key} "), theme.muted()),
                Span::styled(format!("{label} "), theme.muted()),
            ]
        })
        .collect::<Vec<_>>();

    frame.render_widget(
        Paragraph::new(Line::from(spans))
            .style(theme.panel_border())
            .block(Block::default().borders(Borders::TOP)),
        area,
    );
}
