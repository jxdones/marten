use ratatui::{Frame, layout::Rect};

use crate::app::App;
use crate::tui::components::{branches_panel, files_panel, panel, stash_panel};
use crate::tui::layout;

pub fn draw(frame: &mut Frame, area: Rect, app: &mut App, is_focused: bool) {
    let layout = layout::left_sidebar(area);

    files_panel::draw(frame, layout.files, app, is_focused);
    branches_panel::draw(frame, layout.branches, app, false);
    stash_panel::draw(frame, layout.stash, app, false);
}
