use crate::{
    action::Action,
    git::repository::CommitInfo,
    state::{CommitsFinderFocus, Overlay},
};

pub fn matching_commits(commits: &[CommitInfo], query: &str) -> Vec<usize> {
    let query = query.to_lowercase();

    commits
        .iter()
        .enumerate()
        .filter_map(|(index, commit)| {
            if query.is_empty()
                || commit.oid.to_string().contains(&query)
                || commit.subject.to_lowercase().contains(&query)
            {
                Some(index)
            } else {
                None
            }
        })
        .collect()
}

pub fn selected_commit<'a>(overlay: &Overlay, commits: &'a [CommitInfo]) -> Option<&'a CommitInfo> {
    let Overlay::CommitsFinder(state) = overlay else {
        return None;
    };

    matching_commits(commits, &state.query)
        .get(state.selected)
        .and_then(|index| commits.get(*index))
}

pub fn update(overlay: &mut Overlay, action: Action, commits: &[CommitInfo]) {
    let Overlay::CommitsFinder(state) = overlay else {
        return;
    };
    let match_count = matching_commits(commits, &state.query).len();

    match action {
        Action::MoveDown => state.select_next(match_count),
        Action::MoveUp => state.select_previous(match_count),
        Action::CommitsFinderInput(character) => state.insert(character),
        Action::CommitsFinderBackspace => state.backspace(),
        Action::CommitsFinderClear => state.clear(),
        Action::ToggleCommitsFinderFocus => state.toggle_focus(),
        Action::FocusCommitsFinderList => state.focus = CommitsFinderFocus::List,
        _ => {}
    }
}
