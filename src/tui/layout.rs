use ratatui::layout::{Constraint, Direction, Layout, Rect};

pub const MIN_TERMINAL_WIDTH: u16 = 80;
pub const MIN_TERMINAL_HEIGHT: u16 = 24;

pub const SIDEBAR_NARROW_MAX: u16 = 120;

#[derive(Debug, Clone)]
pub struct Home {
    pub top_bar: Rect,
    pub left_sidebar: Rect,
    pub diff: Rect,
    pub shortcuts: Rect,
}

pub fn home(area: Rect, has_sidebar: bool) -> Home {
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(2),
            Constraint::Min(0),
            Constraint::Length(2),
        ])
        .split(area);

    let sidebar_percent = if has_sidebar {
        sidebar_percentage(area.width)
    } else {
        0
    };

    let sidebar_width = Constraint::Percentage(sidebar_percent);
    let diff_width = Constraint::Percentage(100 - sidebar_percent);

    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([sidebar_width, diff_width])
        .split(rows[1]);

    Home {
        top_bar: rows[0],
        left_sidebar: cols[0],
        diff: cols[1],
        shortcuts: rows[2],
    }
}

pub const fn terminal_is_too_small(area: Rect) -> bool {
    area.width < MIN_TERMINAL_WIDTH || area.height < MIN_TERMINAL_HEIGHT
}

fn sidebar_percentage(sidebar_width: u16) -> u16 {
    match sidebar_width {
        width if width < MIN_TERMINAL_WIDTH => 0,
        width if width <= SIDEBAR_NARROW_MAX => 25,
        _ => 15,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sidebar_hidden_below_min_width_even_when_requested() {
        let area = Rect::new(0, 0, MIN_TERMINAL_WIDTH - 1, 24);

        let result = home(area, true);
        assert_eq!(result.left_sidebar.width, 0);
    }

    #[test]
    fn sidebar_shown_at_min_width_when_requested() {
        let area = Rect::new(0, 0, MIN_TERMINAL_WIDTH, 24);
        let result = home(area, true);
        assert_eq!(result.left_sidebar.width, 20);
    }

    #[test]
    fn sidebar_hidden_when_not_requested() {
        let area = Rect::new(0, 0, MIN_TERMINAL_WIDTH, 24);
        let result = home(area, false);
        assert_eq!(result.left_sidebar.width, 0);
    }
}
