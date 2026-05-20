use ratatui::{Frame, layout::Rect};

use crate::app::App;
use crate::state::Focus;
use crate::tui::components::{branches_panel, files_panel, stash_panel};
use crate::tui::layout;

pub fn draw(frame: &mut Frame, area: Rect, app: &mut App, focus: Focus) {
    let layout = layout::left_sidebar(area);

    files_panel::draw(frame, layout.files, app, focus == Focus::Files);
    branches_panel::draw(frame, layout.branches, app, focus == Focus::Branches);
    stash_panel::draw(frame, layout.stash, app, focus == Focus::Stash);
}
