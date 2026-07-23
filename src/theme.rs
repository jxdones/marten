use crate::{action::Action, state::Overlay, tui::theme};

pub fn update(overlay: &mut Overlay, action: Action) {
    let Overlay::ThemeSelector(state) = overlay else {
        return;
    };

    match action {
        Action::MoveDown => state.select_next(theme::THEMES.len()),
        Action::MoveUp => state.select_previous(theme::THEMES.len()),
        _ => {}
    }
}
