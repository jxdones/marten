use git2::Repository;

use crate::action::Action;
use crate::files_panel::FilesPanel;
use crate::git::repository::{self, DiffHunk, FileEntry};
use crate::state::line_index::IndexRow;
use crate::state::review::ReviewState;
use crate::state::{Diff, DiffLoadState, FileKey, Focus, LineIndex, ViewMode};
use crate::store::DiffStore;

const SCROLL_STEP: usize = 1;

pub struct DiffPanel {
    state: Diff,
    current_key: Option<FileKey>,
    review: ReviewState,
}

impl DiffPanel {
    pub fn new() -> Self {
        Self {
            state: Diff::default(),
            current_key: None,
            review: ReviewState::default(),
        }
    }

    pub fn update(
        &mut self,
        action: Action,
        focus: Focus,
        selection_changed: bool,
        files: &mut FilesPanel,
        store: &mut DiffStore,
        repo: &Repository,
    ) {
        if selection_changed {
            self.refresh(files, store, repo);
            if self.review.mode == ViewMode::Continuous {
                self.jump_to_selected_file(files, store);
            }
        }

        match action {
            Action::MoveDown if focus == Focus::Diff => match self.review.mode {
                ViewMode::Continuous => {
                    self.continuous_scroll_down(store);
                    self.sync_files_to_scroll(files, store);
                }
                ViewMode::SingleFile => self.scroll_down(),
            },
            Action::MoveUp if focus == Focus::Diff => match self.review.mode {
                ViewMode::Continuous => {
                    self.continuous_scroll_up();
                    self.sync_files_to_scroll(files, store);
                }
                ViewMode::SingleFile => self.scroll_up(),
            },
            Action::NextHunk => {
                self.select_next_hunk(store);
                if self.review.mode == ViewMode::Continuous {
                    self.sync_files_to_scroll(files, store);
                }
            }
            Action::PreviousHunk => {
                self.select_previous_hunk(store);
                if self.review.mode == ViewMode::Continuous {
                    self.sync_files_to_scroll(files, store);
                }
            }
            Action::ToggleDiffLineNumbers => {
                self.state.toggle_line_numbers();
            }
            Action::ToggleViewMode => match self.review.mode {
                ViewMode::Continuous => self.review.mode = ViewMode::SingleFile,
                ViewMode::SingleFile => self.review.mode = ViewMode::Continuous,
            },
            Action::ForceLoadDiff => {
                self.state.too_large = None;
                self.refresh(files, store, repo);
            }
            _ => {}
        }
    }

    pub fn reload(&mut self, files: &mut FilesPanel, store: &mut DiffStore, repo: &Repository) {
        let selected_key = files.selected_file(store).map(|file| FileKey {
            path: file.path.clone(),
            status: file.status,
        });

        let entries = repository::files(repo).unwrap_or_default();

        store.reload(entries);
        files.mark_dirty();
        files.ensure_rows(store);
        let restored_file_idx = files.restore_selection(store, selected_key);
        self.sync_continuous_scroll_to_file(restored_file_idx, store);
        self.reset();
        store.spawn_workers();
        self.refresh(files, store, repo);
    }

    pub const fn state(&self) -> &Diff {
        &self.state
    }

    pub const fn review(&self) -> &ReviewState {
        &self.review
    }

    pub const fn is_too_large(&self) -> bool {
        self.state.too_large.is_some()
    }

    pub fn refresh(&mut self, files: &mut FilesPanel, store: &mut DiffStore, repo: &Repository) {
        files.ensure_rows(store);
        let Some(file) = files.selected_file(store) else {
            self.current_key = None;
            return;
        };
        let path = file.path.clone();
        let status = file.status;
        let cache_key = FileKey {
            path: path.clone(),
            status,
        };

        if self.state.too_large.is_none()
            && let Ok(n) = repository::file_diff_line_count(repo, &path, status)
            && n > repository::DIFF_LINE_THRESHOLD
        {
            self.state.too_large = Some(n);
            self.state.is_binary = false;
            self.state.line_index = LineIndex::new(&[]);
            self.current_key = Some(cache_key.clone());

            if let Some(&slot_idx) = store.review_doc.by_key.get(&cache_key)
                && !matches!(
                    store.review_doc.files[slot_idx].load,
                    DiffLoadState::Loaded { .. }
                )
            {
                store.review_doc.files[slot_idx].load = DiffLoadState::TooLarge { lines: n };
                store.review_doc.index_dirty = true;
            }
            return;
        }

        self.state.too_large = None;

        if let Some(&slot_idx) = store.review_doc.by_key.get(&cache_key) {
            if matches!(
                store.review_doc.files[slot_idx].load,
                DiffLoadState::NotLoaded
            ) {
                match repository::file_diff(repo, &path, status) {
                    Ok(Some(sections)) => {
                        let hunks = sections.iter().flat_map(|s| s.hunks.clone()).collect();
                        let index = LineIndex::new(&sections);
                        store.review_doc.files[slot_idx].load = DiffLoadState::Loaded {
                            sections,
                            hunks,
                            index,
                        };
                    }
                    Ok(None) => {
                        store.review_doc.files[slot_idx].load = DiffLoadState::Binary;
                    }
                    Err(_) => {}
                }
            }

            let is_binary = matches!(store.review_doc.files[slot_idx].load, DiffLoadState::Binary);
            let (new_line_index, hunk_count) = match &store.review_doc.files[slot_idx].load {
                DiffLoadState::Loaded {
                    sections, hunks, ..
                } => (LineIndex::new(sections), hunks.len()),
                _ => (LineIndex::new(&[]), 0),
            };

            self.state.is_binary = is_binary;
            self.state.line_index = new_line_index;
            self.state.select_first_hunk(hunk_count);
            store.review_doc.index_dirty = true;
        }

        self.current_key = Some(cache_key);
        self.sync_scroll_to_hunk();
    }

    pub fn jump_to_selected_file(&mut self, files: &FilesPanel, store: &DiffStore) {
        if let Some(file) = files.selected_file(store) {
            self.jump_to_file(file, store);
        }
    }

    pub fn reset(&mut self) {
        self.current_key = None;
        self.state.too_large = None;
        self.state.line_index = LineIndex::new(&[]);
        self.state.select_first_hunk(0);
    }

    pub fn diff_hunks<'a>(&self, store: &'a DiffStore) -> Option<&'a Vec<DiffHunk>> {
        let key = self.current_key.as_ref()?;
        let slot_idx = store.review_doc.by_key.get(key)?;
        let slot = store.review_doc.files.get(*slot_idx)?;
        match &slot.load {
            DiffLoadState::Loaded { hunks, .. } => Some(hunks),
            _ => None,
        }
    }

    pub fn set_viewport_height(&mut self, height: usize) {
        let clamped = height.max(1);
        if clamped == self.state.viewport_height {
            return;
        }
        self.state.set_viewport_height(height);
        let offset = self.state.scroll_offset.min(self.max_scroll_offset());
        self.state.set_scroll_offset(offset);
        self.sync_selection_to_scroll();
    }

    pub fn scroll_down(&mut self) {
        let max_offset = self.max_scroll_offset();
        let offset = (self.state.scroll_offset + SCROLL_STEP).min(max_offset);
        self.state.set_scroll_offset(offset);
        self.sync_selection_to_scroll();
    }

    pub fn scroll_up(&mut self) {
        let offset = self.state.scroll_offset.saturating_sub(SCROLL_STEP);
        self.state.set_scroll_offset(offset);
        self.clamp_scroll();
        self.sync_selection_to_scroll();
    }

    pub fn continuous_scroll_down(&mut self, store: &DiffStore) {
        let max_offset = self.max_continuous_scroll_offset(store);
        let offset = (self.review.continuous_scroll + SCROLL_STEP).min(max_offset);
        self.review.continuous_scroll = offset;
    }

    pub fn continuous_scroll_up(&mut self) {
        self.review.continuous_scroll = self.review.continuous_scroll.saturating_sub(SCROLL_STEP);
    }

    pub fn select_next_hunk(&mut self, store: &DiffStore) {
        match self.review.mode {
            ViewMode::Continuous => self.next_continuous_hunk(store),
            ViewMode::SingleFile => {
                let len = self.diff_hunks(store).map_or(0, Vec::len);
                self.state.select_next_hunk(len);
                self.sync_scroll_to_hunk();
            }
        }
    }

    pub fn select_previous_hunk(&mut self, store: &DiffStore) {
        match self.review.mode {
            ViewMode::Continuous => self.prev_continuous_hunk(store),
            ViewMode::SingleFile => {
                let len = self.diff_hunks(store).map_or(0, Vec::len);
                self.state.select_previous_hunk(len);
                self.sync_scroll_to_hunk();
            }
        }
    }

    pub fn jump_to_file(&mut self, file: &FileEntry, store: &DiffStore) {
        let key = FileKey {
            path: file.path.clone(),
            status: file.status,
        };
        let Some(&file_idx) = store.review_doc.by_key.get(&key) else {
            return;
        };
        self.review.continuous_scroll = store.review_doc.index.file_starts[file_idx];
    }

    pub fn sync_scroll_to_hunk(&mut self) {
        let offset = self
            .state
            .selected_hunk
            .and_then(|hunk_idx| self.state.line_index.hunk_starts.get(hunk_idx).copied())
            .unwrap_or(0);
        self.clamp_scroll();
        self.state.set_scroll_offset(offset);
    }

    pub fn sync_selection_to_scroll(&mut self) {
        let row = self.state.line_index.lookup(self.state.scroll_offset);
        match row {
            Some(IndexRow::HunkHeader(hunk_idx)) => {
                self.state.select_hunk_line(hunk_idx, 0);
            }
            Some(IndexRow::DiffLine(hunk_idx, line_idx)) => {
                self.state.select_hunk_line(hunk_idx, line_idx);
            }
            _ => {
                self.state.select_first_hunk(0);
            }
        }
    }

    pub fn sync_continuous_scroll_to_file(&mut self, file_idx: Option<usize>, store: &DiffStore) {
        if let Some(file_idx) = file_idx
            && let Some(&row) = store.review_doc.index.file_starts.get(file_idx)
        {
            self.review.continuous_scroll = row;
            return;
        }
        self.review.continuous_scroll = self
            .review
            .continuous_scroll
            .min(self.max_continuous_scroll_offset(store));
    }

    pub fn current_continuous_file_idx(&self, store: &DiffStore) -> Option<usize> {
        store
            .review_doc
            .index
            .file_at_row(self.review.continuous_scroll)
            .map(|(file_idx, _)| file_idx)
    }

    #[cfg(test)]
    pub fn continuous_scroll(&self) -> usize {
        self.review.continuous_scroll
    }

    #[cfg(test)]
    pub fn set_continuous_scroll(&mut self, scroll: usize) {
        self.review.continuous_scroll = scroll;
    }

    fn sync_files_to_scroll(&mut self, files: &mut FilesPanel, store: &DiffStore) {
        files.match_selected_file(store, self.review.continuous_scroll);
    }

    fn next_continuous_hunk(&mut self, store: &DiffStore) {
        let current = self.review.continuous_scroll;
        let rows = self.continuous_hunk_rows(store);
        if let Some(&row) = rows.iter().find(|&&r| r > current) {
            self.review.continuous_scroll = row;
        }
    }

    fn prev_continuous_hunk(&mut self, store: &DiffStore) {
        let current = self.review.continuous_scroll;
        let rows = self.continuous_hunk_rows(store);
        if let Some(&row) = rows.iter().rfind(|&&r| r < current) {
            self.review.continuous_scroll = row;
        }
    }

    fn continuous_hunk_rows(&self, store: &DiffStore) -> Vec<usize> {
        store
            .review_doc
            .files
            .iter()
            .enumerate()
            .flat_map(|(file_idx, slot)| {
                let file_start = store
                    .review_doc
                    .index
                    .file_starts
                    .get(file_idx)
                    .copied()
                    .unwrap_or(0);
                if let DiffLoadState::Loaded { index, .. } = &slot.load {
                    index
                        .hunk_starts
                        .iter()
                        .map(move |&h| file_start + 1 + h)
                        .collect::<Vec<_>>()
                } else {
                    vec![]
                }
            })
            .collect()
    }

    fn clamp_scroll(&mut self) {
        let max = self.max_scroll_offset();
        self.state.scroll_offset = self.state.scroll_offset.min(max);
    }

    fn max_scroll_offset(&self) -> usize {
        self.state
            .line_index
            .total_rows
            .saturating_sub(self.state.viewport_height)
    }

    fn max_continuous_scroll_offset(&self, store: &DiffStore) -> usize {
        store
            .review_doc
            .index
            .total_rows
            .saturating_sub(self.state.viewport_height)
    }
}
