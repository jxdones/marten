use ratatui::{Frame, layout::Rect};

use crate::app::App;
use crate::state::Focus;
use crate::tui::components::files_panel;

pub fn draw(frame: &mut Frame, area: Rect, app: &mut App, focus: Focus) {
    files_panel::draw(frame, area, app, focus == Focus::Files);
}
