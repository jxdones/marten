use ratatui::style::{Color, Modifier, Style};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Theme {
    pub syntax_theme: &'static str,
    pub syntax_palette: Option<SyntaxPalette>,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SyntaxPalette {
    pub comment: Color,
    pub keyword: Color,
    pub function: Color,
    pub variable: Color,
    pub string: Color,
    pub number: Color,
    pub type_name: Color,
    pub operator: Color,
    pub punctuation: Color,
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
    ThemeEntry {
        name: "Catppuccin",
        id: "catppuccin",
        appearance: "dark",
        theme: CATPPUCCIN,
    },
    ThemeEntry {
        name: "Catppuccin Latte",
        id: "catppuccin-latte",
        appearance: "light",
        theme: CATPPUCCIN_LATTE,
    },
    ThemeEntry {
        name: "Dracula",
        id: "dracula",
        appearance: "dark",
        theme: DRACULA,
    },
    ThemeEntry {
        name: "Everforest",
        id: "everforest",
        appearance: "dark",
        theme: EVERFOREST,
    },
    ThemeEntry {
        name: "Everforest Light Soft",
        id: "everforest-light-soft",
        appearance: "light",
        theme: EVERFOREST_LIGHT_SOFT,
    },
    ThemeEntry {
        name: "Gruvbox",
        id: "gruvbox",
        appearance: "dark",
        theme: GRUVBOX,
    },
    ThemeEntry {
        name: "Tokyo Night",
        id: "tokyonight",
        appearance: "dark",
        theme: TOKYO_NIGHT,
    },
    ThemeEntry {
        name: "GitHub",
        id: "github",
        appearance: "dark",
        theme: GITHUB,
    },
    ThemeEntry {
        name: "GitHub Light",
        id: "github-light",
        appearance: "light",
        theme: GITHUB_LIGHT,
    },
    ThemeEntry {
        name: "Flexoki Light",
        id: "flexoki-light",
        appearance: "light",
        theme: FLEXOKI_LIGHT,
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
    syntax_palette: None,

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
    syntax_palette: None,

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

pub const CATPPUCCIN: Theme = Theme {
    syntax_theme: "catppuccin",
    syntax_palette: Some(SyntaxPalette {
        comment: Color::Rgb(147, 153, 178),
        keyword: Color::Rgb(203, 166, 247),
        function: Color::Rgb(137, 180, 250),
        variable: Color::Rgb(243, 139, 168),
        string: Color::Rgb(166, 227, 161),
        number: Color::Rgb(250, 179, 135),
        type_name: Color::Rgb(249, 226, 175),
        operator: Color::Rgb(137, 220, 235),
        punctuation: Color::Rgb(205, 214, 244),
    }),

    bg: Color::Rgb(30, 30, 46),
    sidebar_bg: Color::Rgb(30, 30, 46),
    line: Color::Rgb(49, 50, 68),

    fg: Color::Rgb(205, 214, 244),
    dim: Color::Rgb(147, 153, 178),

    accent: Color::Rgb(137, 180, 250),
    select: Color::Rgb(49, 50, 68),
    select_hi: Color::Rgb(69, 71, 90),

    file_header_bg: Color::Rgb(24, 24, 37),
    hunk_header_bg: Color::Rgb(49, 50, 68),

    add_bg: Color::Rgb(36, 49, 43),
    add_inline_bg: Color::Rgb(50, 78, 59),
    add_fg: Color::Rgb(166, 227, 161),
    add_gutter: Color::Rgb(166, 227, 161),
    del_bg: Color::Rgb(60, 42, 50),
    del_inline_bg: Color::Rgb(90, 48, 60),
    del_fg: Color::Rgb(243, 139, 168),
    del_gutter: Color::Rgb(243, 139, 168),

    staged: Color::Rgb(166, 227, 161),
    partial: Color::Rgb(249, 226, 175),
    unstaged: Color::Rgb(243, 139, 168),
    untracked: Color::Rgb(250, 179, 135),
    conflict: Color::Rgb(243, 139, 168),
};

pub const DRACULA: Theme = Theme {
    syntax_theme: "dracula",
    syntax_palette: Some(SyntaxPalette {
        comment: Color::Rgb(98, 114, 164),
        keyword: Color::Rgb(255, 121, 198),
        function: Color::Rgb(80, 250, 123),
        variable: Color::Rgb(248, 248, 242),
        string: Color::Rgb(241, 250, 140),
        number: Color::Rgb(189, 147, 249),
        type_name: Color::Rgb(139, 233, 253),
        operator: Color::Rgb(255, 121, 198),
        punctuation: Color::Rgb(248, 248, 242),
    }),

    bg: Color::Rgb(40, 42, 54),
    sidebar_bg: Color::Rgb(40, 42, 54),
    line: Color::Rgb(68, 71, 90),

    fg: Color::Rgb(248, 248, 242),
    dim: Color::Rgb(98, 114, 164),

    accent: Color::Rgb(189, 147, 249),
    select: Color::Rgb(68, 71, 90),
    select_hi: Color::Rgb(85, 88, 109),

    file_header_bg: Color::Rgb(33, 34, 44),
    hunk_header_bg: Color::Rgb(68, 71, 90),

    add_bg: Color::Rgb(26, 58, 26),
    add_inline_bg: Color::Rgb(35, 85, 42),
    add_fg: Color::Rgb(80, 250, 123),
    add_gutter: Color::Rgb(80, 250, 123),
    del_bg: Color::Rgb(58, 26, 26),
    del_inline_bg: Color::Rgb(91, 36, 43),
    del_fg: Color::Rgb(255, 85, 85),
    del_gutter: Color::Rgb(255, 85, 85),

    staged: Color::Rgb(80, 250, 123),
    partial: Color::Rgb(241, 250, 140),
    unstaged: Color::Rgb(255, 85, 85),
    untracked: Color::Rgb(255, 184, 108),
    conflict: Color::Rgb(255, 85, 85),
};

pub const EVERFOREST: Theme = Theme {
    syntax_theme: "everforest",
    syntax_palette: Some(SyntaxPalette {
        comment: Color::Rgb(122, 132, 120),
        keyword: Color::Rgb(214, 153, 182),
        function: Color::Rgb(167, 192, 128),
        variable: Color::Rgb(230, 126, 128),
        string: Color::Rgb(167, 192, 128),
        number: Color::Rgb(230, 152, 117),
        type_name: Color::Rgb(219, 188, 127),
        operator: Color::Rgb(131, 192, 146),
        punctuation: Color::Rgb(211, 198, 170),
    }),

    bg: Color::Rgb(45, 53, 59),
    sidebar_bg: Color::Rgb(45, 53, 59),
    line: Color::Rgb(71, 82, 88),

    fg: Color::Rgb(211, 198, 170),
    dim: Color::Rgb(122, 132, 120),

    accent: Color::Rgb(167, 192, 128),
    select: Color::Rgb(52, 63, 68),
    select_hi: Color::Rgb(61, 72, 77),

    file_header_bg: Color::Rgb(51, 60, 67),
    hunk_header_bg: Color::Rgb(52, 63, 68),

    add_bg: Color::Rgb(32, 48, 59),
    add_inline_bg: Color::Rgb(42, 75, 69),
    add_fg: Color::Rgb(184, 219, 135),
    add_gutter: Color::Rgb(79, 214, 190),
    del_bg: Color::Rgb(55, 34, 44),
    del_inline_bg: Color::Rgb(91, 45, 57),
    del_fg: Color::Rgb(226, 106, 117),
    del_gutter: Color::Rgb(197, 59, 83),

    staged: Color::Rgb(167, 192, 128),
    partial: Color::Rgb(219, 188, 127),
    unstaged: Color::Rgb(230, 126, 128),
    untracked: Color::Rgb(230, 152, 117),
    conflict: Color::Rgb(230, 126, 128),
};

pub const GRUVBOX: Theme = Theme {
    syntax_theme: "gruvbox",
    syntax_palette: Some(SyntaxPalette {
        comment: Color::Rgb(146, 131, 116),
        keyword: Color::Rgb(251, 73, 52),
        function: Color::Rgb(184, 187, 38),
        variable: Color::Rgb(131, 165, 152),
        string: Color::Rgb(250, 189, 47),
        number: Color::Rgb(211, 134, 155),
        type_name: Color::Rgb(142, 192, 124),
        operator: Color::Rgb(254, 128, 25),
        punctuation: Color::Rgb(235, 219, 178),
    }),

    bg: Color::Rgb(40, 40, 40),
    sidebar_bg: Color::Rgb(40, 40, 40),
    line: Color::Rgb(102, 92, 84),

    fg: Color::Rgb(235, 219, 178),
    dim: Color::Rgb(146, 131, 116),

    accent: Color::Rgb(131, 165, 152),
    select: Color::Rgb(80, 73, 69),
    select_hi: Color::Rgb(102, 92, 84),

    file_header_bg: Color::Rgb(60, 56, 54),
    hunk_header_bg: Color::Rgb(80, 73, 69),

    add_bg: Color::Rgb(50, 48, 47),
    add_inline_bg: Color::Rgb(68, 72, 38),
    add_fg: Color::Rgb(184, 187, 38),
    add_gutter: Color::Rgb(152, 151, 26),
    del_bg: Color::Rgb(50, 41, 41),
    del_inline_bg: Color::Rgb(82, 43, 38),
    del_fg: Color::Rgb(251, 73, 52),
    del_gutter: Color::Rgb(204, 36, 29),

    staged: Color::Rgb(184, 187, 38),
    partial: Color::Rgb(250, 189, 47),
    unstaged: Color::Rgb(251, 73, 52),
    untracked: Color::Rgb(254, 128, 25),
    conflict: Color::Rgb(251, 73, 52),
};

pub const TOKYO_NIGHT: Theme = Theme {
    syntax_theme: "tokyonight",
    syntax_palette: Some(SyntaxPalette {
        comment: Color::Rgb(130, 139, 184),
        keyword: Color::Rgb(192, 153, 255),
        function: Color::Rgb(130, 170, 255),
        variable: Color::Rgb(255, 117, 127),
        string: Color::Rgb(195, 232, 141),
        number: Color::Rgb(255, 150, 108),
        type_name: Color::Rgb(255, 199, 119),
        operator: Color::Rgb(134, 225, 252),
        punctuation: Color::Rgb(200, 211, 245),
    }),

    bg: Color::Rgb(26, 27, 38),
    sidebar_bg: Color::Rgb(26, 27, 38),
    line: Color::Rgb(84, 92, 126),

    fg: Color::Rgb(200, 211, 245),
    dim: Color::Rgb(130, 139, 184),

    accent: Color::Rgb(130, 170, 255),
    select: Color::Rgb(34, 36, 54),
    select_hi: Color::Rgb(41, 46, 66),

    file_header_bg: Color::Rgb(30, 32, 48),
    hunk_header_bg: Color::Rgb(34, 36, 54),

    add_bg: Color::Rgb(32, 48, 59),
    add_inline_bg: Color::Rgb(42, 75, 69),
    add_fg: Color::Rgb(184, 219, 135),
    add_gutter: Color::Rgb(79, 214, 190),
    del_bg: Color::Rgb(55, 34, 44),
    del_inline_bg: Color::Rgb(91, 45, 57),
    del_fg: Color::Rgb(226, 106, 117),
    del_gutter: Color::Rgb(197, 59, 83),

    staged: Color::Rgb(195, 232, 141),
    partial: Color::Rgb(255, 199, 119),
    unstaged: Color::Rgb(255, 117, 127),
    untracked: Color::Rgb(255, 150, 108),
    conflict: Color::Rgb(255, 117, 127),
};

pub const GITHUB: Theme = Theme {
    syntax_theme: "github",
    syntax_palette: Some(SyntaxPalette {
        comment: Color::Rgb(139, 148, 158),
        keyword: Color::Rgb(255, 123, 114),
        function: Color::Rgb(188, 140, 255),
        variable: Color::Rgb(210, 153, 34),
        string: Color::Rgb(57, 197, 207),
        number: Color::Rgb(88, 166, 255),
        type_name: Color::Rgb(210, 153, 34),
        operator: Color::Rgb(255, 123, 114),
        punctuation: Color::Rgb(201, 209, 217),
    }),

    bg: Color::Rgb(13, 17, 23),
    sidebar_bg: Color::Rgb(13, 17, 23),
    line: Color::Rgb(48, 54, 61),

    fg: Color::Rgb(201, 209, 217),
    dim: Color::Rgb(139, 148, 158),

    accent: Color::Rgb(88, 166, 255),
    select: Color::Rgb(22, 27, 34),
    select_hi: Color::Rgb(33, 38, 45),

    file_header_bg: Color::Rgb(1, 4, 9),
    hunk_header_bg: Color::Rgb(22, 27, 34),

    add_bg: Color::Rgb(3, 58, 22),
    add_inline_bg: Color::Rgb(5, 90, 35),
    add_fg: Color::Rgb(63, 185, 80),
    add_gutter: Color::Rgb(63, 185, 80),
    del_bg: Color::Rgb(103, 6, 12),
    del_inline_bg: Color::Rgb(142, 17, 26),
    del_fg: Color::Rgb(248, 81, 73),
    del_gutter: Color::Rgb(248, 81, 73),

    staged: Color::Rgb(63, 185, 80),
    partial: Color::Rgb(227, 179, 65),
    unstaged: Color::Rgb(248, 81, 73),
    untracked: Color::Rgb(210, 153, 34),
    conflict: Color::Rgb(248, 81, 73),
};

pub const FLEXOKI_LIGHT: Theme = Theme {
    syntax_theme: "flexoki-light",
    syntax_palette: Some(SyntaxPalette {
        comment: Color::Rgb(183, 181, 172),
        keyword: Color::Rgb(102, 128, 11),
        function: Color::Rgb(188, 82, 21),
        variable: Color::Rgb(32, 94, 166),
        string: Color::Rgb(36, 131, 123),
        number: Color::Rgb(94, 64, 157),
        type_name: Color::Rgb(160, 47, 111),
        operator: Color::Rgb(111, 110, 105),
        punctuation: Color::Rgb(111, 110, 105),
    }),

    bg: Color::Rgb(255, 252, 240),
    sidebar_bg: Color::Rgb(255, 252, 240),
    line: Color::Rgb(230, 228, 217),

    fg: Color::Rgb(16, 15, 15),
    dim: Color::Rgb(111, 110, 105),

    accent: Color::Rgb(32, 94, 166),
    select: Color::Rgb(242, 240, 229),
    select_hi: Color::Rgb(218, 216, 206),

    file_header_bg: Color::Rgb(242, 240, 229),
    hunk_header_bg: Color::Rgb(230, 228, 217),

    add_bg: Color::Rgb(237, 238, 207),
    add_inline_bg: Color::Rgb(221, 226, 178),
    add_fg: Color::Rgb(102, 128, 11),
    add_gutter: Color::Rgb(102, 128, 11),
    del_bg: Color::Rgb(255, 225, 213),
    del_inline_bg: Color::Rgb(255, 202, 187),
    del_fg: Color::Rgb(175, 48, 41),
    del_gutter: Color::Rgb(175, 48, 41),

    staged: Color::Rgb(102, 128, 11),
    partial: Color::Rgb(173, 131, 1),
    unstaged: Color::Rgb(175, 48, 41),
    untracked: Color::Rgb(188, 82, 21),
    conflict: Color::Rgb(160, 47, 111),
};

pub const GITHUB_LIGHT: Theme = Theme {
    syntax_theme: "github-light",
    syntax_palette: Some(SyntaxPalette {
        comment: Color::Rgb(87, 96, 106),
        keyword: Color::Rgb(207, 34, 46),
        function: Color::Rgb(130, 80, 223),
        variable: Color::Rgb(188, 76, 0),
        string: Color::Rgb(9, 105, 218),
        number: Color::Rgb(27, 124, 131),
        type_name: Color::Rgb(188, 76, 0),
        operator: Color::Rgb(207, 34, 46),
        punctuation: Color::Rgb(36, 41, 47),
    }),

    bg: Color::Rgb(255, 255, 255),
    sidebar_bg: Color::Rgb(255, 255, 255),
    line: Color::Rgb(208, 215, 222),

    fg: Color::Rgb(36, 41, 47),
    dim: Color::Rgb(87, 96, 106),

    accent: Color::Rgb(9, 105, 218),
    select: Color::Rgb(246, 248, 250),
    select_hi: Color::Rgb(216, 222, 228),

    file_header_bg: Color::Rgb(246, 248, 250),
    hunk_header_bg: Color::Rgb(240, 243, 246),

    add_bg: Color::Rgb(218, 251, 225),
    add_inline_bg: Color::Rgb(172, 238, 187),
    add_fg: Color::Rgb(26, 127, 55),
    add_gutter: Color::Rgb(26, 127, 55),
    del_bg: Color::Rgb(255, 235, 233),
    del_inline_bg: Color::Rgb(255, 193, 189),
    del_fg: Color::Rgb(207, 34, 46),
    del_gutter: Color::Rgb(207, 34, 46),

    staged: Color::Rgb(26, 127, 55),
    partial: Color::Rgb(154, 103, 0),
    unstaged: Color::Rgb(207, 34, 46),
    untracked: Color::Rgb(188, 76, 0),
    conflict: Color::Rgb(207, 34, 46),
};

pub const CATPPUCCIN_LATTE: Theme = Theme {
    syntax_theme: "catppuccin-latte",
    syntax_palette: Some(SyntaxPalette {
        comment: Color::Rgb(124, 127, 147),
        keyword: Color::Rgb(136, 57, 239),
        function: Color::Rgb(30, 102, 245),
        variable: Color::Rgb(210, 15, 57),
        string: Color::Rgb(64, 160, 43),
        number: Color::Rgb(254, 100, 11),
        type_name: Color::Rgb(223, 142, 29),
        operator: Color::Rgb(4, 165, 229),
        punctuation: Color::Rgb(76, 79, 105),
    }),

    bg: Color::Rgb(239, 241, 245),
    sidebar_bg: Color::Rgb(239, 241, 245),
    line: Color::Rgb(204, 208, 218),

    fg: Color::Rgb(76, 79, 105),
    dim: Color::Rgb(124, 127, 147),

    accent: Color::Rgb(30, 102, 245),
    select: Color::Rgb(230, 233, 239),
    select_hi: Color::Rgb(220, 224, 232),

    file_header_bg: Color::Rgb(230, 233, 239),
    hunk_header_bg: Color::Rgb(220, 224, 232),

    add_bg: Color::Rgb(214, 240, 217),
    add_inline_bg: Color::Rgb(201, 227, 203),
    add_fg: Color::Rgb(64, 160, 43),
    add_gutter: Color::Rgb(64, 160, 43),
    del_bg: Color::Rgb(246, 223, 226),
    del_inline_bg: Color::Rgb(233, 211, 214),
    del_fg: Color::Rgb(210, 15, 57),
    del_gutter: Color::Rgb(210, 15, 57),

    staged: Color::Rgb(64, 160, 43),
    partial: Color::Rgb(223, 142, 29),
    unstaged: Color::Rgb(210, 15, 57),
    untracked: Color::Rgb(254, 100, 11),
    conflict: Color::Rgb(210, 15, 57),
};

pub const EVERFOREST_LIGHT_SOFT: Theme = Theme {
    syntax_theme: "everforest-light-soft",
    syntax_palette: Some(SyntaxPalette {
        comment: Color::Rgb(130, 145, 129),
        keyword: Color::Rgb(223, 105, 186),
        function: Color::Rgb(141, 161, 1),
        variable: Color::Rgb(248, 85, 82),
        string: Color::Rgb(141, 161, 1),
        number: Color::Rgb(245, 125, 38),
        type_name: Color::Rgb(223, 160, 0),
        operator: Color::Rgb(53, 167, 124),
        punctuation: Color::Rgb(92, 106, 114),
    }),

    bg: Color::Rgb(243, 234, 211),
    sidebar_bg: Color::Rgb(243, 234, 211),
    line: Color::Rgb(216, 211, 186),

    fg: Color::Rgb(92, 106, 114),
    dim: Color::Rgb(130, 145, 129),

    accent: Color::Rgb(141, 161, 1),
    select: Color::Rgb(234, 228, 202),
    select_hi: Color::Rgb(221, 216, 190),

    file_header_bg: Color::Rgb(234, 228, 202),
    hunk_header_bg: Color::Rgb(229, 223, 197),

    add_bg: Color::Rgb(229, 230, 197),
    add_inline_bg: Color::Rgb(211, 217, 169),
    add_fg: Color::Rgb(141, 161, 1),
    add_gutter: Color::Rgb(141, 161, 1),
    del_bg: Color::Rgb(250, 219, 208),
    del_inline_bg: Color::Rgb(242, 194, 178),
    del_fg: Color::Rgb(248, 85, 82),
    del_gutter: Color::Rgb(248, 85, 82),

    staged: Color::Rgb(141, 161, 1),
    partial: Color::Rgb(223, 160, 0),
    unstaged: Color::Rgb(248, 85, 82),
    untracked: Color::Rgb(245, 125, 38),
    conflict: Color::Rgb(248, 85, 82),
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
