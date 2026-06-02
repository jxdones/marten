use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Layout, Rect},
    text::{Line, Span},
    widgets::Paragraph,
};

use crate::{
    app::App,
    state::Focus,
    tui::{
        components::{diff_panel, left_sidebar, shortcuts, top_bar},
        layout,
        theme::Theme,
    },
};

pub fn draw(frame: &mut Frame, area: Rect, app: &mut App) {
    if area.width < 80 || area.height < 24 {
        draw_too_small(frame, area, app.theme());
        return;
    }

    let layout = layout::home(area);
    let focus = app.focus();

    top_bar::draw(frame, layout.top_bar, app);
    left_sidebar::draw(frame, layout.left_sidebar, app, focus);

    app.set_diff_viewport_height(layout.diff.height.saturating_sub(2) as usize);
    diff_panel::draw(frame, layout.diff, app, focus == Focus::Diff);

    shortcuts::draw(frame, layout.shortcuts, app);
}

fn draw_too_small(frame: &mut Frame, area: Rect, theme: Theme) {
    let width_status_color = if area.width >= 80 {
        theme.success()
    } else {
        theme.danger()
    };

    let height_status_color = if area.height >= 24 {
        theme.success()
    } else {
        theme.danger()
    };

    let lines = vec![
        Line::from(Span::styled("Oops!", theme.muted())),
        Line::from(Span::styled("Terminal size too small", theme.muted())),
        Line::from(vec![
            Span::styled(area.width.to_string(), width_status_color),
            Span::styled("x", theme.muted()),
            Span::styled(area.height.to_string(), height_status_color),
        ]),
    ];

    let max_width = lines.iter().map(|l| l.width()).max().unwrap_or(0) as u16;
    let widget = Paragraph::new(lines).alignment(Alignment::Center);

    let vertical = Layout::vertical([
        Constraint::Fill(1),
        Constraint::Length(3),
        Constraint::Fill(1),
    ])
    .split(area);

    let horizontal = Layout::horizontal([
        Constraint::Fill(1),
        Constraint::Length(max_width),
        Constraint::Fill(1),
    ])
    .split(vertical[1]);

    frame.render_widget(widget, horizontal[1]);
}
