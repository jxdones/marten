use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use crossterm::{execute, terminal::SetTitle};
use git2::Repository;

use crate::action::Action;
use crate::event::Event;
use crate::git::repository::{self, DiffHunk, DiffSection, FileEntry, RepositoryStatus};
use crate::state::line_index::IndexRow;
use crate::state::{
    Diff, Files, Focus, Screen,
    tree::{TreeRow, tree_rows},
};
use crate::state::{
    DiffLoadState, FileKey, FileSlot, LineIndex, ReviewDoc, ReviewIndex, ReviewState, ViewMode,
    WorkerResult,
};
use crate::tui::theme::{self, Theme};

const SCROLL_STEP: usize = 1;

pub struct FilesPanel {
    state: Files,
    cached_rows: Vec<TreeRow>,
    collapsed: HashSet<String>,
    dirty: bool,
}

pub struct DiffPanel {
    state: Diff,
    current_key: Option<FileKey>,
}

pub struct App {
    screen: Screen,
    focus: Focus,
    theme: Theme,
    repo: Repository,
    repository_status: Option<RepositoryStatus>,
    should_quit: bool,

    files: FilesPanel,
    diff: DiffPanel,
    review: ReviewState,
    review_doc: ReviewDoc,

    worker_rx: std::sync::mpsc::Receiver<WorkerResult>,
    worker_tx: std::sync::mpsc::Sender<WorkerResult>,
}

impl App {
    pub fn new() -> Self {
        // set app title
        execute!(std::io::stdout(), SetTitle("marten")).ok();

        let repo = Repository::discover(".").expect("not a git repo");
        let repository_status = repository::status(&repo).ok();
        let files = repository::files(&repo).ok().map(|mut f| {
            Self::sort_file_entries(&mut f);
            f
        });

        let review_doc = Self::build_review_doc(files.unwrap_or_default(), 0);

        let (tx, rx) = std::sync::mpsc::channel::<WorkerResult>();

        let mut app = Self {
            screen: Screen::Home,
            focus: Focus::Files,
            files: FilesPanel {
                state: Files::default(),
                cached_rows: Vec::new(),
                collapsed: HashSet::new(),
                dirty: true,
            },
            diff: DiffPanel {
                state: Diff::default(),
                current_key: None,
            },
            review_doc,
            review: ReviewState::default(),
            worker_rx: rx,
            worker_tx: tx,
            theme: theme::DEFAULT,
            repo,
            should_quit: false,
            repository_status,
        };

        app.spawn_diff_workers();

        app.review_doc.rebuild_index();
        app.select_first_file();
        app.ensure_rows();
        app.refresh_diff();
        app
    }

    pub const fn screen(&self) -> Screen {
        self.screen
    }

    pub const fn focus(&self) -> Focus {
        self.focus
    }

    pub const fn files_state(&self) -> &Files {
        &self.files.state
    }

    pub const fn theme(&self) -> Theme {
        self.theme
    }

    pub const fn should_quit(&self) -> bool {
        self.should_quit
    }

    pub const fn repository_status(&self) -> Option<&RepositoryStatus> {
        self.repository_status.as_ref()
    }

    pub fn files(&self) -> &[FileSlot] {
        &self.review_doc.files
    }

    pub const fn diff_state(&self) -> &Diff {
        &self.diff.state
    }

    pub fn diff_hunks(&self) -> Option<&Vec<DiffHunk>> {
        let key = self.diff.current_key.as_ref()?;
        let slot_idx = self.review_doc.by_key.get(key)?;
        let slot = self.review_doc.files.get(*slot_idx)?;
        match &slot.load {
            DiffLoadState::Loaded { hunks, .. } => Some(hunks),
            _ => None,
        }
    }

    pub const fn collapsed_files(&self) -> &HashSet<String> {
        &self.files.collapsed
    }

    pub fn set_diff_viewport_height(&mut self, height: usize) {
        let clamped = height.max(1);
        if clamped == self.diff.state.viewport_height {
            return;
        }
        self.diff.state.set_viewport_height(height);
        let offset = self
            .diff
            .state
            .scroll_offset
            .min(self.max_diff_scroll_offset());
        self.diff.state.set_scroll_offset(offset);
        self.sync_diff_selection_to_scroll();
    }

    pub fn handle_event(&self, event: Event) -> Action {
        match event {
            Event::Key(key) => self.handle_key(key),
            Event::Resize(..) => Action::Noop,
        }
    }

    pub fn update(&mut self, action: Action) {
        match action {
            Action::Noop => {}
            Action::Quit => {
                self.should_quit = true;
            }
            Action::NextFocus => {
                self.focus = self.focus.next();
            }
            Action::PreviousFocus => {
                self.focus = self.focus.previous();
            }
            Action::FocusPanel(focus) => {
                self.focus = focus;
            }
            Action::MoveDown => match self.focus {
                Focus::Files => match self.review.mode {
                    ViewMode::SingleFile => {
                        self.select_next_row();
                        self.refresh_diff();
                    }
                    ViewMode::Continuous => {
                        self.select_next_row();
                        self.refresh_diff();
                        self.jump_to_selected_file();
                    }
                },
                Focus::Diff => match self.review.mode {
                    ViewMode::Continuous => self.scroll_continuous_diff_down(),
                    ViewMode::SingleFile => self.scroll_diff_down(),
                },
            },
            Action::MoveUp => match self.focus {
                Focus::Files => match self.review.mode {
                    ViewMode::SingleFile => {
                        self.select_previous_row();
                        self.refresh_diff();
                    }
                    ViewMode::Continuous => {
                        self.select_previous_row();
                        self.refresh_diff();
                        self.jump_to_selected_file();
                    }
                },
                Focus::Diff => match self.review.mode {
                    ViewMode::Continuous => self.scroll_continuous_diff_up(),
                    ViewMode::SingleFile => self.scroll_diff_up(),
                },
            },
            Action::NextHunk => {
                self.select_next_hunk();
            }
            Action::PreviousHunk => {
                self.select_previous_hunk();
            }
            Action::NextFile => {
                self.select_next_file();
                self.refresh_diff();
                if self.review.mode == ViewMode::Continuous {
                    self.jump_to_selected_file();
                }
            }
            Action::PreviousFile => {
                self.select_previous_file();
                self.refresh_diff();
                if self.review.mode == ViewMode::Continuous {
                    self.jump_to_selected_file();
                }
            }
            Action::ToggleDiffLineNumbers => {
                self.diff.state.toggle_line_numbers();
            }
            Action::Refresh => {
                self.repository_status = repository::status(&self.repo).ok();
                self.reload_review_doc();
            }
            Action::GoToFirst => {
                self.select_first_file();
                self.refresh_diff();
            }
            Action::GoToLast => {
                self.select_last_file();
                self.refresh_diff();
            }
            Action::ToggleCollapsed => {
                if let Some(path) = self.selected_dir() {
                    self.toggle_collapsed(path);
                }
            }
            Action::ToggleViewMode => match self.review.mode {
                ViewMode::Continuous => self.review.mode = ViewMode::SingleFile,
                ViewMode::SingleFile => self.review.mode = ViewMode::Continuous,
            },
            Action::ForceLoadDiff => {
                self.force_refresh_diff();
            }
        }

        if self.review_doc.index_dirty {
            let file_anchor = self
                .selected_file_idx()
                .or_else(|| self.current_continuous_file_idx());
            self.review_doc.rebuild_index();
            self.review_doc.index_dirty = false;
            self.sync_continuous_scroll_to_file(file_anchor);
        }
    }

    fn handle_key(&self, key: KeyEvent) -> Action {
        match key.code {
            KeyCode::Char('q') => Action::Quit,
            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => Action::Quit,
            KeyCode::Tab => Action::NextFocus,
            KeyCode::BackTab => Action::PreviousFocus,
            KeyCode::Char('0') => Action::FocusPanel(Focus::Diff),
            KeyCode::Char('1') => Action::FocusPanel(Focus::Files),
            KeyCode::Down | KeyCode::Char('j') => Action::MoveDown,
            KeyCode::Up | KeyCode::Char('k') => Action::MoveUp,
            KeyCode::Char(']') if self.focus == Focus::Diff => Action::NextHunk,
            KeyCode::Char('[') if self.focus == Focus::Diff => Action::PreviousHunk,
            KeyCode::Char('n') => Action::NextFile,
            KeyCode::Char('p') => Action::PreviousFile,
            KeyCode::Char('l') if self.focus == Focus::Diff => Action::ToggleDiffLineNumbers,
            KeyCode::Char('v') => Action::ToggleViewMode,
            KeyCode::Char('r') => Action::Refresh,
            KeyCode::Char('g') if self.focus == Focus::Files => Action::GoToFirst,
            KeyCode::Char('G') if self.focus == Focus::Files => Action::GoToLast,
            KeyCode::Enter if self.focus == Focus::Diff && self.diff.state.too_large.is_some() => {
                Action::ForceLoadDiff
            }
            KeyCode::Enter | KeyCode::Char(' ') if self.focus == Focus::Files => {
                Action::ToggleCollapsed
            }
            _ => Action::Noop,
        }
    }

    pub fn selected_file(&self) -> Option<&FileEntry> {
        let idx = self.files.state.selected?;
        let row = self.files.cached_rows.get(idx)?;
        if let TreeRow::File(entry_idx, _) = row {
            return self
                .review_doc
                .files
                .get(*entry_idx)
                .map(|slot| &slot.entry);
        }
        None
    }

    pub fn match_selected_file(&mut self) {
        let Some((file_idx, _)) = self
            .review_doc
            .index
            .file_at_row(self.review.continuous_scroll)
        else {
            return;
        };
        self.files.state.selected = self
            .files
            .cached_rows
            .iter()
            .enumerate()
            .find(|(_, row)| matches!(row, TreeRow::File(entry_idx, _) if *entry_idx == file_idx))
            .map(|(pos, _)| pos);
    }

    pub fn jump_to_selected_file(&mut self) {
        let Some(file) = self.selected_file() else {
            return;
        };
        let key = FileKey {
            path: file.path.clone(),
            status: file.status,
        };
        let Some(&file_idx) = self.review_doc.by_key.get(&key) else {
            return;
        };
        self.review.continuous_scroll = self.review_doc.index.file_starts[file_idx];
    }

    pub const fn set_tree_row_count(&mut self, len: usize) {
        self.files.state.tree_row_count = len;
    }

    pub fn toggle_collapsed(&mut self, path: String) {
        if !self.files.collapsed.remove(&path) {
            self.files.collapsed.insert(path);
        }
        self.files.dirty = true;
        self.ensure_rows();
        let len = self.files.cached_rows.len();
        if len == 0 {
            self.files.state.selected = None;
        } else if let Some(sel) = self.files.state.selected {
            self.files.state.selected = Some(sel.min(len - 1));
        }
    }

    pub fn ensure_rows(&mut self) {
        if !self.files.dirty {
            return;
        }
        self.files.cached_rows = tree_rows(&self.review_doc.files, &self.files.collapsed);
        self.files.state.tree_row_count = self.files.cached_rows.len();
        self.files.dirty = false;
    }

    pub fn cached_rows(&self) -> &[TreeRow] {
        &self.files.cached_rows
    }

    pub fn review_doc(&self) -> &ReviewDoc {
        &self.review_doc
    }

    pub fn review_state(&self) -> &ReviewState {
        &self.review
    }

    const fn select_first_file(&mut self) {
        self.files.state.select_first();
    }

    const fn select_last_file(&mut self) {
        self.files.state.select_last();
    }

    const fn select_next_row(&mut self) {
        self.files.state.select_next();
    }

    const fn select_previous_row(&mut self) {
        self.files.state.select_previous();
    }

    fn select_next_file(&mut self) {
        let rows = &self.files.cached_rows;
        let current = self.files.state.selected.unwrap_or(0);
        if let Some(next) = rows
            .iter()
            .enumerate()
            .skip(current + 1)
            .find(|(_, row)| matches!(row, TreeRow::File(..)))
            .map(|(i, _)| i)
        {
            self.files.state.selected = Some(next);
        }
    }

    fn select_previous_file(&mut self) {
        let rows = &self.files.cached_rows;
        let current = self.files.state.selected.unwrap_or(0);
        if let Some(prev) = rows
            .iter()
            .enumerate()
            .take(current)
            .rfind(|(_, row)| matches!(row, TreeRow::File(..)))
            .map(|(i, _)| i)
        {
            self.files.state.selected = Some(prev);
        }
    }

    pub fn refresh_diff(&mut self) {
        self.refresh_diff_internal(false);
    }

    pub fn force_refresh_diff(&mut self) {
        self.diff.state.too_large = None;
        self.refresh_diff_internal(true);
    }

    pub fn poll_workers(&mut self) {
        while let Ok(msg) = self.worker_rx.try_recv() {
            if msg.generation != self.review_doc.generation {
                continue;
            }

            let slot = &mut self.review_doc.files[msg.file_idx];
            slot.load = match msg.result {
                Ok((sections, hunks, index)) => DiffLoadState::Loaded {
                    sections,
                    hunks,
                    index,
                },
                Err(e) => DiffLoadState::Error(e),
            };
            self.review_doc.index_dirty = true;
        }

        if self.review_doc.index_dirty {
            let file_anchor = self
                .selected_file_idx()
                .or_else(|| self.current_continuous_file_idx());
            self.review_doc.rebuild_index();
            self.review_doc.index_dirty = false;
            self.sync_continuous_scroll_to_file(file_anchor);
        }
    }

    fn refresh_diff_internal(&mut self, force: bool) {
        self.ensure_rows();
        let Some(file) = self.selected_file() else {
            self.diff.current_key = None;
            return;
        };
        let path = file.path.clone();
        let status = file.status;

        let cache_key = FileKey {
            path: path.clone(),
            status,
        };

        if !force
            && let Ok(n) = repository::file_diff_line_count(&self.repo, &path, status)
            && n > repository::DIFF_LINE_THRESHOLD
        {
            self.diff.state.too_large = Some(n);
            self.diff.state.line_index = LineIndex::new(&[]);
            self.diff.current_key = Some(cache_key.clone());

            if let Some(&slot_idx) = self.review_doc.by_key.get(&cache_key)
                && !matches!(
                    self.review_doc.files[slot_idx].load,
                    DiffLoadState::Loaded { .. }
                )
            {
                self.review_doc.files[slot_idx].load = DiffLoadState::TooLarge { lines: n };
                self.review_doc.index_dirty = true;
            }
            return;
        }

        self.diff.state.too_large = None;

        if let Some(&slot_idx) = self.review_doc.by_key.get(&cache_key) {
            if matches!(
                self.review_doc.files[slot_idx].load,
                DiffLoadState::NotLoaded
            ) && let Ok(sections) = repository::file_diff(&self.repo, &path, status)
            {
                let hunks: Vec<DiffHunk> = sections.iter().flat_map(|s| s.hunks.clone()).collect();
                let index = LineIndex::new(&sections);
                self.review_doc.files[slot_idx].load = DiffLoadState::Loaded {
                    sections,
                    hunks,
                    index,
                };
            }

            let (sections, hunks): (&[DiffSection], &[DiffHunk]) =
                match &self.review_doc.files[slot_idx].load {
                    DiffLoadState::Loaded {
                        sections, hunks, ..
                    } => (sections, hunks),
                    _ => (&[], &[]),
                };

            self.diff.state.line_index = LineIndex::new(sections);
            self.diff.state.select_first_hunk(hunks.len());
            self.review_doc.index_dirty = true;
        }

        self.diff.current_key = Some(cache_key);
        self.sync_diff_scroll_to_hunk();
    }

    fn reload_review_doc(&mut self) {
        let selected_key = self.selected_file().map(|file| FileKey {
            path: file.path.clone(),
            status: file.status,
        });
        let next_generation = self.review_doc.generation + 1;
        let mut entries = repository::files(&self.repo).unwrap_or_default();
        Self::sort_file_entries(&mut entries);

        self.review_doc = Self::build_review_doc(entries, next_generation);
        self.review_doc.rebuild_index();
        self.files.dirty = true;
        self.ensure_rows();
        let restored_file_idx = self.restore_selection(selected_key);
        self.sync_continuous_scroll_to_file(restored_file_idx);
        self.diff.current_key = None;
        self.diff.state.too_large = None;
        self.diff.state.line_index = LineIndex::new(&[]);
        self.diff.state.select_first_hunk(0);
        self.spawn_diff_workers();
        self.refresh_diff();
    }

    fn restore_selection(&mut self, selected_key: Option<FileKey>) -> Option<usize> {
        if self.files.cached_rows.is_empty() {
            self.files.state.selected = None;
            return None;
        }

        if let Some(key) = selected_key
            && let Some(&file_idx) = self.review_doc.by_key.get(&key)
            && let Some(row_idx) = self.files.cached_rows.iter().position(
                |row| matches!(row, TreeRow::File(entry_idx, _) if *entry_idx == file_idx),
            )
        {
            self.files.state.selected = Some(row_idx);
            return Some(file_idx);
        }

        self.files.state.selected = Some(
            self.files
                .state
                .selected
                .unwrap_or(0)
                .min(self.files.cached_rows.len() - 1),
        );
        self.selected_file_idx()
    }

    fn selected_file_idx(&self) -> Option<usize> {
        let idx = self.files.state.selected?;
        let row = self.files.cached_rows.get(idx)?;
        match row {
            TreeRow::File(entry_idx, _) => Some(*entry_idx),
            TreeRow::Dir(..) => None,
        }
    }

    fn current_continuous_file_idx(&self) -> Option<usize> {
        self.review_doc
            .index
            .file_at_row(self.review.continuous_scroll)
            .map(|(file_idx, _)| file_idx)
    }

    fn sync_continuous_scroll_to_file(&mut self, file_idx: Option<usize>) {
        if let Some(file_idx) = file_idx
            && let Some(row) = self.review_doc.index.file_starts.get(file_idx)
        {
            self.review.continuous_scroll = *row;
            return;
        }

        self.review.continuous_scroll = self
            .review
            .continuous_scroll
            .min(self.max_continuous_diff_scroll_offset());
    }

    fn spawn_diff_workers(&self) {
        let generation = self.review_doc.generation;

        // Collect every file's work item up front, then hand them to a small,
        // fixed pool of workers that share the queue. Spawning one thread per
        // file would open thousands of git handles at once (causing a file-descriptor
        // exhaustion) and thousands of thread stacks. A bounded pool keeps both
        // proportional to the pool size, not the repo size.
        let jobs: Vec<_> = self
            .review_doc
            .files
            .iter()
            .enumerate()
            .map(|(file_idx, file)| (file_idx, file.entry.path.clone(), file.entry.status))
            .collect();
        let queue = Arc::new(Mutex::new(jobs.into_iter()));

        let worker_count = std::thread::available_parallelism()
            .map_or(4, |n| n.get())
            .min(8);

        for _ in 0..worker_count {
            let tx_clone = self.worker_tx.clone();
            let queue = Arc::clone(&queue);
            std::thread::spawn(move || {
                // Repository is !Send, so it must be created inside the thread,
                // never shared across threads.
                let Ok(repo) = Repository::discover(".") else {
                    return;
                };

                loop {
                    let job = queue.lock().unwrap().next();
                    let Some((file_idx, path, status)) = job else {
                        break;
                    };

                    let result = repository::file_diff(&repo, &path, status)
                        .map(|sections| {
                            let hunks: Vec<DiffHunk> =
                                sections.iter().flat_map(|s| s.hunks.clone()).collect();
                            let index = LineIndex::new(&sections);
                            (sections, hunks, index)
                        })
                        .map_err(|e| e.to_string());

                    if tx_clone
                        .send(WorkerResult {
                            generation,
                            file_idx,
                            result,
                        })
                        .is_err()
                    {
                        break;
                    }
                }
            });
        }
    }

    fn select_next_hunk(&mut self) {
        match self.review.mode {
            ViewMode::Continuous => self.select_next_continuous_hunk(),
            ViewMode::SingleFile => {
                let len = self.diff_hunks().map_or(0, std::vec::Vec::len);
                self.diff.state.select_next_hunk(len);
                self.sync_diff_scroll_to_hunk();
            }
        }
    }

    fn select_previous_hunk(&mut self) {
        match self.review.mode {
            ViewMode::Continuous => self.select_prev_continuous_hunk(),
            ViewMode::SingleFile => {
                let len = self.diff_hunks().map_or(0, std::vec::Vec::len);
                self.diff.state.select_previous_hunk(len);
                self.sync_diff_scroll_to_hunk();
            }
        }
    }

    fn continuous_hunk_rows(&self) -> Vec<usize> {
        self.review_doc
            .files
            .iter()
            .enumerate()
            .flat_map(|(file_idx, slot)| {
                let file_start = self
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

    fn select_next_continuous_hunk(&mut self) {
        let current = self.review.continuous_scroll;
        let rows = self.continuous_hunk_rows();
        if let Some(&row) = rows.iter().find(|&&r| r > current) {
            self.review.continuous_scroll = row;
            self.match_selected_file();
        }
    }

    fn select_prev_continuous_hunk(&mut self) {
        let current = self.review.continuous_scroll;
        let rows = self.continuous_hunk_rows();
        if let Some(&row) = rows.iter().rfind(|&&r| r < current) {
            self.review.continuous_scroll = row;
            self.match_selected_file();
        }
    }

    fn scroll_diff_down(&mut self) {
        let max_offset = self.max_diff_scroll_offset();
        let offset = (self.diff.state.scroll_offset + SCROLL_STEP).min(max_offset);
        self.diff.state.set_scroll_offset(offset);
        self.sync_diff_selection_to_scroll();
    }

    fn scroll_diff_up(&mut self) {
        let offset = self.diff.state.scroll_offset.saturating_sub(SCROLL_STEP);
        self.diff.state.set_scroll_offset(offset);
        self.clamp_scroll();
        self.sync_diff_selection_to_scroll();
    }

    fn max_continuous_diff_scroll_offset(&self) -> usize {
        self.review_doc
            .index
            .total_rows
            .saturating_sub(self.diff.state.viewport_height)
    }

    fn scroll_continuous_diff_down(&mut self) {
        let max_offset = self.max_continuous_diff_scroll_offset();
        let offset = (self.review.continuous_scroll + SCROLL_STEP).min(max_offset);
        self.review.continuous_scroll = offset;
        self.match_selected_file();
    }

    fn scroll_continuous_diff_up(&mut self) {
        let offset = self.review.continuous_scroll.saturating_sub(SCROLL_STEP);
        self.review.continuous_scroll = offset;
        self.match_selected_file();
    }

    const fn diff_row_count(&self) -> usize {
        self.diff.state.line_index.total_rows
    }

    const fn max_diff_scroll_offset(&self) -> usize {
        self.diff_row_count()
            .saturating_sub(self.diff.state.viewport_height)
    }

    fn sync_diff_scroll_to_hunk(&mut self) {
        let offset = self
            .diff
            .state
            .selected_hunk
            .and_then(|hunk_idx| {
                self.diff
                    .state
                    .line_index
                    .hunk_starts
                    .get(hunk_idx)
                    .copied()
            })
            .unwrap_or(0);

        self.clamp_scroll();
        self.diff.state.set_scroll_offset(offset);
    }

    fn sync_diff_selection_to_scroll(&mut self) {
        let row = self
            .diff
            .state
            .line_index
            .lookup(self.diff.state.scroll_offset);
        match row {
            Some(IndexRow::HunkHeader(hunk_idx)) => {
                self.diff.state.select_hunk_line(hunk_idx, 0);
            }
            Some(IndexRow::DiffLine(hunk_idx, line_idx)) => {
                self.diff.state.select_hunk_line(hunk_idx, line_idx);
            }
            _ => {
                self.diff.state.select_first_hunk(0);
            }
        }
    }

    fn selected_dir(&self) -> Option<String> {
        let idx = self.files.state.selected?;

        let rows = &self.files.cached_rows;
        if let Some(TreeRow::Dir(path, _)) = rows.get(idx) {
            return Some(path.clone());
        }
        None
    }

    fn clamp_scroll(&mut self) {
        let max = self
            .diff_row_count()
            .saturating_sub(self.diff.state.viewport_height);
        self.diff.state.scroll_offset = self.diff.state.scroll_offset.min(max);
    }

    fn sort_file_entries(entries: &mut [FileEntry]) {
        entries.sort_by(|a, b| {
            let a_key = (a.path.contains('/'), a.path.to_lowercase());
            let b_key = (b.path.contains('/'), b.path.to_lowercase());
            a_key.cmp(&b_key)
        });
    }

    fn tree_sort_key(path: &str) -> Vec<(u8, &str)> {
        let segments: Vec<&str> = path.split('/').collect();
        let last = segments.len().saturating_sub(1);
        segments
            .into_iter()
            .enumerate()
            // 0u8 = directory (not last segment), 1u8 = file (last segment)
            // sorts dirs before files
            .map(|(i, seg)| (if i == last { 1u8 } else { 0u8 }, seg))
            .collect()
    }

    fn build_review_doc(mut entries: Vec<FileEntry>, generation: u64) -> ReviewDoc {
        entries.sort_by(|a, b| Self::tree_sort_key(&a.path).cmp(&Self::tree_sort_key(&b.path)));

        let mut file_slots = Vec::new();
        let mut by_key = HashMap::new();

        for (idx, entry) in entries.into_iter().enumerate() {
            let file_key = FileKey {
                path: entry.path.clone(),
                status: entry.status,
            };

            by_key.insert(file_key, idx);
            file_slots.push(FileSlot {
                entry,
                load: DiffLoadState::NotLoaded,
            });
        }

        ReviewDoc {
            files: file_slots,
            by_key,
            index: ReviewIndex::default(),
            index_dirty: false,
            generation,
        }
    }
}

impl std::fmt::Debug for App {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("App").finish_non_exhaustive()
    }
}
