use git2::Repository;
use unicode_width::UnicodeWidthChar;

use crate::action::{Action, ScrollDirection};
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
const VERTICAL_SCROLL_MARGIN: usize = 2;

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
    pub fn new(tab_width: usize) -> Self {
        Self {
            state: Diff::new(tab_width),
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
                self.select_next_line(diff_ctx.store);
                self.sync_files_to_selection(diff_ctx.files, diff_ctx.store);
            }
            Action::MoveUp if focus == Focus::Diff => {
                self.select_previous_line(diff_ctx.store);
                self.sync_files_to_selection(diff_ctx.files, diff_ctx.store);
            }
            Action::ScrollDiffLeft => {
                self.state.scroll_left(HORIZONTAL_SCROLL_STEP);
            }
            Action::ScrollDiffRight => {
                self.state.scroll_right(HORIZONTAL_SCROLL_STEP);
            }
            Action::NextHunk => {
                self.select_next_hunk(diff_ctx.store);
                self.sync_files_to_selection(diff_ctx.files, diff_ctx.store);
            }
            Action::PreviousHunk => {
                self.select_previous_hunk(diff_ctx.store);
                self.sync_files_to_selection(diff_ctx.files, diff_ctx.store);
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
            Action::ScrollDiff { direction, lines } => {
                self.continuous_scroll_by(direction, lines, diff_ctx.store);
                self.keep_selection_in_view(diff_ctx.store);
                self.sync_files_to_selection(diff_ctx.files, diff_ctx.store);
            }
            Action::ToggleReviewed => {
                if let Some(file_idx) = self.current_continuous_file_idx(diff_ctx.store) {
                    if diff_ctx.store.continuous_diff.files[file_idx].ignored {
                        return;
                    }
                    let will_be_reviewed = !diff_ctx.store.continuous_diff.files[file_idx].reviewed;
                    diff_ctx.store.continuous_diff.toggle_reviewed(file_idx);
                    diff_ctx.store.continuous_diff.rebuild_index();
                    diff_ctx.store.continuous_diff.index_dirty = false;

                    let anchor = if will_be_reviewed {
                        diff_ctx
                            .store
                            .continuous_diff
                            .next_unreviewed_after(file_idx)
                            .or(Some(file_idx))
                    } else {
                        Some(file_idx)
                    };
                    self.sync_continuous_scroll_to_file(anchor, diff_ctx.store);
                    self.sync_files_to_selection(diff_ctx.files, diff_ctx.store);
                }
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
            DiffSource::Range(_) => "refresh revision range",
        };
        let ignore_whitespace = diff_ctx.store.ignore_whitespace();
        let entries =
            repository::files_for_source(diff_ctx.repo, diff_ctx.diff_source, ignore_whitespace)
                .map_err(|error| error.with_operation(operation))?;

        self.replace_entries(diff_ctx, entries, selected_key);
        Ok(())
    }

    pub fn load_source(&mut self, diff_ctx: &mut DiffContext, entries: Vec<FileEntry>) {
        let selected_key = diff_ctx
            .files
            .selected_file(diff_ctx.store)
            .map(|file| FileKey {
                path: file.path.clone(),
                status: file.status,
            });

        self.replace_entries(diff_ctx, entries, selected_key);
    }

    fn replace_entries(
        &mut self,
        diff_ctx: &mut DiffContext,
        entries: Vec<FileEntry>,
        selected_key: Option<FileKey>,
    ) {
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
        let previous_path = file.previous_path.clone();
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
                    previous_path.as_deref(),
                    status,
                    diff_ctx.store.ignore_whitespace(),
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
        let hunk_anchor = match store.continuous_diff.lookup_row(self.review.selected_row) {
            Some(RenderedRow::HunkHeader { file_idx, hunk_idx }) => {
                Some((file_idx, hunk_idx, None))
            }
            Some(RenderedRow::DiffRow {
                file_idx,
                hunk_idx,
                row_idx,
            }) => Some((file_idx, hunk_idx, Some(row_idx))),
            _ => None,
        };
        store.continuous_diff.layout = layout;
        store.continuous_diff.rebuild_index();

        if let Some((file_idx, hunk_idx, row_idx)) = hunk_anchor
            && let Some(&file_start) = store.continuous_diff.index.file_starts.get(file_idx)
            && let DiffLoadState::Loaded { hunks, index } =
                &store.continuous_diff.files[file_idx].load
            && let Some(&hunk_start) = index.hunk_starts_for(layout).get(hunk_idx)
        {
            let hunk_header = file_start + 1 + hunk_start;
            let row_count = match layout {
                DiffLayout::Unified => hunks[hunk_idx].lines.len(),
                DiffLayout::SideBySide => hunks[hunk_idx].comparison_rows.len(),
            };
            let row_offset = row_idx
                .filter(|_| row_count > 0)
                .map_or(0, |row_idx| 1 + row_idx.min(row_count - 1));
            self.review.selected_row = hunk_header + row_offset;
            self.review.continuous_scroll = self.review.selected_row;
            self.ensure_selection_visible(store);
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

    fn continuous_scroll_by(
        &mut self,
        direction: ScrollDirection,
        lines: usize,
        store: &DiffStore,
    ) {
        let distance = lines.saturating_mul(SCROLL_STEP);
        self.review.continuous_scroll = match direction {
            ScrollDirection::Down => self
                .review
                .continuous_scroll
                .saturating_add(distance)
                .min(self.max_continuous_scroll_offset(store)),
            ScrollDirection::Up => self.review.continuous_scroll.saturating_sub(distance),
        };
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
        self.set_position_to_file(file_idx, store);
    }

    pub fn sync_continuous_scroll_to_file(&mut self, file_idx: Option<usize>, store: &DiffStore) {
        if let Some(file_idx) = file_idx
            && file_idx < store.continuous_diff.files.len()
        {
            self.set_position_to_file(file_idx, store);
            return;
        }
        let max_offset = self.max_continuous_scroll_offset(store);
        self.review.continuous_scroll = self.review.continuous_scroll.min(max_offset);
        self.review.selected_row = self.review.selected_row.min(max_offset);
    }

    pub fn current_continuous_file_idx(&self, store: &DiffStore) -> Option<usize> {
        store
            .continuous_diff
            .index
            .file_at_row(self.review.selected_row)
            .map(|(file_idx, _)| file_idx)
    }

    pub fn select_next_line(&mut self, store: &DiffStore) {
        if let Some(row) = Self::next_selectable_row(store, self.review.selected_row) {
            self.review.selected_row = row;
            self.ensure_selection_visible(store);
        }
    }

    pub fn select_previous_line(&mut self, store: &DiffStore) {
        if let Some(row) = Self::previous_selectable_row(store, self.review.selected_row) {
            self.review.selected_row = row;
            self.ensure_selection_visible(store);
        }
    }

    fn ensure_selection_visible(&mut self, store: &DiffStore) {
        // Reserve one row for the pinned file header.
        let visible_height = self.state.viewport_height.saturating_sub(1).max(1);
        let margin = VERTICAL_SCROLL_MARGIN.min(visible_height.saturating_sub(1) / 2);
        let selected = self.review.selected_row;
        let top = self.review.continuous_scroll;
        let bottom = top.saturating_add(visible_height - 1);
        let upper_edge = top.saturating_add(margin);
        let lower_edge = bottom.saturating_sub(margin);

        if selected < upper_edge {
            self.review.continuous_scroll = selected.saturating_sub(margin);
        } else if selected > lower_edge {
            self.review.continuous_scroll = selected.saturating_sub(visible_height - 1 - margin);
        }

        let max_scroll = store
            .continuous_diff
            .index
            .total_rows
            .saturating_sub(visible_height);

        self.review.continuous_scroll = self.review.continuous_scroll.min(max_scroll);
    }

    fn keep_selection_in_view(&mut self, store: &DiffStore) {
        let visible_height = self.state.viewport_height.saturating_sub(1).max(1);
        let top = self.review.continuous_scroll;
        let bottom = top.saturating_add(visible_height - 1);

        if self.review.selected_row < top {
            if let Some(row) = (top..=bottom).find(|&row| Self::is_selectable_row(store, row)) {
                self.review.selected_row = row;
            }
        } else if self.review.selected_row > bottom
            && let Some(row) = (top..=bottom)
                .rev()
                .find(|&row| Self::is_selectable_row(store, row))
        {
            self.review.selected_row = row;
        }
    }

    fn is_selectable_row(store: &DiffStore, row: usize) -> bool {
        matches!(
            store.continuous_diff.lookup_row(row),
            Some(RenderedRow::DiffRow { .. })
        )
    }

    fn next_selectable_row(store: &DiffStore, current: usize) -> Option<usize> {
        let start = current.saturating_add(1);
        (start..store.continuous_diff.index.total_rows)
            .find(|&row| Self::is_selectable_row(store, row))
    }

    fn previous_selectable_row(store: &DiffStore, current: usize) -> Option<usize> {
        (0..current)
            .rev()
            .find(|&row| Self::is_selectable_row(store, row))
    }

    #[cfg(test)]
    pub fn continuous_scroll(&self) -> usize {
        self.review.continuous_scroll
    }

    #[cfg(test)]
    pub fn set_continuous_scroll(&mut self, scroll: usize) {
        self.review.continuous_scroll = scroll;
        self.review.selected_row = scroll;
    }

    fn sync_files_to_selection(&mut self, files: &mut FilesPanel, store: &DiffStore) {
        files.match_selected_file(store, self.review.selected_row);
    }

    fn next_continuous_hunk(&mut self, store: &DiffStore) {
        let current = self
            .selected_hunk_header_row(store)
            .unwrap_or(self.review.selected_row);
        let rows = self.continuous_hunk_rows(store);
        if let Some(&row) = rows.iter().find(|&&r| r > current) {
            self.select_hunk_at_row(row, store);
        }
    }

    fn prev_continuous_hunk(&mut self, store: &DiffStore) {
        let current = self
            .selected_hunk_header_row(store)
            .unwrap_or(self.review.selected_row);
        let rows = self.continuous_hunk_rows(store);
        if let Some(&row) = rows.iter().rfind(|&&r| r < current) {
            self.select_hunk_at_row(row, store);
        }
    }

    fn continuous_hunk_rows(&self, store: &DiffStore) -> Vec<usize> {
        store
            .continuous_diff
            .files
            .iter()
            .enumerate()
            .flat_map(|(file_idx, slot)| {
                if slot.reviewed {
                    return vec![];
                }
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

    fn selected_hunk_header_row(&self, store: &DiffStore) -> Option<usize> {
        let (file_idx, hunk_idx) =
            match store.continuous_diff.lookup_row(self.review.selected_row)? {
                RenderedRow::HunkHeader { file_idx, hunk_idx }
                | RenderedRow::DiffRow {
                    file_idx, hunk_idx, ..
                } => (file_idx, hunk_idx),
                _ => return None,
            };
        let file_start = *store.continuous_diff.index.file_starts.get(file_idx)?;
        let DiffLoadState::Loaded { index, .. } = &store.continuous_diff.files[file_idx].load
        else {
            return None;
        };
        let hunk_start = *index
            .hunk_starts_for(store.continuous_diff.layout)
            .get(hunk_idx)?;
        Some(file_start + 1 + hunk_start)
    }

    fn select_hunk_at_row(&mut self, hunk_header: usize, store: &DiffStore) {
        self.review.continuous_scroll = hunk_header;
        self.review.selected_row = hunk_header
            .checked_add(1)
            .filter(|&row| Self::is_selectable_row(store, row))
            .unwrap_or(hunk_header);
    }

    fn set_position_to_file(&mut self, file_idx: usize, store: &DiffStore) {
        let file_start = store.continuous_diff.index.file_starts[file_idx];
        let file_end = store
            .continuous_diff
            .index
            .file_starts
            .get(file_idx + 1)
            .copied()
            .unwrap_or(store.continuous_diff.index.total_rows);
        let selected_row = (file_start..file_end)
            .find(|&row| Self::is_selectable_row(store, row))
            .unwrap_or(file_start);

        self.review.continuous_scroll = file_start;
        self.review.selected_row = selected_row;
    }

    fn max_continuous_scroll_offset(&self, store: &DiffStore) -> usize {
        store.continuous_diff.index.total_rows.saturating_sub(1)
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
                            let overflow = display_width(&line.content, self.state.tab_width)
                                .saturating_sub(available);
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
                                let overflow = display_width(&line.content, self.state.tab_width)
                                    .saturating_sub(available);
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
                                let overflow = display_width(&line.content, self.state.tab_width)
                                    .saturating_sub(available);
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

fn display_width(content: &str, tab_width: usize) -> usize {
    content
        .trim_end()
        .chars()
        .map(|ch| {
            if ch == '\t' {
                tab_width
            } else {
                ch.width().unwrap_or(0)
            }
        })
        .sum()
}

#[cfg(test)]
mod tests {
    use crate::{
        config::DEFAULT_TAB_WIDTH,
        git::repository::{
            DiffHunk, DiffLine, DiffSection, DiffSectionKind, FileChange, FileStatus,
        },
    };

    use super::*;

    fn store_with_hunks(line_counts: &[usize]) -> DiffStore {
        let total_lines = line_counts.iter().sum();
        let entry = FileEntry {
            path: "src/main.rs".to_string(),
            previous_path: None,
            status: FileStatus::Unstaged,
            change: Some(FileChange::Modified),
            type_change: None,
            insertions: total_lines,
            deletions: 0,
        };
        let mut next_line = 1;
        let hunks: Vec<DiffHunk> = line_counts
            .iter()
            .map(|&line_count| {
                let lines = (0..line_count)
                    .map(|_| {
                        let line = DiffLine {
                            old_lineno: None,
                            new_lineno: Some(next_line),
                            origin: '+',
                            content: format!("line {next_line}\n"),
                        };
                        next_line += 1;
                        line
                    })
                    .collect();
                DiffHunk::new("@@ hunk @@\n".to_string(), lines)
            })
            .collect();
        let sections = vec![DiffSection {
            kind: DiffSectionKind::Unstaged,
            hunks: hunks.clone(),
        }];
        let index = LineIndex::new(&sections);
        let mut store = DiffStore::new(vec![entry], Vec::new(), false);
        store.continuous_diff.files[0].load = DiffLoadState::Loaded { hunks, index };
        store.continuous_diff.rebuild_index();
        store
    }

    fn store_with_lines(line_count: usize) -> DiffStore {
        store_with_hunks(&[line_count])
    }

    #[test]
    fn line_navigation_skips_headers_and_scrolls_near_the_viewport_edge() {
        let store = store_with_lines(6);
        let mut panel = DiffPanel::new(DEFAULT_TAB_WIDTH);
        panel.set_viewport_height(8);
        panel.sync_continuous_scroll_to_file(Some(0), &store);

        assert!(matches!(
            store
                .continuous_diff
                .lookup_row(panel.review().selected_row),
            Some(RenderedRow::DiffRow { row_idx: 0, .. })
        ));
        assert_eq!(panel.review().continuous_scroll, 0);

        let first_row = panel.review().selected_row;
        panel.select_next_line(&store);
        assert_eq!(panel.review().selected_row, first_row + 1);
        assert_eq!(panel.review().continuous_scroll, 0);

        panel.select_next_line(&store);
        assert_eq!(panel.review().selected_row, first_row + 2);
        assert_eq!(panel.review().continuous_scroll, 0);

        panel.select_next_line(&store);
        assert_eq!(panel.review().selected_row, first_row + 3);
        assert_eq!(panel.review().continuous_scroll, 1);

        panel.select_previous_line(&store);
        assert_eq!(panel.review().selected_row, first_row + 2);
    }

    #[test]
    fn hunk_navigation_anchors_the_hunk_header_at_the_viewport_top() {
        let store = store_with_hunks(&[2, 2]);
        let mut panel = DiffPanel::new(DEFAULT_TAB_WIDTH);
        panel.set_viewport_height(8);
        panel.sync_continuous_scroll_to_file(Some(0), &store);

        panel.select_next_hunk(&store);

        assert!(matches!(
            store
                .continuous_diff
                .lookup_row(panel.review().continuous_scroll),
            Some(RenderedRow::HunkHeader { hunk_idx: 1, .. })
        ));
        assert!(matches!(
            store
                .continuous_diff
                .lookup_row(panel.review().selected_row),
            Some(RenderedRow::DiffRow {
                hunk_idx: 1,
                row_idx: 0,
                ..
            })
        ));
    }
}
