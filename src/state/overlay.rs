#[derive(Debug, Clone)]
pub enum Overlay {
    None,
    CommandPalette(CommandPaletteState),
    ThemeSelector(ThemeSelectorState),
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
