use ratatui::style::{Color, Modifier, Style};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Theme {
    pub syntax_theme: &'static str,
    pub bg: Color,
    pub sidebar_bg: Color,
    pub line: Color,

    pub fg: Color,
    pub dim: Color,

    pub accent: Color,
    pub select: Color,
    pub select_hi: Color,

    pub file_header_bg: Color,
    pub hunk_header_bg: Color,

    pub add_bg: Color,
    pub add_inline_bg: Color,
    pub add_fg: Color,
    pub add_gutter: Color,
    pub del_bg: Color,
    pub del_inline_bg: Color,
    pub del_fg: Color,
    pub del_gutter: Color,

    pub staged: Color,
    pub partial: Color,
    pub unstaged: Color,
    pub untracked: Color,
    pub conflict: Color,
}

pub struct ThemeEntry {
    pub name: &'static str,
    pub id: &'static str,
    pub appearance: &'static str,
    pub theme: Theme,
}

pub const THEMES: &[ThemeEntry] = &[
    ThemeEntry {
        name: "Marten",
        id: "marten",
        appearance: "dark",
        theme: MARTEN_DARK,
    },
    ThemeEntry {
        name: "Ermine",
        id: "ermine",
        appearance: "light",
        theme: MARTEN_LIGHT,
    },
];

pub fn entry_by_id(id: &str) -> Option<&'static ThemeEntry> {
    THEMES.iter().find(|entry| entry.id == id)
}

pub fn default_entry() -> &'static ThemeEntry {
    &THEMES[0]
}

// Original to Marten.
pub const MARTEN_DARK: Theme = Theme {
    syntax_theme: "base16-ocean.dark",

    bg: Color::Rgb(22, 17, 13),
    sidebar_bg: Color::Rgb(22, 17, 13),
    line: Color::Rgb(85, 68, 54),

    fg: Color::Rgb(239, 228, 210),
    dim: Color::Rgb(160, 141, 118),

    accent: Color::Rgb(212, 163, 104),
    select: Color::Rgb(46, 35, 23),
    select_hi: Color::Rgb(70, 52, 31),

    file_header_bg: Color::Rgb(29, 23, 18),
    hunk_header_bg: Color::Rgb(36, 29, 23),

    add_bg: Color::Rgb(43, 46, 28),
    add_inline_bg: Color::Rgb(68, 82, 38),
    add_fg: Color::Rgb(181, 201, 122),
    add_gutter: Color::Rgb(138, 168, 105),
    del_bg: Color::Rgb(47, 29, 22),
    del_inline_bg: Color::Rgb(86, 35, 27),
    del_fg: Color::Rgb(224, 139, 111),
    del_gutter: Color::Rgb(196, 82, 58),

    staged: Color::Rgb(181, 201, 122),
    partial: Color::Rgb(224, 194, 78),
    unstaged: Color::Rgb(224, 139, 111),
    untracked: Color::Rgb(212, 163, 104),
    conflict: Color::Rgb(230, 88, 106),
};

// Original to Marten.
pub const MARTEN_LIGHT: Theme = Theme {
    syntax_theme: "base16-ocean.light",

    bg: Color::Rgb(245, 239, 228),
    sidebar_bg: Color::Rgb(245, 239, 228),
    line: Color::Rgb(212, 198, 175),

    fg: Color::Rgb(48, 38, 32),
    dim: Color::Rgb(124, 107, 87),

    accent: Color::Rgb(158, 101, 26),
    select: Color::Rgb(244, 231, 196),
    select_hi: Color::Rgb(235, 217, 168),

    file_header_bg: Color::Rgb(243, 236, 223),
    hunk_header_bg: Color::Rgb(231, 220, 199),

    add_bg: Color::Rgb(233, 238, 211),
    add_inline_bg: Color::Rgb(200, 218, 152),
    add_fg: Color::Rgb(88, 112, 43),
    add_gutter: Color::Rgb(122, 152, 60),
    del_bg: Color::Rgb(251, 229, 220),
    del_inline_bg: Color::Rgb(245, 197, 183),
    del_fg: Color::Rgb(158, 62, 40),
    del_gutter: Color::Rgb(190, 76, 52),

    staged: Color::Rgb(88, 112, 43),
    partial: Color::Rgb(166, 122, 28),
    unstaged: Color::Rgb(158, 62, 40),
    untracked: Color::Rgb(158, 101, 26),
    conflict: Color::Rgb(178, 36, 74),
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

    pub fn hunk_header(self) -> Style {
        Style::default().fg(self.dim).bg(self.hunk_header_bg)
    }
}
