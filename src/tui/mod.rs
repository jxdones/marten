pub mod components;
pub mod layout;
pub mod screens;
pub mod theme;

use ratatui::Frame;

use crate::{app::App, state::Screen};

pub fn draw(frame: &mut Frame, app: &mut App) {
    let area = frame.area();

    match app.screen() {
        Screen::Home => screens::home::draw(frame, area, app),
    }
}
