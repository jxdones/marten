use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use crossterm::{execute, terminal::SetTitle};
use git2::Repository;

use crate::action::Action;
use crate::event::Event;
use crate::git::repository::{self, DiffHunk, FileEntry, RepositoryStatus};
use crate::state::{Diff, Files, Focus, Screen, files::STATUS_ORDER};
use crate::tui::theme::{self, Theme};

const SCROLL_STEP: usize = 3;

pub struct App {
    screen: Screen,
    focus: Focus,
    files_state: Files,
    diff_state: Diff,
    theme: Theme,
    repo: Repository,
    repository_status: Option<RepositoryStatus>,
    files: Option<Vec<FileEntry>>,
    diff_hunks: Option<Vec<DiffHunk>>,
    should_quit: bool,
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
            files_state: Files::default(),
            diff_state: Diff::default(),
            diff_hunks: None,
            theme: theme::DEFAULT,
            repo,
            should_quit: false,
            repository_status,
            files,
        };

        app.select_first_file();
        app.refresh_diff();
        app
    }

    pub fn screen(&self) -> Screen {
        self.screen
    }

    pub fn focus(&self) -> Focus {
        self.focus
    }

    pub fn files_state(&self) -> &Files {
        &self.files_state
    }

    pub fn theme(&self) -> Theme {
        self.theme
    }

    pub fn should_quit(&self) -> bool {
        self.should_quit
    }

    pub fn repository_status(&self) -> Option<&RepositoryStatus> {
        self.repository_status.as_ref()
    }

    pub fn files(&self) -> Option<&Vec<FileEntry>> {
        self.files.as_ref()
    }

    pub fn diff_state(&self) -> &Diff {
        &self.diff_state
    }

    pub fn diff_hunks(&self) -> Option<&Vec<DiffHunk>> {
        self.diff_hunks.as_ref()
    }

    pub fn set_diff_viewport_height(&mut self, height: usize) {
        let clamped = height.max(1);
        if clamped == self.diff_state.viewport_height {
            return;
        }
        self.diff_state.set_viewport_height(height);
        let offset = self
            .diff_state
            .scroll_offset
            .min(self.max_diff_scroll_offset());
        self.diff_state.set_scroll_offset(offset);
        self.sync_diff_selection_to_scroll();
    }

    pub fn handle_event(&self, event: Event) -> Action {
        match event {
            Event::Key(key) => self.handle_key(key),
            _ => Action::Noop,
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
            Action::MoveDown => match self.focus {
                Focus::Files => {
                    self.select_next_file();
                    self.refresh_diff();
                }
                Focus::Diff => {
                    self.scroll_diff_down();
                }
                _ => {}
            },
            Action::MoveUp => match self.focus {
                Focus::Files => {
                    self.select_previous_file();
                    self.refresh_diff();
                }
                Focus::Diff => {
                    self.scroll_diff_up();
                }
                _ => {}
            },
            Action::NextHunk => {
                self.select_next_hunk();
            }
            Action::PreviousHunk => {
                self.select_previous_hunk();
            }
            Action::ToggleDiffLineNumbers => {
                self.diff_state.toggle_line_numbers();
            }
            Action::Refresh => {
                self.repository_status = repository::status(&self.repo).ok();
                self.files = repository::files(&self.repo).ok().map(|mut f| {
                    f.sort_by_key(|e| {
                        STATUS_ORDER
                            .iter()
                            .position(|s| *s == e.status)
                            .unwrap_or(99)
                    });
                    f
                });

                let len = self.files.as_ref().map_or(0, |f| f.len());
                if len == 0 {
                    self.files_state.selected = None;
                } else {
                    self.files_state.selected =
                        Some(self.files_state.selected.unwrap_or(0).min(len - 1));
                }
                self.refresh_diff();
            },
            Action::GoToFirst => {
                self.select_first_file();
                self.refresh_diff();
            }
            Action::GoToLast => {
                self.select_last_file();
                self.refresh_diff();
            }
        }
    }

    fn handle_key(&self, key: KeyEvent) -> Action {
        match key.code {
            KeyCode::Char('q') => Action::Quit,
            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => Action::Quit,
            KeyCode::Tab => Action::NextFocus,
            KeyCode::BackTab => Action::PreviousFocus,
            KeyCode::Down | KeyCode::Char('j') => Action::MoveDown,
            KeyCode::Up | KeyCode::Char('k') => Action::MoveUp,
            KeyCode::Char(']') if self.focus == Focus::Diff => Action::NextHunk,
            KeyCode::Char('[') if self.focus == Focus::Diff => Action::PreviousHunk,
            KeyCode::Char('l') if self.focus == Focus::Diff => Action::ToggleDiffLineNumbers,
            KeyCode::Char('r') => Action::Refresh,
            KeyCode::Char('g') if self.focus == Focus::Files => Action::GoToFirst,
            KeyCode::Char('G') if self.focus == Focus::Files => Action::GoToLast,
            _ => Action::Noop,
        }
    }

    pub fn selected_file(&self) -> Option<&FileEntry> {
        let files = self.files.as_ref()?;
        let idx = self.files_state.selected?;
        files.get(idx)
    }

    fn select_first_file(&mut self) {
        let len = self.files.as_ref().map_or(0, |f| f.len());
        self.files_state.select_first(len);
    }

    fn select_last_file(&mut self) {
        let len = self.files.as_ref().map_or(0, |f| f.len());
        self.files_state.select_last(len);
    }

    fn select_next_file(&mut self) {
        let len = self.files.as_ref().map_or(0, |f| f.len());
        self.files_state.select_next(len);
    }

    fn select_previous_file(&mut self) {
        let len = self.files.as_ref().map_or(0, |f| f.len());
        self.files_state.select_previous(len);
    }

    fn refresh_diff(&mut self) {
        let Some(file) = self.selected_file() else {
            self.diff_hunks = None;
            return;
        };
        let path = file.path.clone();
        let status = file.status;
        self.diff_hunks = repository::file_diff(&self.repo, &path, status).ok();
        self.diff_state
            .select_first_hunk(self.diff_hunks.as_ref().map_or(0, |hunk| hunk.len()));
        self.sync_diff_scroll_to_hunk();
    }

    fn select_next_hunk(&mut self) {
        let len = self.diff_hunks.as_ref().map_or(0, |h| h.len());
        self.diff_state.select_next_hunk(len);
        self.sync_diff_scroll_to_hunk();
    }

    fn select_previous_hunk(&mut self) {
        let len = self.diff_hunks.as_ref().map_or(0, |h| h.len());
        self.diff_state.select_previous_hunk(len);
        self.sync_diff_scroll_to_hunk();
    }

    fn scroll_diff_down(&mut self) {
        let max_offset = self.max_diff_scroll_offset();
        let offset = (self.diff_state.scroll_offset + SCROLL_STEP).min(max_offset);
        self.diff_state.set_scroll_offset(offset);
        self.sync_diff_selection_to_scroll();
    }

    fn scroll_diff_up(&mut self) {
        let offset = self.diff_state.scroll_offset.saturating_sub(SCROLL_STEP);
        self.diff_state.set_scroll_offset(offset);
        self.sync_diff_selection_to_scroll();
    }

    fn diff_row_count(&self) -> usize {
        self.diff_hunks.as_ref().map_or(0, |hunks| {
            hunks.iter().map(|hunk| 1 + hunk.lines.len()).sum()
        })
    }

    fn max_diff_scroll_offset(&self) -> usize {
        self.diff_row_count()
            .saturating_sub(self.diff_state.viewport_height)
    }

    fn sync_diff_scroll_to_hunk(&mut self) {
        let offset = self
            .diff_state
            .selected_hunk
            .and_then(|selected| {
                self.diff_hunks.as_ref().map(|hunks| {
                    hunks
                        .iter()
                        .take(selected)
                        .map(|hunk| 1 + hunk.lines.len())
                        .sum()
                })
            })
            .unwrap_or(0);

        self.diff_state
            .set_scroll_offset(offset.min(self.max_diff_scroll_offset()));
    }

    fn sync_diff_selection_to_scroll(&mut self) {
        let Some(hunks) = self.diff_hunks.as_ref() else {
            self.diff_state.select_first_hunk(0);
            return;
        };

        let mut row_start = 0;
        for (hunk_idx, hunk) in hunks.iter().enumerate() {
            let row_end = row_start + 1 + hunk.lines.len();
            if self.diff_state.scroll_offset < row_end {
                let line_idx = self
                    .diff_state
                    .scroll_offset
                    .saturating_sub(row_start + 1)
                    .min(hunk.lines.len().saturating_sub(1));
                self.diff_state.select_hunk_line(hunk_idx, line_idx);
                return;
            }
            row_start = row_end;
        }

        if let Some((hunk_idx, hunk)) = hunks.iter().enumerate().next_back() {
            self.diff_state
                .select_hunk_line(hunk_idx, hunk.lines.len().saturating_sub(1));
        }
    }
}

impl std::fmt::Debug for App {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("App").finish_non_exhaustive()
    }
}
