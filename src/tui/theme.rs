use ratatui::style::{Color, Modifier, Style};

#[derive(Debug, Clone, Copy)]
pub struct Theme {
    pub bg: Color,
    pub panel: Color,
    pub panel_focused: Color,
    pub line: Color,
    pub line_hi: Color,

    pub fg: Color,
    pub dim: Color,
    pub vdim: Color,

    pub accent: Color,
    pub select: Color,
    pub select_hi: Color,

    pub add_bg: Color,
    pub add_fg: Color,
    pub add_gutter: Color,
    pub del_bg: Color,
    pub del_fg: Color,
    pub del_gutter: Color,

    pub staged: Color,
    pub partial: Color,
    pub unstaged: Color,
    pub untracked: Color,
    pub conflict: Color,
}

pub const DEFAULT: Theme = Theme {
    bg: Color::Rgb(22, 17, 13),
    panel: Color::Rgb(29, 23, 18),
    panel_focused: Color::Rgb(40, 32, 26),
    line: Color::Rgb(85, 68, 54),
    line_hi: Color::Rgb(112, 89, 71),

    fg: Color::Rgb(239, 228, 210),
    dim: Color::Rgb(160, 141, 118),
    vdim: Color::Rgb(94, 79, 66),

    accent: Color::Rgb(212, 163, 104),
    select: Color::Rgb(41, 32, 22),
    select_hi: Color::Rgb(64, 49, 33),

    add_bg: Color::Rgb(42, 43, 29),
    add_fg: Color::Rgb(181, 201, 122),
    add_gutter: Color::Rgb(138, 168, 105),
    del_bg: Color::Rgb(50, 27, 20),
    del_fg: Color::Rgb(224, 139, 111),
    del_gutter: Color::Rgb(196, 82, 58),

    staged: Color::Rgb(181, 201, 122),
    partial: Color::Rgb(220, 190, 100),
    unstaged: Color::Rgb(224, 139, 111),
    untracked: Color::Rgb(212, 163, 104),
    conflict: Color::Rgb(212, 84, 63),
};

impl Theme {
    pub fn focused_border(self) -> Style {
        Style::default().fg(self.accent)
    }

    pub fn panel_border(self) -> Style {
        Style::default().fg(self.line)
    }

    pub fn repo_name(self) -> Style {
        Style::default()
            .fg(self.accent)
            .add_modifier(Modifier::BOLD)
    }

    pub fn branch_name(self) -> Style {
        Style::default().fg(self.fg).add_modifier(Modifier::BOLD)
    }

    pub fn muted(self) -> Style {
        Style::default().fg(self.dim)
    }

    pub fn text_primary(self) -> Style {
        Style::default().fg(self.fg)
    }

    pub fn accent(self) -> Style {
        Style::default().fg(self.accent)
    }

    pub fn success(self) -> Style {
        Style::default().fg(self.staged)
    }

    pub fn danger(self) -> Style {
        Style::default().fg(self.conflict)
    }

    pub fn staged(self) -> Style {
        Style::default().fg(self.staged)
    }

    pub fn partial(self) -> Style {
        Style::default().fg(self.partial)
    }

    pub fn unstaged(self) -> Style {
        Style::default().fg(self.unstaged)
    }

    pub fn untracked(self) -> Style {
        Style::default().fg(self.untracked)
    }

    pub fn conflict(self) -> Style {
        Style::default().fg(self.conflict)
    }

    pub fn diff_add(self) -> Style {
        Style::default().fg(self.add_fg).bg(self.add_bg)
    }

    pub fn diff_del(self) -> Style {
        Style::default().fg(self.del_fg).bg(self.del_bg)
    }
}
