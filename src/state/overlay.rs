use crate::tui::theme::ThemeFilter;

#[derive(Debug, Clone)]
pub enum Overlay {
    None,
    CommandPalette(CommandPaletteState),
    ThemeSelector(ThemeSelectorState),
    FileFinder(FileFinderState),
    CommitsFinder(CommitsFinderState),
}

#[derive(Debug, Clone, Default)]
pub struct CommandPaletteState {
    pub selected: usize,
}

#[derive(Debug, Clone, Default)]
pub struct ThemeSelectorState {
    pub selected: usize,
    pub original: usize,
    pub filter: ThemeFilter,
}

#[derive(Debug, Clone, Default)]
pub struct FileFinderState {
    pub query: String,
    pub selected: usize,
}

#[derive(Debug, Clone, Default)]
pub struct CommitsFinderState {
    pub query: String,
    pub selected: usize,
    pub focus: CommitsFinderFocus,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum CommitsFinderFocus {
    #[default]
    List,
    Search,
}

impl CommandPaletteState {
    pub const fn select_next(&mut self, len: usize) {
        if len == 0 {
            return;
        }
        self.selected = (self.selected + 1) % len;
    }

    pub const fn select_previous(&mut self, len: usize) {
        if len == 0 {
            return;
        }
        self.selected = (self.selected + len - 1) % len;
    }
}

impl FileFinderState {
    pub const fn select_next(&mut self, len: usize) {
        if len == 0 {
            return;
        }
        self.selected = (self.selected + 1) % len;
    }

    pub const fn select_previous(&mut self, len: usize) {
        if len == 0 {
            return;
        }
        self.selected = (self.selected + len - 1) % len;
    }

    pub fn insert(&mut self, character: char) {
        self.query.push(character);
        self.selected = 0;
    }

    pub fn backspace(&mut self) {
        self.query.pop();
        self.selected = 0;
    }

    pub fn clear(&mut self) {
        self.query.clear();
        self.selected = 0;
    }
}

impl CommitsFinderState {
    pub const fn select_next(&mut self, len: usize) {
        if len == 0 {
            return;
        }
        self.selected = (self.selected + 1) % len;
    }

    pub const fn select_previous(&mut self, len: usize) {
        if len == 0 {
            return;
        }
        self.selected = (self.selected + len - 1) % len;
    }

    pub fn insert(&mut self, character: char) {
        self.query.push(character);
        self.selected = 0;
    }

    pub fn backspace(&mut self) {
        self.query.pop();
        self.selected = 0;
    }

    pub fn clear(&mut self) {
        self.query.clear();
        self.selected = 0;
    }

    pub const fn toggle_focus(&mut self) {
        self.focus = match self.focus {
            CommitsFinderFocus::List => CommitsFinderFocus::Search,
            CommitsFinderFocus::Search => CommitsFinderFocus::List,
        };
    }
}
