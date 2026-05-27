use std::collections::{HashMap, HashSet};

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use crossterm::{execute, terminal::SetTitle};
use git2::Repository;

use crate::action::Action;
use crate::event::Event;
use crate::git::repository::{self, DiffHunk, FileEntry, RepositoryStatus};
use crate::state::{
    Diff, Files, Focus, Screen,
    files::STATUS_ORDER,
    tree::{TreeRow, tree_rows},
};
use crate::state::{
    DiffLoadState, FileKey, FileSlot, LineIndex, LoadingProgress, ReviewDoc, ReviewIndex, ReviewState, ViewMode, WorkerResult
};
use crate::tui::theme::{self, Theme};

const SCROLL_STEP: usize = 1;

pub struct FilesPanel {
    state: Files,
    entries: Option<Vec<FileEntry>>,
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
}

impl App {
    pub fn new() -> Self {
        // set app title
        execute!(std::io::stdout(), SetTitle("marten")).ok();

        let repo = Repository::discover(".").expect("not a git repo");
        let repository_status = repository::status(&repo).ok();
        let files = repository::files(&repo).ok().map(|mut f| {
            f.sort_by_key(|e| {
                STATUS_ORDER
                    .iter()
                    .position(|s| *s == e.status)
                    .unwrap_or(99)
            });
            f
        });

        let review_doc = match &files {
            None => ReviewDoc {
                files: vec![],
                by_key: HashMap::new(),
                index: ReviewIndex::default(),
                index_dirty: false,
                generation: 0,
                loading: LoadingProgress::default(),
            },
            Some(entries) => {
                let mut file_slots = Vec::new();
                let mut by_key = HashMap::new();

                for (idx, entry) in entries.iter().enumerate() {
                    let file_key = FileKey {
                        path: entry.path.clone(),
                        status: entry.status,
                    };

                    let file_slot = FileSlot {
                        entry: entry.clone(),
                        load: crate::state::DiffLoadState::NotLoaded,
                    };

                    by_key.insert(file_key, idx);
                    file_slots.push(file_slot);
                }
                ReviewDoc {
                    files: file_slots,
                    by_key,
                    index: ReviewIndex::default(),
                    index_dirty: false,
                    generation: 0,
                    loading: LoadingProgress::default(),
                }
            }
        };

        let (tx, rx) = std::sync::mpsc::channel::<WorkerResult>();

        let mut app = Self {
            screen: Screen::Home,
            focus: Focus::Files,
            files: FilesPanel {
                state: Files::default(),
                // TODO: FilesPanel.entries should read from ReviewDoc
                entries: files,
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
            theme: theme::DEFAULT,
            repo,
            should_quit: false,
            repository_status,
        };

        let generation = app.review_doc.generation;
        for (file_idx, file) in app.review_doc.files.iter().enumerate() {
            let tx_clone = tx.clone();
            let path = file.entry.path.clone();
            let status = file.entry.status;
            std::thread::spawn(move || {
                let repo = Repository::discover(".").unwrap();
                let result = repository::file_diff(&repo, &path, status)
                    .map(|hunks| {
                        let index = LineIndex::new(&hunks);
                        (hunks, index)
                    })
                    .map_err(|e| e.to_string());

                tx_clone.send(WorkerResult { generation, file_idx, result }).ok();
                
            });
        }

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

    pub const fn files(&self) -> Option<&Vec<FileEntry>> {
        self.files.entries.as_ref()
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
                Focus::Files => {
                    self.select_next_file();
                    self.refresh_diff();
                }
                Focus::Diff => match self.review.mode {
                    ViewMode::Continuous => self.scroll_continuous_diff_down(),
                    ViewMode::SingleFile => self.scroll_diff_down()
                }
            },
            Action::MoveUp => match self.focus {
                Focus::Files => {
                    self.select_previous_file();
                    self.refresh_diff();
                }
                Focus::Diff => match self.review.mode {
                    ViewMode::Continuous => self.scroll_continuous_diff_up(),
                    ViewMode::SingleFile => self.scroll_diff_up()
                }
            },
            Action::NextHunk => {
                self.select_next_hunk();
            }
            Action::PreviousHunk => {
                self.select_previous_hunk();
            }
            Action::ToggleDiffLineNumbers => {
                self.diff.state.toggle_line_numbers();
            }
            Action::Refresh => {
                self.repository_status = repository::status(&self.repo).ok();
                self.files.entries = repository::files(&self.repo).ok().map(|mut f| {
                    f.sort_by_key(|e| {
                        STATUS_ORDER
                            .iter()
                            .position(|s| *s == e.status)
                            .unwrap_or(99)
                    });
                    f
                });
                self.files.dirty = true;
                self.ensure_rows();
                let len = self.files.cached_rows.len();
                if len == 0 {
                    self.files.state.selected = None;
                } else {
                    self.files.state.selected =
                        Some(self.files.state.selected.unwrap_or(0).min(len - 1));
                }
                self.review_doc.by_key.clear();
                self.refresh_diff();
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
            Action::ToggleViewMode => {
                match self.review.mode {
                    ViewMode::Continuous => self.review.mode = ViewMode::SingleFile,
                    ViewMode::SingleFile => self.review.mode = ViewMode::Continuous,
                }
            }
            Action::ForceLoadDiff => {
                self.force_refresh_diff();
            }
        }

        if self.review_doc.index_dirty {
            self.review_doc.rebuild_index();
            self.review_doc.index_dirty = false;
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
            return self.files.entries.as_ref()?.get(*entry_idx);
        }
        None
    }

    pub fn match_selected_file(&mut self) {
        let Some((file_idx, _)) = self.review_doc.index.file_at_row(self.review.continuous_scroll) else {
            return;
        };
        self.files.state.selected = self.files.cached_rows
            .iter()
            .enumerate()
            .find(|(_, row)| matches!(row, TreeRow::File(entry_idx, _) if *entry_idx == file_idx))
            .map(|(pos, _)| pos);
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
        if let Some(entries) = &self.files.entries {
            self.files.cached_rows = tree_rows(entries, &self.files.collapsed);
        } else {
            self.files.cached_rows.clear();
        }
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

    const fn select_next_file(&mut self) {
        self.files.state.select_next();
    }

    const fn select_previous_file(&mut self) {
        self.files.state.select_previous();
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
                Ok((hunks, index)) => DiffLoadState::Loaded { hunks, index },
                Err(e) => DiffLoadState::Error(e),
            };
            self.review_doc.index_dirty = true;
        }

        if self.review_doc.index_dirty {
            self.review_doc.rebuild_index();
            self.review_doc.index_dirty = false;
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

            if let Some(&slot_idx) = self.review_doc.by_key.get(&cache_key) {
                self.review_doc.files[slot_idx].load = DiffLoadState::TooLarge {
                    lines: n,
                    forced: false,
                };
            }
            return;
        }

        self.diff.state.too_large = None;

        if let Some(&slot_idx) = self.review_doc.by_key.get(&cache_key) {
            if matches!(
                self.review_doc.files[slot_idx].load,
                DiffLoadState::NotLoaded
            ) && let Ok(hunks) = repository::file_diff(&self.repo, &path, status)
            {
                let index = LineIndex::new(&hunks);
                self.review_doc.files[slot_idx].load = DiffLoadState::Loaded { hunks, index };
            }

            let hunks: &[DiffHunk] = match &self.review_doc.files[slot_idx].load {
                DiffLoadState::Loaded { hunks, .. } => hunks,
                _ => &[],
            };

            self.diff.state.line_index = LineIndex::new(hunks);
            self.diff.state.select_first_hunk(hunks.len());
            self.review_doc.index_dirty = true;
        }

        self.diff.current_key = Some(cache_key);
        self.sync_diff_scroll_to_hunk();
    }

    fn select_next_hunk(&mut self) {
        let len = self.diff_hunks().map_or(0, std::vec::Vec::len);
        self.diff.state.select_next_hunk(len);
        self.sync_diff_scroll_to_hunk();
    }

    fn select_previous_hunk(&mut self) {
        let len = self.diff_hunks().map_or(0, std::vec::Vec::len);
        self.diff.state.select_previous_hunk(len);
        self.sync_diff_scroll_to_hunk();
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
        self.review_doc.index.total_rows
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
        let Some((hunk_idx, line_in_hunk)) = self
            .diff
            .state
            .line_index
            .lookup(self.diff.state.scroll_offset)
        else {
            self.diff.state.select_first_hunk(0);
            return;
        };

        let line_idx = if line_in_hunk == 0 {
            0
        } else {
            line_in_hunk - 1
        };
        self.diff.state.select_hunk_line(hunk_idx, line_idx);
    }

    fn selected_dir(&self) -> Option<String> {
        let idx = self.files.state.selected?;
        self.files.entries.as_ref()?;

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
}

impl std::fmt::Debug for App {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("App").finish_non_exhaustive()
    }
}
