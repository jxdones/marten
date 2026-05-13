use ratatui::style::{Color, Modifier, Style};

#[derive(Debug, Clone, Copy)]
pub struct Theme {
    pub focus_border: Color,
    pub panel_border: Color,
    pub text_primary: Color,
    pub text_muted: Color,
    pub accent: Color,
    pub success: Color,
    pub danger: Color,
}

pub const DEFAULT: Theme = Theme {
    focus_border: Color::Blue,
    panel_border: Color::DarkGray,
    text_primary: Color::White,
    text_muted: Color::Gray,
    accent: Color::Rgb(230, 180, 90),
    success: Color::Green,
    danger: Color::Red,
};

impl Theme {
    pub fn focused_border(self) -> Style {
        Style::default().fg(self.focus_border)
    }

    pub fn panel_border(self) -> Style {
        Style::default().fg(self.panel_border)
    }

    pub fn repo_name(self) -> Style {
        Style::default()
            .fg(self.accent)
            .add_modifier(Modifier::BOLD)
    }

    pub fn branch_name(self) -> Style {
        Style::default()
            .fg(self.text_primary)
            .add_modifier(Modifier::BOLD)
    }

    pub fn muted(self) -> Style {
        Style::default().fg(self.text_muted)
    }

    pub fn text_primary(self) -> Style {
        Style::default().fg(self.text_primary)
    }

    pub fn accent(self) -> Style {
        Style::default().fg(self.accent)
    }

    pub fn success(self) -> Style {
        Style::default().fg(self.success)
    }

    pub fn danger(self) -> Style {
        Style::default().fg(self.danger)
    }
}
