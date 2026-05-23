use ratatui::layout::{Constraint, Direction, Layout, Rect};

#[derive(Debug, Clone)]
pub struct Home {
    pub top_bar: Rect,
    pub left_sidebar: Rect,
    pub diff: Rect,
    pub shortcuts: Rect,
}

pub fn home(area: Rect) -> Home {
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Min(0),
            Constraint::Length(2),
        ])
        .split(area);

    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(25), Constraint::Percentage(75)])
        .split(rows[1]);

    Home {
        top_bar: rows[0],
        left_sidebar: cols[0],
        diff: cols[1],
        shortcuts: rows[2],
    }
}
