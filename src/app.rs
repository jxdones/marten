use std::collections::HashSet;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers, MouseEvent, MouseEventKind};
use crossterm::{execute, terminal::SetTitle};
use git2::{ErrorCode, Repository};

use crate::action::Action;
use crate::cli::Command;
use crate::diff_panel::{DiffContext, DiffPanel};
use crate::error::{AppError, AppResult};
use crate::event::Event;
use crate::files_panel::FilesPanel;
use crate::git::repository::{self, DiffHunk, DiffSource};
use crate::state::{ContinuousDiff, Diff, FileSlot, Files, Focus, ReviewState, Screen, TreeRow};
use crate::store::DiffStore;
use crate::tui::theme::{self, Theme};

pub struct App {
    screen: Screen,
    focus: Focus,
    theme: Theme,
    repo: Repository,

    should_quit: bool,
    show_sidebar: bool,

    files: FilesPanel,
    diff: DiffPanel,

    store: DiffStore,
    repository_status: Option<repository::RepositoryStatus>,
    diff_source: DiffSource,
}

impl App {
    pub fn new(command: Option<Command>) -> AppResult<Self> {
        execute!(std::io::stdout(), SetTitle("marten"))?;
        let repo = Repository::discover(".").map_err(|source| {
            if source.code() == ErrorCode::NotFound {
                AppError::NotRepository { source }
            } else {
                AppError::git("open repository", source)
            }
        })?;

        let diff_source = match &command {
            Some(Command::Show { oid }) => {
                let revision = repository::resolve_revision(&repo, oid)?;
                DiffSource::Revision(revision)
            }
            None => DiffSource::Worktree,
        };

        Self::init(repo, diff_source)
    }

    fn init(repo: Repository, diff_source: DiffSource) -> AppResult<Self> {
        let repository_status = Some(
            repository::status(&repo)
                .map_err(|error| error.with_operation("read repository status"))?,
        );
        let operation = match diff_source {
            DiffSource::Worktree => "load working-tree changes",
            DiffSource::Revision(_) => "load revision changes",
        };
        let entries = repository::files_for_source(&repo, &diff_source)
            .map_err(|error| error.with_operation(operation))?;

        let mut store = DiffStore::new(entries);
        store.continuous_diff.rebuild_index();
        store.spawn_workers(&diff_source);

        let mut files = FilesPanel::new();
        files.ensure_rows(&store);
        files.select_first();

        let mut diff = DiffPanel::new();
        diff.refresh(&mut DiffContext {
            files: &mut files,
            store: &mut store,
            repo: &repo,
            diff_source: &diff_source,
        });

        let (width, _) = crossterm::terminal::size().unwrap_or((0, 0));

        let focus = if width <= 120 {
            Focus::Diff
        } else {
            Focus::Files
        };

        let show_sidebar = width > 120;

        Ok(Self {
            screen: Screen::Home,
            focus,
            files,
            diff,
            store,
            theme: theme::DEFAULT,
            repo,
            should_quit: false,
            show_sidebar,
            repository_status,
            diff_source,
        })
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

    pub const fn show_sidebar(&self) -> bool {
        self.show_sidebar
    }

    pub const fn repository_status(&self) -> Option<&repository::RepositoryStatus> {
        self.repository_status.as_ref()
    }

    pub const fn diff_source(&self) -> &DiffSource {
        &self.diff_source
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
            Event::Mouse(mouse) => self.handle_mouse(mouse),
            Event::Resize(width, _) => {
                if width <= 120 && self.focus != Focus::Diff {
                    Action::FocusPanel(Focus::Diff)
                } else {
                    Action::Noop
                }
            }
        }
    }

    pub fn update(&mut self, action: Action) -> AppResult<()> {
        let App {
            focus,
            files,
            diff,
            store,
            repo,
            repository_status,
            diff_source,
            show_sidebar,
            should_quit,
            ..
        } = self;

        match action {
            Action::Quit => {
                *should_quit = true;
                return Ok(());
            }
            Action::NextFocus => {
                *focus = focus.next();
                return Ok(());
            }
            Action::PreviousFocus => {
                *focus = focus.previous();
                return Ok(());
            }
            Action::FocusPanel(f) => {
                *focus = f;
                return Ok(());
            }
            Action::Refresh => {
                *repository_status = Some(
                    repository::status(repo)
                        .map_err(|error| error.with_operation("refresh repository status"))?,
                );
                diff.reload(&mut DiffContext {
                    files,
                    store,
                    repo,
                    diff_source,
                })?;
            }
            Action::ToggleSidebar => {
                *show_sidebar = !*show_sidebar;

                if *focus == Focus::Files {
                    *focus = Focus::Diff;
                } else {
                    *focus = Focus::Files;
                }
                return Ok(());
            }
            _ => {
                let selection_changed = files.update(action, *focus, store);
                diff.update(
                    action,
                    *focus,
                    selection_changed,
                    &mut DiffContext {
                        files,
                        store,
                        repo,
                        diff_source,
                    },
                );
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

        Ok(())
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

    fn handle_mouse(&mut self, mouse: MouseEvent) -> Action {
        match mouse.kind {
            MouseEventKind::ScrollUp => Action::MoveUp,
            MouseEventKind::ScrollDown => Action::MoveDown,
            _ => Action::Noop,
        }
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
            KeyCode::Char('s') => Action::ToggleSidebar,
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

        let mut app = App::init(repo, DiffSource::Worktree).unwrap();
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
