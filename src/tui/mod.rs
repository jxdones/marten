pub mod layout;
use ratatui::Frame;

pub fn draw(frame: &mut Frame) {
    let area = frame.area();
    layout::draw_base_layout(frame, area);
}
