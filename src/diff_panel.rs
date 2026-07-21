use git2::Repository;
use unicode_width::UnicodeWidthChar;

use crate::action::Action;
use crate::error::AppResult;
use crate::files_panel::FilesPanel;
use crate::git::repository::{self, DiffSource, FileEntry};
use crate::state::review::RenderedRow;
use crate::state::review::ReviewState;
use crate::state::{Diff, DiffLayout, DiffLoadState, FileKey, Focus, LineIndex};
use crate::store::DiffStore;

const SCROLL_STEP: usize = 1;
const HORIZONTAL_SCROLL_STEP: usize = 4;
const GUTTER_WIDTH: usize = 1;

pub struct DiffPanel {
    state: Diff,
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
            self.jump_to_selected_file(diff_ctx.files, diff_ctx.store);
        }

        match action {
            Action::MoveDown if focus == Focus::Diff => {
                self.continuous_scroll_down(diff_ctx.store);
                self.sync_files_to_scroll(diff_ctx.files, diff_ctx.store);
            }
            Action::MoveUp if focus == Focus::Diff => {
                self.continuous_scroll_up();
                self.sync_files_to_scroll(diff_ctx.files, diff_ctx.store);
            }
            Action::ScrollDiffLeft => {
                self.state.scroll_left(HORIZONTAL_SCROLL_STEP);
            }
            Action::ScrollDiffRight => {
                self.state.scroll_right(HORIZONTAL_SCROLL_STEP);
            }
            Action::NextHunk => {
                self.select_next_hunk(diff_ctx.store);
                self.sync_files_to_scroll(diff_ctx.files, diff_ctx.store);
            }
            Action::PreviousHunk => {
                self.select_previous_hunk(diff_ctx.store);
                self.sync_files_to_scroll(diff_ctx.files, diff_ctx.store);
            }
            Action::ToggleDiffLineNumbers => {
                self.state.toggle_line_numbers();
                self.refresh_horizontal_scroll_bounds(diff_ctx.store);
            }
            Action::ToggleDiffLayout => {
                let layout = self
                    .state
                    .toggle_layout_override(diff_ctx.store.continuous_diff.layout);
                self.set_layout(layout, diff_ctx.store);
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

    pub fn refresh(&mut self, diff_ctx: &mut DiffContext) {
        diff_ctx.files.ensure_rows(diff_ctx.store);
        let Some(file) = diff_ctx.files.selected_file(diff_ctx.store) else {
            return;
        };
        let path = file.path.clone();
        let status = file.status;
        let cache_key = FileKey {
            path: path.clone(),
            status,
        };

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
                        let index = LineIndex::new(&sections);
                        let hunks = sections
                            .into_iter()
                            .flat_map(|section| section.hunks)
                            .collect();
                        diff_ctx.store.continuous_diff.files[slot_idx].load =
                            DiffLoadState::Loaded { hunks, index };
                    }
                    Ok(None) => {
                        diff_ctx.store.continuous_diff.files[slot_idx].load = DiffLoadState::Binary;
                    }
                    Err(_) => {}
                }
            }

            diff_ctx.store.continuous_diff.index_dirty = true;
            self.refresh_horizontal_scroll_bounds(diff_ctx.store);
        }
    }

    pub fn jump_to_selected_file(&mut self, files: &FilesPanel, store: &DiffStore) {
        if let Some(file) = files.selected_file(store) {
            self.jump_to_file(file, store);
        }
    }

    pub fn reset(&mut self) {
        self.state.set_max_horizontal_scroll(0);
    }

    pub fn set_viewport_height(&mut self, height: usize) {
        self.state.set_viewport_height(height);
    }

    pub fn set_viewport_width(&mut self, width: usize, store: &DiffStore) {
        if self.state.set_viewport_width(width) {
            self.refresh_horizontal_scroll_bounds(store);
        }
    }

    pub fn set_layout(&mut self, layout: DiffLayout, store: &mut DiffStore) {
        if store.continuous_diff.layout == layout {
            return;
        }

        let file_anchor = self.current_continuous_file_idx(store);
        let hunk_anchor = match store
            .continuous_diff
            .lookup_row(self.review.continuous_scroll)
        {
            Some(RenderedRow::HunkHeader { file_idx, hunk_idx })
            | Some(RenderedRow::DiffRow {
                file_idx, hunk_idx, ..
            }) => Some((file_idx, hunk_idx)),
            _ => None,
        };
        store.continuous_diff.layout = layout;
        store.continuous_diff.rebuild_index();

        if let Some((file_idx, hunk_idx)) = hunk_anchor
            && let Some(&file_start) = store.continuous_diff.index.file_starts.get(file_idx)
            && let DiffLoadState::Loaded { index, .. } = &store.continuous_diff.files[file_idx].load
            && let Some(&hunk_start) = index.hunk_starts_for(layout).get(hunk_idx)
        {
            self.review.continuous_scroll = file_start + 1 + hunk_start;
        } else {
            self.sync_continuous_scroll_to_file(file_anchor, store);
        }
        self.refresh_horizontal_scroll_bounds(store);
    }

    pub fn layout_for_width(&self, width: usize) -> DiffLayout {
        self.state.layout_for_width(width)
    }

    pub fn refresh_horizontal_scroll_bounds(&mut self, store: &DiffStore) {
        let max = self.max_horizontal_scroll(store);
        self.state.set_max_horizontal_scroll(max);
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
        self.next_continuous_hunk(store);
    }

    pub fn select_previous_hunk(&mut self, store: &DiffStore) {
        self.prev_continuous_hunk(store);
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
                        .hunk_starts_for(store.continuous_diff.layout)
                        .iter()
                        .map(move |&h| file_start + 1 + h)
                        .collect::<Vec<_>>()
                } else {
                    vec![]
                }
            })
            .collect()
    }

    fn max_continuous_scroll_offset(&self, store: &DiffStore) -> usize {
        store
            .continuous_diff
            .index
            .total_rows
            .saturating_sub(self.state.viewport_height)
    }

    fn max_horizontal_scroll(&self, store: &DiffStore) -> usize {
        let left_width = self.state.viewport_width / 2;
        let right_width = self.state.viewport_width.saturating_sub(left_width);
        let mut max_scroll = 0;

        for slot in &store.continuous_diff.files {
            let DiffLoadState::Loaded { hunks, .. } = &slot.load else {
                continue;
            };

            for hunk in hunks {
                match store.continuous_diff.layout {
                    DiffLayout::Unified => {
                        for line in &hunk.lines {
                            let prefix_width = unified_prefix_width(
                                line.old_lineno,
                                line.new_lineno,
                                self.state.show_line_numbers,
                            );
                            let available = self
                                .state
                                .viewport_width
                                .saturating_sub(GUTTER_WIDTH + prefix_width);
                            let overflow = display_width(&line.content).saturating_sub(available);
                            max_scroll = max_scroll.max(overflow);
                        }
                    }
                    DiffLayout::SideBySide => {
                        for row in &hunk.comparison_rows {
                            if let Some(line_idx) = row.old_line_idx {
                                let line = &hunk.lines[line_idx];
                                let prefix_width = split_prefix_width(
                                    line.old_lineno,
                                    self.state.show_line_numbers,
                                );
                                let available =
                                    left_width.saturating_sub(GUTTER_WIDTH + prefix_width);
                                let overflow =
                                    display_width(&line.content).saturating_sub(available);
                                max_scroll = max_scroll.max(overflow);
                            }
                            if let Some(line_idx) = row.new_line_idx {
                                let line = &hunk.lines[line_idx];
                                let prefix_width = split_prefix_width(
                                    line.new_lineno,
                                    self.state.show_line_numbers,
                                );
                                let available =
                                    right_width.saturating_sub(GUTTER_WIDTH + prefix_width);
                                let overflow =
                                    display_width(&line.content).saturating_sub(available);
                                max_scroll = max_scroll.max(overflow);
                            }
                        }
                    }
                }
            }
        }

        max_scroll
    }
}

fn unified_prefix_width(
    old_lineno: Option<u32>,
    new_lineno: Option<u32>,
    show_line_numbers: bool,
) -> usize {
    if show_line_numbers {
        line_number_width(old_lineno) + line_number_width(new_lineno) + 4
    } else {
        2
    }
}

fn split_prefix_width(line_number: Option<u32>, show_line_numbers: bool) -> usize {
    if show_line_numbers {
        line_number_width(line_number) + 3
    } else {
        2
    }
}

fn line_number_width(line_number: Option<u32>) -> usize {
    line_number.map_or(4, |number| number.to_string().len().max(4))
}

fn display_width(content: &str) -> usize {
    content
        .trim_end()
        .chars()
        .map(|ch| {
            if ch == '\t' {
                4
            } else {
                ch.width().unwrap_or(0)
            }
        })
        .sum()
}
