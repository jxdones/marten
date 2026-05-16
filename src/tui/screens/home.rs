use ratatui::{Frame, layout::Rect};

use crate::{
    app::App,
    state::Focus,
    tui::{
        components::{diff_panel, history_panel, left_sidebar, shortcuts, top_bar},
        layout,
    },
};

pub fn draw(frame: &mut Frame, area: Rect, app: &mut App) {
    let layout = layout::home(area);
    let focus = app.focus();

    top_bar::draw(frame, layout.top_bar, app);
    left_sidebar::draw(frame, layout.left_sidebar, app, focus == Focus::Files);

    app.set_diff_viewport_height(layout.diff.height.saturating_sub(2) as usize);
    diff_panel::draw(frame, layout.diff, app, focus == Focus::Diff);

    history_panel::draw(frame, layout.history, app, focus == Focus::History);

    shortcuts::draw(frame, layout.shortcuts, app);
}
