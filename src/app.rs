use std::collections::HashSet;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use crossterm::{execute, terminal::SetTitle};
use git2::Repository;

use crate::action::Action;
use crate::cli::Command;
use crate::diff_panel::DiffPanel;
use crate::event::Event;
use crate::files_panel::FilesPanel;
use crate::git::repository::{self, DiffHunk};
use crate::state::{ContinuousDiff, Diff, FileSlot, Files, Focus, ReviewState, Screen, TreeRow};
use crate::store::DiffStore;
use crate::tui::theme::{self, Theme};

pub struct App {
    screen: Screen,
    focus: Focus,
    theme: Theme,
    repo: Repository,
    repository_status: Option<repository::RepositoryStatus>,
    should_quit: bool,
    files: FilesPanel,
    diff: DiffPanel,
    store: DiffStore,
}

impl App {
    pub fn new(command: Option<Command>) -> Self {
        execute!(std::io::stdout(), SetTitle("marten")).ok();
        let repo = Repository::discover(".").expect("not a git repo");
        Self::init(repo, command)
    }

    fn init(repo: Repository, command: Option<Command>) -> Self {
        let repository_status = repository::status(&repo).ok();
        let entries = match &command {
            Some(Command::Show { oid }) => repository::files_from_commit(&repo, oid)
                .ok()
                .unwrap_or_default(),
            _ => repository::files(&repo).ok().unwrap_or_default(),
        };

        let mut store = DiffStore::new(entries);
        store.continuous_diff.rebuild_index();
        let commit_hash = match &command {
            Some(Command::Show { oid }) => Some(oid.clone()),
            _ => None,
        };
        store.spawn_workers(commit_hash);

        let mut files = FilesPanel::new();
        files.ensure_rows(&store);
        files.select_first();

        let mut diff = DiffPanel::new();
        diff.refresh(&mut files, &mut store, &repo);

        let (width, _) = crossterm::terminal::size().unwrap_or((0, 0));

        let focus = if width <= 120 {
            Focus::Diff
        } else {
            Focus::Files
        };

        Self {
            screen: Screen::Home,
            focus,
            files,
            diff,
            store,
            theme: theme::DEFAULT,
            repo,
            should_quit: false,
            repository_status,
        }
    }

    pub const fn screen(&self) -> Screen {
        self.screen
    }

    pub const fn focus(&self) -> Focus {
        self.focus
    }

    pub const fn files_state(&self) -> &Files {
        self.files.state()
    }

    pub const fn theme(&self) -> Theme {
        self.theme
    }

    pub const fn should_quit(&self) -> bool {
        self.should_quit
    }

    pub const fn repository_status(&self) -> Option<&repository::RepositoryStatus> {
        self.repository_status.as_ref()
    }

    pub fn files(&self) -> &[FileSlot] {
        &self.store.continuous_diff.files
    }

    pub fn selected_file(&self) -> Option<&repository::FileEntry> {
        self.files.selected_file(&self.store)
    }

    pub const fn diff_state(&self) -> &Diff {
        self.diff.state()
    }

    pub fn diff_hunks(&self) -> Option<&Vec<DiffHunk>> {
        self.diff.diff_hunks(&self.store)
    }

    pub const fn collapsed_files(&self) -> &HashSet<String> {
        self.files.collapsed()
    }

    pub fn set_tree_row_count(&mut self, len: usize) {
        self.files.set_tree_row_count(len);
    }

    pub fn set_diff_viewport_height(&mut self, height: usize) {
        self.diff.set_viewport_height(height);
    }

    pub fn ensure_rows(&mut self) {
        self.files.ensure_rows(&self.store);
    }

    pub fn cached_rows(&self) -> &[TreeRow] {
        self.files.cached_rows()
    }

    pub fn continuous_diff(&self) -> &ContinuousDiff {
        &self.store.continuous_diff
    }

    pub fn review_state(&self) -> &ReviewState {
        self.diff.review()
    }

    pub fn handle_event(&mut self, event: Event) -> Action {
        match event {
            Event::Key(key) => self.handle_key(key),
            Event::Resize(width, _) => {
                if width <= 120 && self.focus != Focus::Diff {
                    Action::FocusPanel(Focus::Diff)
                } else {
                    Action::Noop
                }
            }
        }
    }

    pub fn update(&mut self, action: Action) {
        let App {
            focus,
            files,
            diff,
            store,
            repo,
            repository_status,
            should_quit,
            ..
        } = self;

        match action {
            Action::Quit => {
                *should_quit = true;
                return;
            }
            Action::NextFocus => {
                *focus = focus.next();
                return;
            }
            Action::PreviousFocus => {
                *focus = focus.previous();
                return;
            }
            Action::FocusPanel(f) => {
                *focus = f;
                return;
            }
            Action::Refresh => {
                *repository_status = repository::status(repo).ok();
                diff.reload(files, store, repo);
            }
            _ => {
                let selection_changed = files.update(action, *focus, store);
                diff.update(action, *focus, selection_changed, files, store, repo);
            }
        }

        if store.continuous_diff.index_dirty {
            let file_anchor = files
                .selected_file_idx()
                .or_else(|| diff.current_continuous_file_idx(store));
            store.continuous_diff.rebuild_index();
            store.continuous_diff.index_dirty = false;
            diff.sync_continuous_scroll_to_file(file_anchor, store);
        }
    }

    pub fn poll_workers(&mut self) -> bool {
        let changed = self.store.poll_workers();
        if self.store.continuous_diff.index_dirty {
            let file_anchor = self
                .files
                .selected_file_idx()
                .or_else(|| self.diff.current_continuous_file_idx(&self.store));
            self.store.continuous_diff.rebuild_index();
            self.store.continuous_diff.index_dirty = false;
            self.diff
                .sync_continuous_scroll_to_file(file_anchor, &self.store);
        }
        changed
    }

    fn handle_key(&mut self, key: KeyEvent) -> Action {
        match key.code {
            KeyCode::Char('q') => Action::Quit,
            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => Action::Quit,
            KeyCode::Tab => Action::NextFocus,
            KeyCode::BackTab => Action::PreviousFocus,
            KeyCode::Char('0') => Action::FocusPanel(Focus::Diff),
            KeyCode::Char('1') => Action::FocusPanel(Focus::Files),
            KeyCode::Down | KeyCode::Char('j') => Action::MoveDown,
            KeyCode::Up | KeyCode::Char('k') => Action::MoveUp,
            KeyCode::Char(']') => match self.focus {
                Focus::Diff => Action::NextHunk,
                Focus::Files => {
                    if !self.files().is_empty() {
                        self.focus = Focus::Diff;
                        Action::NextHunk
                    } else {
                        Action::Noop
                    }
                }
            },
            KeyCode::Char('[') if self.focus == Focus::Diff => Action::PreviousHunk,
            KeyCode::Char('n') => Action::NextFile,
            KeyCode::Char('p') => Action::PreviousFile,
            KeyCode::Char('l') if self.focus == Focus::Diff => Action::ToggleDiffLineNumbers,
            KeyCode::Char('v') => Action::ToggleViewMode,
            KeyCode::Char('r') => Action::Refresh,
            KeyCode::Char('g') if self.focus == Focus::Files => Action::GoToFirst,
            KeyCode::Char('G') if self.focus == Focus::Files => Action::GoToLast,
            KeyCode::Enter if self.focus == Focus::Diff && self.diff.is_too_large() => {
                Action::ForceLoadDiff
            }
            KeyCode::Enter | KeyCode::Char(' ') if self.focus == Focus::Files => {
                Action::ToggleCollapsed
            }
            _ => Action::Noop,
        }
    }
}

impl std::fmt::Debug for App {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("App").finish_non_exhaustive()
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use git2::Repository;
    use tempfile::TempDir;

    use super::*;
    use crate::state::tree::TreeRow;

    fn init_repo() -> (TempDir, Repository) {
        let dir = TempDir::new().unwrap();
        let repo = Repository::init(dir.path()).unwrap();
        (dir, repo)
    }

    #[test]
    fn match_selected_file_expands_collapsed_dir() {
        let (dir, repo) = init_repo();

        let file_path = dir.path().join("src").join("main.rs");
        fs::create_dir_all(file_path.parent().unwrap()).unwrap();
        fs::write(&file_path, "fn main() {}\n").unwrap();

        let mut app = App::init(repo, None);
        assert_eq!(app.store.continuous_diff.files.len(), 1);
        assert_eq!(app.store.continuous_diff.files[0].entry.path, "src/main.rs");
        assert!(
            app.files
                .cached_rows()
                .iter()
                .any(|r| matches!(r, TreeRow::File(..)))
        );

        app.files.collapse_dir_for_test("src");
        app.ensure_rows();
        assert!(
            !app.files
                .cached_rows()
                .iter()
                .any(|r| matches!(r, TreeRow::File(..)))
        );
        assert!(app.files.collapsed().contains("src"));

        app.diff
            .set_continuous_scroll(app.store.continuous_diff.index.file_starts[0]);
        let scroll = app.diff.continuous_scroll();
        app.files.match_selected_file(&app.store, scroll);
        assert!(!app.files.collapsed().contains("src"));
        assert!(
            app.files
                .cached_rows()
                .iter()
                .any(|r| matches!(r, TreeRow::File(..)))
        );
        assert_eq!(app.files.state().selected, Some(1));
    }
}
