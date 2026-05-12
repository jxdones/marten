pub mod layout;
use ratatui::Frame;

use crate::app::App;

pub fn draw(frame: &mut Frame, _app: &App) {
    let area = frame.area();
    layout::draw_base_layout(frame, area);
}
