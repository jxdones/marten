use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::action::Action;
use crate::event::Event;
use crate::state::{Focus, Screen};
use crate::tui::theme::{self, Theme};

#[derive(Debug)]
pub struct App {
    screen: Screen,
    focus: Focus,
    theme: Theme,
    should_quit: bool,
}

impl App {
    pub fn new() -> Self {
        Self {
            screen: Screen::Home,
            focus: Focus::Files,
            theme: theme::DEFAULT,
            should_quit: false,
        }
    }

    pub fn screen(&self) -> Screen {
        self.screen
    }

    pub fn focus(&self) -> Focus {
        self.focus
    }

    pub fn theme(&self) -> Theme {
        self.theme
    }

    pub fn should_quit(&self) -> bool {
        self.should_quit
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
