use ratatui::{Frame, layout::Rect};

use crate::{
    app::App,
    state::Focus,
    tui::{
        components::{diff_panel, history_panel, left_sidebar, right_sidebar, shortcuts, top_bar},
        layout,
    },
};

pub fn draw(frame: &mut Frame, area: Rect, app: &App) {
    let layout = layout::home(area);

    top_bar::draw(frame, layout.top_bar, app, false);
    left_sidebar::draw(frame, layout.left_sidebar, app, app.focus() == Focus::Files);

    diff_panel::draw(frame, layout.diff, app, app.focus() == Focus::Diff);

    history_panel::draw(frame, layout.history, app, app.focus() == Focus::History);

    right_sidebar::draw(
        frame,
        layout.right_sidebar,
        app,
        app.focus() == Focus::Details,
    );

    shortcuts::draw(frame, layout.shortcuts, app);
}
