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
    let lowercase_query = query.to_lowercase();

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

            let range = matching_range(&file.entry.path, &lowercase_query)?;

            Some(FileMatch {
                file_index: index,
                range: Some(range),
            })
        })
        .collect()
}

fn matching_range(path: &str, lowercase_query: &str) -> Option<Range<usize>> {
    let start = path.to_lowercase().find(lowercase_query)?;
    let end = start + lowercase_query.len();
    if path.is_ascii() {
        return Some(start..end);
    }
    let mut lowercase_offset = 0;
    let mut original_start = None;

    for (offset, character) in path.char_indices() {
        // Lowercasing can change UTF-8 length or expand one character into several.
        // Highlight whole characters in the original path, even for a partial expansion.
        lowercase_offset += character.to_lowercase().map(char::len_utf8).sum::<usize>();
        if original_start.is_none() && start < lowercase_offset {
            original_start = Some(offset);
        }
        if end <= lowercase_offset {
            return Some(original_start?..offset + character.len_utf8());
        }
    }

    None
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        git::repository::{FileEntry, FileStatus},
        store::DiffStore,
    };

    #[test]
    fn unicode_search_highlights_original_filename_characters() {
        for (path, query, highlighted) in [
            ("src/main.rs", "MAIN", "main"),
            ("src/Änderung.rs", "änderung", "Änderung"),
            ("src/Änderung.rs", "ÄNDERUNG", "Änderung"),
            ("Kelvin.rs", "kelvin", "Kelvin"),
            ("İstanbul.rs", "i", "İ"),
        ] {
            let store = DiffStore::new(
                vec![FileEntry {
                    path: path.into(),
                    previous_path: None,
                    status: FileStatus::Untracked,
                    change: None,
                    type_change: None,
                    insertions: 0,
                    deletions: 0,
                }],
                vec![],
                false,
            );
            let matches = matching_files(&store.continuous_diff.files, query);

            assert_eq!(matches.len(), 1, "path: {path}, query: {query}");
            assert_eq!(&path[matches[0].range.clone().unwrap()], highlighted);
        }
    }
}
