use std::collections::HashSet;

use crate::action::Action;
use crate::git::repository::FileEntry;
use crate::state::tree::tree_rows;
use crate::state::{FileKey, Files, Focus, TreeRow};
use crate::store::DiffStore;

pub struct FilesPanel {
    state: Files,
    cached_rows: Vec<TreeRow>,
    collapsed: HashSet<String>,
    dirty: bool,
}

impl FilesPanel {
    pub fn new() -> Self {
        Self {
            state: Files::default(),
            cached_rows: Vec::new(),
            collapsed: HashSet::new(),
            dirty: true,
        }
    }

    pub fn update(&mut self, action: Action, focus: Focus, store: &DiffStore) -> bool {
        match action {
            Action::MoveDown if focus == Focus::Files => {
                self.select_next_row();
                true
            }
            Action::MoveUp if focus == Focus::Files => {
                self.select_previous_row();
                true
            }
            Action::NextFile => {
                self.select_next_file();
                true
            }
            Action::PreviousFile => {
                self.select_previous_file();
                true
            }
            Action::GoToFirst => {
                self.select_first();
                true
            }
            Action::GoToLast => {
                self.select_last();
                true
            }
            Action::ToggleCollapsed => {
                if let Some(path) = self.selected_dir() {
                    self.toggle_collapsed(path);
                    self.ensure_rows(store);
                    self.clamp_selection();
                }
                false
            }
            _ => false,
        }
    }

    pub fn mark_dirty(&mut self) {
        self.dirty = true;
    }

    pub const fn state(&self) -> &Files {
        &self.state
    }

    pub fn set_tree_row_count(&mut self, len: usize) {
        self.state.tree_row_count = len;
    }

    pub fn cached_rows(&self) -> &[TreeRow] {
        &self.cached_rows
    }

    pub const fn collapsed(&self) -> &HashSet<String> {
        &self.collapsed
    }

    pub fn ensure_rows(&mut self, store: &DiffStore) {
        if !self.dirty {
            return;
        }
        self.cached_rows = tree_rows(&store.review_doc.files, &self.collapsed);
        self.state.tree_row_count = self.cached_rows.len();
        self.dirty = false;
    }

    pub fn select_first(&mut self) {
        self.state.select_first();
    }

    pub fn select_last(&mut self) {
        self.state.select_last();
    }

    pub fn select_next_row(&mut self) {
        self.state.select_next();
    }

    pub fn select_previous_row(&mut self) {
        self.state.select_previous();
    }

    pub fn select_next_file(&mut self) {
        let current = self.state.selected.unwrap_or(0);
        if let Some(next) = self
            .cached_rows
            .iter()
            .enumerate()
            .skip(current + 1)
            .find(|(_, row)| matches!(row, TreeRow::File(..)))
            .map(|(i, _)| i)
        {
            self.state.selected = Some(next);
        }
    }

    pub fn select_previous_file(&mut self) {
        let current = self.state.selected.unwrap_or(0);
        if let Some(prev) = self
            .cached_rows
            .iter()
            .enumerate()
            .take(current)
            .rfind(|(_, row)| matches!(row, TreeRow::File(..)))
            .map(|(i, _)| i)
        {
            self.state.selected = Some(prev);
        }
    }

    pub fn selected_file_idx(&self) -> Option<usize> {
        let idx = self.state.selected?;
        match self.cached_rows.get(idx)? {
            TreeRow::File(entry_idx, _) => Some(*entry_idx),
            TreeRow::Dir(..) => None,
        }
    }

    pub fn selected_file<'a>(&self, store: &'a DiffStore) -> Option<&'a FileEntry> {
        let idx = self.state.selected?;
        if let TreeRow::File(entry_idx, _) = self.cached_rows.get(idx)? {
            return store
                .review_doc
                .files
                .get(*entry_idx)
                .map(|slot| &slot.entry);
        }
        None
    }

    pub fn selected_dir(&self) -> Option<String> {
        let idx = self.state.selected?;
        match self.cached_rows.get(idx)? {
            TreeRow::Dir(path, _) => Some(path.clone()),
            _ => None,
        }
    }

    pub fn toggle_collapsed(&mut self, path: String) {
        if !self.collapsed.remove(&path) {
            self.collapsed.insert(path);
        }
        self.dirty = true;
    }

    pub fn clamp_selection(&mut self) {
        let len = self.cached_rows.len();
        if len == 0 {
            self.state.selected = None;
        } else if let Some(sel) = self.state.selected {
            self.state.selected = Some(sel.min(len - 1));
        }
    }

    pub fn match_selected_file(&mut self, store: &DiffStore, continuous_scroll: usize) {
        let Some((file_idx, _)) = store.review_doc.index.file_at_row(continuous_scroll) else {
            return;
        };

        if let Some(pos) = self
            .cached_rows
            .iter()
            .enumerate()
            .find(|(_, row)| matches!(row, TreeRow::File(entry_idx, _) if *entry_idx == file_idx))
            .map(|(pos, _)| pos)
        {
            self.state.selected = Some(pos);
            return;
        }

        let path = &store.review_doc.files[file_idx].entry.path;
        let to_expand: Vec<String> = self
            .collapsed
            .iter()
            .filter(|c| **path == **c || path.starts_with(&format!("{c}/")))
            .cloned()
            .collect();

        for dir in to_expand {
            self.collapsed.remove(&dir);
        }

        self.dirty = true;
        self.ensure_rows(store);

        self.state.selected = self
            .cached_rows
            .iter()
            .enumerate()
            .find(|(_, row)| matches!(row, TreeRow::File(entry_idx, _) if *entry_idx == file_idx))
            .map(|(pos, _)| pos);
    }

    pub fn restore_selection(
        &mut self,
        store: &DiffStore,
        selected_key: Option<FileKey>,
    ) -> Option<usize> {
        if self.cached_rows.is_empty() {
            self.state.selected = None;
            return None;
        }

        if let Some(key) = selected_key
            && let Some(&file_idx) = store.review_doc.by_key.get(&key)
            && let Some(row_idx) = self.cached_rows.iter().position(
                |row| matches!(row, TreeRow::File(entry_idx, _) if *entry_idx == file_idx),
            )
        {
            self.state.selected = Some(row_idx);
            return Some(file_idx);
        }

        self.state.selected = Some(
            self.state
                .selected
                .unwrap_or(0)
                .min(self.cached_rows.len() - 1),
        );
        self.selected_file_idx()
    }

    #[cfg(test)]
    pub fn collapse_dir_for_test(&mut self, path: impl Into<String>) {
        self.collapsed.insert(path.into());
        self.mark_dirty();
    }
}
