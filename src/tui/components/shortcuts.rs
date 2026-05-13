use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
};

use crate::app::App;

pub fn draw(frame: &mut Frame, area: Rect, _app: &App) {
    let shortcuts = [("q", "quit"), ("tab", "next"), ("shift+tab", "previous")];

    let spans = shortcuts
        .into_iter()
        .flat_map(|(key, label)| {
            [
                Span::styled(format!(" {key} "), Style::default().fg(Color::White)),
                Span::styled(format!("{label} "), Style::default().fg(Color::White)),
            ]
        })
        .collect::<Vec<_>>();

    frame.render_widget(
        Paragraph::new(Line::from(spans))
            .style(Style::default().fg(Color::White))
            .block(Block::default().borders(Borders::TOP)),
        area,
    );
}
