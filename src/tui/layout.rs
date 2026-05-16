use ratatui::layout::{Constraint, Direction, Layout, Rect};

#[derive(Debug, Clone)]
pub struct Home {
    pub top_bar: Rect,
    pub left_sidebar: Rect,
    pub diff: Rect,
    pub history: Rect,
    pub shortcuts: Rect,
}

#[derive(Debug, Clone)]
pub struct LeftSidebar {
    pub files: Rect,
    pub branches: Rect,
    pub stash: Rect,
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

    let center_rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(65), Constraint::Percentage(35)])
        .split(cols[1]);

    Home {
        top_bar: rows[0],
        left_sidebar: cols[0],
        diff: center_rows[0],
        history: center_rows[1],
        shortcuts: rows[2],
    }
}

pub fn left_sidebar(area: Rect) -> LeftSidebar {
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage(34),
            Constraint::Percentage(33),
            Constraint::Percentage(33),
        ])
        .split(area);

    LeftSidebar {
        files: rows[0],
        branches: rows[1],
        stash: rows[2],
    }
}
