use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::action::Action;
use crate::event::Event;
use crate::git::repository::{self, FileEntry, RepositoryStatus};
use crate::state::{Files, Focus, Screen};
use crate::tui::theme::{self, Theme};

#[derive(Debug)]
pub struct App {
    screen: Screen,
    focus: Focus,
    files_state: Files,

    theme: Theme,
    should_quit: bool,

    repository_status: Option<RepositoryStatus>,
    files: Option<Vec<FileEntry>>,
}

impl App {
    pub fn new() -> Self {
        let repository_status = repository::status(".").ok();
        let files = repository::files(".").ok();
        Self {
            screen: Screen::Home,
            focus: Focus::Files,
            files_state: Files::default(),
            theme: theme::DEFAULT,
            should_quit: false,
            repository_status,
            files,
        }
    }

    pub fn screen(&self) -> Screen {
        self.screen
    }

    pub fn focus(&self) -> Focus {
        self.focus
    }

    pub fn files_state_mut(&mut self) -> &mut Files {
        &mut self.files_state
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
        }
    }

    fn handle_key(&self, key: KeyEvent) -> Action {
        match key.code {
            KeyCode::Char('q') => Action::Quit,
            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => Action::Quit,
            KeyCode::Tab => Action::NextFocus,
            KeyCode::BackTab => Action::PreviousFocus,
            _ => Action::Noop,
        }
    }
}
