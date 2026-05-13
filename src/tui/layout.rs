use ratatui::layout::{Constraint, Direction, Layout, Rect};

#[derive(Debug, Clone)]
pub struct Home {
    pub top_bar: Rect,
    pub left_sidebar: Rect,
    pub diff: Rect,
    pub history: Rect,
    pub right_sidebar: Rect,
    pub shortcuts: Rect,
}

pub fn home(area: Rect) -> Home {
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(0),
            Constraint::Length(3),
        ])
        .split(area);

    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(22),
            Constraint::Percentage(56),
            Constraint::Percentage(22),
        ])
        .split(rows[1]);

    let center_rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(65), Constraint::Percentage(35)])
        .split(cols[1]);

    Home {
        top_bar: rows[0],
        left_sidebar: cols[0],
        diff: center_rows[0],
        history: center_rows[1],
        right_sidebar: cols[2],
        shortcuts: rows[2],
    }
}
