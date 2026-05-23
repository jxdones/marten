use std::collections::HashSet;

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
use crate::tui::theme::{self, Theme};

const SCROLL_STEP: usize = 3;

pub struct FilesPanel {
    state: Files,
    entries: Option<Vec<FileEntry>>,
    cached_rows: Vec<TreeRow>,
    collapsed: HashSet<String>,
    dirty: bool,
}

pub struct DiffPanel {
    state: Diff,
    hunks: Option<Vec<DiffHunk>>,
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
        let mut app = Self {
            screen: Screen::Home,
            focus: Focus::Files,
            files: FilesPanel {
                state: Files::default(),
                entries: files,
                cached_rows: Vec::new(),
                collapsed: HashSet::new(),
                dirty: true,
            },
            diff: DiffPanel {
                state: Diff::default(),
                hunks: None,
            },
            theme: theme::DEFAULT,
            repo,
            should_quit: false,
            repository_status,
        };

        app.select_first_file();
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

    pub const fn diff_hunks(&self) -> Option<&Vec<DiffHunk>> {
        self.diff.hunks.as_ref()
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
                Focus::Diff => {
                    self.scroll_diff_down();
                }
            },
            Action::MoveUp => match self.focus {
                Focus::Files => {
                    self.select_previous_file();
                    self.refresh_diff();
                }
                Focus::Diff => {
                    self.scroll_diff_up();
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
            KeyCode::Char('r') => Action::Refresh,
            KeyCode::Char('g') if self.focus == Focus::Files => Action::GoToFirst,
            KeyCode::Char('G') if self.focus == Focus::Files => Action::GoToLast,
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

    pub fn set_tree_row_count(&mut self, len: usize) {
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

    fn select_first_file(&mut self) {
        self.files.state.select_first();
    }

    fn select_last_file(&mut self) {
        self.files.state.select_last();
    }

    fn select_next_file(&mut self) {
        self.files.state.select_next();
    }

    fn select_previous_file(&mut self) {
        self.files.state.select_previous();
    }

    fn refresh_diff(&mut self) {
        self.ensure_rows();
        let Some(file) = self.selected_file() else {
            self.diff.hunks = None;
            return;
        };
        let path = file.path.clone();
        let status = file.status;
        self.diff.hunks = repository::file_diff(&self.repo, &path, status).ok();
        self.diff
            .state
            .select_first_hunk(self.diff.hunks.as_ref().map_or(0, Vec::len));
        self.sync_diff_scroll_to_hunk();
    }

    fn select_next_hunk(&mut self) {
        let len = self.diff.hunks.as_ref().map_or(0, Vec::len);
        self.diff.state.select_next_hunk(len);
        self.sync_diff_scroll_to_hunk();
    }

    fn select_previous_hunk(&mut self) {
        let len = self.diff.hunks.as_ref().map_or(0, Vec::len);
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
        self.sync_diff_selection_to_scroll();
    }

    fn diff_row_count(&self) -> usize {
        self.diff.hunks.as_ref().map_or(0, |hunks| {
            hunks.iter().map(|hunk| 1 + hunk.lines.len()).sum()
        })
    }

    fn max_diff_scroll_offset(&self) -> usize {
        self.diff_row_count()
            .saturating_sub(self.diff.state.viewport_height)
    }

    fn sync_diff_scroll_to_hunk(&mut self) {
        let offset = self
            .diff
            .state
            .selected_hunk
            .and_then(|selected| {
                self.diff.hunks.as_ref().map(|hunks| {
                    hunks
                        .iter()
                        .take(selected)
                        .map(|hunk| 1 + hunk.lines.len())
                        .sum()
                })
            })
            .unwrap_or(0);

        self.diff
            .state
            .set_scroll_offset(offset.min(self.max_diff_scroll_offset()));
    }

    fn sync_diff_selection_to_scroll(&mut self) {
        let Some(hunks) = self.diff.hunks.as_ref() else {
            self.diff.state.select_first_hunk(0);
            return;
        };

        let mut row_start = 0;
        for (hunk_idx, hunk) in hunks.iter().enumerate() {
            let row_end = row_start + 1 + hunk.lines.len();
            if self.diff.state.scroll_offset < row_end {
                let line_idx = self
                    .diff
                    .state
                    .scroll_offset
                    .saturating_sub(row_start + 1)
                    .min(hunk.lines.len().saturating_sub(1));
                self.diff.state.select_hunk_line(hunk_idx, line_idx);
                return;
            }
            row_start = row_end;
        }

        if let Some((hunk_idx, hunk)) = hunks.iter().enumerate().next_back() {
            self.diff
                .state
                .select_hunk_line(hunk_idx, hunk.lines.len().saturating_sub(1));
        }
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
}

impl std::fmt::Debug for App {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("App").finish_non_exhaustive()
    }
}
