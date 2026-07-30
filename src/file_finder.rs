use std::ops::Range;

use crate::{
    action::Action,
    state::{FileSlot, Overlay},
};

pub struct FileMatch {
    pub file_index: usize,
    pub range: Option<Range<usize>>,
}

pub fn matching_files(files: &[FileSlot], query: &str) -> Vec<FileMatch> {
    let lowercase_query = query.to_ascii_lowercase();

    files
        .iter()
        .enumerate()
        .filter_map(|(index, file)| {
            if query.is_empty() {
                return Some(FileMatch {
                    file_index: index,
                    range: None,
                });
            }

            let start = file
                .entry
                .path
                .to_ascii_lowercase()
                .find(&lowercase_query)?;

            Some(FileMatch {
                file_index: index,
                range: Some(start..start + lowercase_query.len()),
            })
        })
        .collect()
}

pub fn selected_file_index(overlay: &Overlay, files: &[FileSlot]) -> Option<usize> {
    let Overlay::FileFinder(state) = overlay else {
        return None;
    };

    matching_files(files, &state.query)
        .get(state.selected)
        .map(|file_match| file_match.file_index)
}

pub fn update(overlay: &mut Overlay, action: Action, files: &[FileSlot]) {
    let Overlay::FileFinder(state) = overlay else {
        return;
    };
    let match_count = matching_files(files, &state.query).len();

    match action {
        Action::MoveDown => state.select_next(match_count),
        Action::MoveUp => state.select_previous(match_count),
        Action::FileFinderBackspace => state.backspace(),
        Action::FileFinderClear => state.clear(),
        Action::FileFinderInput(character) => state.insert(character),
        _ => {}
    }
}
