use git2::Repository;

use crate::action::Action;
use crate::error::AppResult;
use crate::files_panel::FilesPanel;
use crate::git::repository::{self, DiffHunk, DiffSource, FileEntry};
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

pub struct DiffContext<'a> {
    pub files: &'a mut FilesPanel,
    pub store: &'a mut DiffStore,
    pub repo: &'a Repository,
    pub diff_source: &'a DiffSource,
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
        diff_ctx: &mut DiffContext,
    ) {
        if selection_changed {
            self.refresh(diff_ctx);
            if self.review.mode == ViewMode::Continuous {
                self.jump_to_selected_file(diff_ctx.files, diff_ctx.store);
            }
        }

        match action {
            Action::MoveDown if focus == Focus::Diff => match self.review.mode {
                ViewMode::Continuous => {
                    self.continuous_scroll_down(diff_ctx.store);
                    self.sync_files_to_scroll(diff_ctx.files, diff_ctx.store);
                }
                ViewMode::SingleFile => self.scroll_down(),
            },
            Action::MoveUp if focus == Focus::Diff => match self.review.mode {
                ViewMode::Continuous => {
                    self.continuous_scroll_up();
                    self.sync_files_to_scroll(diff_ctx.files, diff_ctx.store);
                }
                ViewMode::SingleFile => self.scroll_up(),
            },
            Action::NextHunk => {
                self.select_next_hunk(diff_ctx.store);
                if self.review.mode == ViewMode::Continuous {
                    self.sync_files_to_scroll(diff_ctx.files, diff_ctx.store);
                }
            }
            Action::PreviousHunk => {
                self.select_previous_hunk(diff_ctx.store);
                if self.review.mode == ViewMode::Continuous {
                    self.sync_files_to_scroll(diff_ctx.files, diff_ctx.store);
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
                self.refresh(diff_ctx);
            }
            _ => {}
        }
    }

    pub fn reload(&mut self, diff_ctx: &mut DiffContext) -> AppResult<()> {
        let selected_key = diff_ctx
            .files
            .selected_file(diff_ctx.store)
            .map(|file| FileKey {
                path: file.path.clone(),
                status: file.status,
            });

        let operation = match diff_ctx.diff_source {
            DiffSource::Worktree => "refresh working-tree changes",
            DiffSource::Revision(_) => "refresh revision changes",
        };
        let entries = repository::files_for_source(diff_ctx.repo, diff_ctx.diff_source)
            .map_err(|error| error.with_operation(operation))?;

        diff_ctx.store.reload(entries);
        diff_ctx.files.mark_dirty();
        diff_ctx.files.ensure_rows(diff_ctx.store);
        let restored_file_idx = diff_ctx
            .files
            .restore_selection(diff_ctx.store, selected_key);
        self.sync_continuous_scroll_to_file(restored_file_idx, diff_ctx.store);
        self.reset();

        diff_ctx.store.spawn_workers(diff_ctx.diff_source);

        self.refresh(diff_ctx);
        Ok(())
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

    pub fn refresh(&mut self, diff_ctx: &mut DiffContext) {
        diff_ctx.files.ensure_rows(diff_ctx.store);
        let Some(file) = diff_ctx.files.selected_file(diff_ctx.store) else {
            self.current_key = None;
            return;
        };
        let path = file.path.clone();
        let status = file.status;
        let cache_key = FileKey {
            path: path.clone(),
            status,
        };

        let line_count = match diff_ctx.diff_source {
            DiffSource::Worktree => {
                repository::file_diff_line_count(diff_ctx.repo, &path, status).ok()
            }
            DiffSource::Revision(_) => None,
        };

        if self.state.too_large.is_none()
            && let Some(n) = line_count
            && n > repository::DIFF_LINE_THRESHOLD
        {
            self.state.too_large = Some(n);
            self.state.is_binary = false;
            self.state.line_index = LineIndex::new(&[]);
            self.current_key = Some(cache_key.clone());

            if let Some(&slot_idx) = diff_ctx.store.continuous_diff.by_key.get(&cache_key)
                && !matches!(
                    diff_ctx.store.continuous_diff.files[slot_idx].load,
                    DiffLoadState::Loaded { .. }
                )
            {
                diff_ctx.store.continuous_diff.files[slot_idx].load =
                    DiffLoadState::TooLarge { lines: n };
                diff_ctx.store.continuous_diff.index_dirty = true;
            }
            return;
        }

        self.state.too_large = None;

        if let Some(&slot_idx) = diff_ctx.store.continuous_diff.by_key.get(&cache_key) {
            if matches!(
                diff_ctx.store.continuous_diff.files[slot_idx].load,
                DiffLoadState::NotLoaded
            ) {
                let result = repository::file_diff_for_source(
                    diff_ctx.repo,
                    diff_ctx.diff_source,
                    &path,
                    status,
                );

                match result {
                    Ok(Some(sections)) => {
                        let hunks = sections.iter().flat_map(|s| s.hunks.clone()).collect();
                        let index = LineIndex::new(&sections);
                        diff_ctx.store.continuous_diff.files[slot_idx].load =
                            DiffLoadState::Loaded {
                                sections,
                                hunks,
                                index,
                            };
                    }
                    Ok(None) => {
                        diff_ctx.store.continuous_diff.files[slot_idx].load = DiffLoadState::Binary;
                    }
                    Err(_) => {}
                }
            }

            let is_binary = matches!(
                diff_ctx.store.continuous_diff.files[slot_idx].load,
                DiffLoadState::Binary
            );
            let (new_line_index, hunk_count) =
                match &diff_ctx.store.continuous_diff.files[slot_idx].load {
                    DiffLoadState::Loaded {
                        sections, hunks, ..
                    } => (LineIndex::new(sections), hunks.len()),
                    _ => (LineIndex::new(&[]), 0),
                };

            self.state.is_binary = is_binary;
            self.state.line_index = new_line_index;
            self.state.select_first_hunk(hunk_count);
            diff_ctx.store.continuous_diff.index_dirty = true;
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
        let slot_idx = store.continuous_diff.by_key.get(key)?;
        let slot = store.continuous_diff.files.get(*slot_idx)?;
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
        let Some(&file_idx) = store.continuous_diff.by_key.get(&key) else {
            return;
        };
        self.review.continuous_scroll = store.continuous_diff.index.file_starts[file_idx];
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
            && let Some(&row) = store.continuous_diff.index.file_starts.get(file_idx)
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
            .continuous_diff
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
            .continuous_diff
            .files
            .iter()
            .enumerate()
            .flat_map(|(file_idx, slot)| {
                let file_start = store
                    .continuous_diff
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
            .continuous_diff
            .index
            .total_rows
            .saturating_sub(self.state.viewport_height)
    }
}
