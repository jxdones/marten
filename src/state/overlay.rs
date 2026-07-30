#[derive(Debug, Clone)]
pub enum Overlay {
    None,
    CommandPalette(CommandPaletteState),
    ThemeSelector(ThemeSelectorState),
    FileFinder(FileFinderState),
}

#[derive(Debug, Clone, Default)]
pub struct CommandPaletteState {
    pub selected: usize,
}

#[derive(Debug, Clone, Default)]
pub struct ThemeSelectorState {
    pub selected: usize,
    pub original: usize,
}

#[derive(Debug, Clone, Default)]
pub struct FileFinderState {
    pub query: String,
    pub selected: usize,
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

impl ThemeSelectorState {
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
