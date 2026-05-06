use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
};

pub fn draw_base_layout(frame: &mut Frame, area: Rect) {
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(0),
            Constraint::Length(3),
        ])
        .split(area);

    frame.render_widget(
        Block::default().title("top bar").borders(Borders::BOTTOM),
        rows[0],
    );

    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(22),
            Constraint::Percentage(56),
            Constraint::Percentage(22),
        ])
        .split(rows[1]);
    // render left sidebar
    frame.render_widget(
        Block::default().title("left sidebar").borders(Borders::ALL),
        cols[0],
    );

    // render diff and history
    let center_rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(65), Constraint::Percentage(35)])
        .split(cols[1]);

    frame.render_widget(
        Block::default().title("diff").borders(Borders::ALL),
        center_rows[0],
    );

    frame.render_widget(
        Block::default().title("history").borders(Borders::ALL),
        center_rows[1],
    );

    // render right
    frame.render_widget(
        Block::default()
            .title("right sidebar")
            .borders(Borders::ALL),
        cols[2],
    );

    let shortcuts = vec![("q: quit")];
    let mut spans = Vec::new();
    for shortcut in shortcuts {
        spans.push(Span::styled(shortcut, Style::default().fg(Color::White)));
    }
    frame.render_widget(
        Paragraph::new(Line::from(spans))
            .style(Style::default().fg(Color::White))
            .block(Block::default().borders(Borders::TOP)),
        rows[2],
    );
}
