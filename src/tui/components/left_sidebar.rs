use ratatui::{Frame, layout::Rect};

use crate::app::App;
use crate::tui::components::panel;

pub fn draw(frame: &mut Frame, area: Rect, app: &App, is_focused: bool) {
    frame.render_widget(panel::block("left sidebar", app.theme(), is_focused), area);
}
