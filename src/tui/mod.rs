pub mod components;
pub mod layout;
pub mod screens;
pub mod theme;

use ratatui::Frame;
use ratatui::style::Style;
use ratatui::widgets::{Block, Borders};

use crate::{app::App, state::Screen};

pub fn draw(frame: &mut Frame, app: &mut App) {
    let area = frame.area();
    let theme = app.theme();

    // Clear the entire frame with the theme background so no terminal
    // default color bleeds through empty space.
    frame.render_widget(
        Block::default()
            .borders(Borders::NONE)
            .style(Style::default().bg(theme.bg)),
        area,
    );

    match app.screen() {
        Screen::Home => screens::home::draw(frame, area, app),
    }
}
