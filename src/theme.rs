use crate::{action::Action, state::Overlay, tui::theme};

pub fn update(overlay: &mut Overlay, action: Action) {
    let Overlay::ThemeSelector(state) = overlay else {
        return;
    };

    match action {
        Action::MoveDown | Action::MoveUp => {
            let visible = theme::visible_themes(state.filter);
            let current_position = visible
                .iter()
                .position(|(index, _)| *index == state.selected)
                .unwrap_or(0);

            let len = visible.len();
            let next_position = match action {
                Action::MoveDown => (current_position + 1) % len,
                Action::MoveUp => (current_position + len - 1) % len,
                _ => unreachable!(),
            };

            state.selected = visible[next_position].0;
        }
        Action::ToggleThemeFilter => {
            state.filter = state.filter.cycle();

            let visible = theme::visible_themes(state.filter);
            if !visible.iter().any(|(index, _)| *index == state.selected)
                && let Some((index, _)) = visible.first()
            {
                state.selected = *index;
            }
        }
        _ => {}
    }
}
