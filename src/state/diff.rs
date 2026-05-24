use crate::state::LineIndex;

#[derive(Debug)]
pub struct Diff {
    pub selected_hunk: Option<usize>,
    pub selected_line: usize,
    pub scroll_offset: usize,
    pub viewport_height: usize,
    pub show_line_numbers: bool,
    pub line_index: LineIndex,
    pub too_large: Option<usize>,
}

impl Default for Diff {
    fn default() -> Self {
        Self {
            selected_hunk: None,
            selected_line: 0,
            scroll_offset: 0,
            viewport_height: 1,
            show_line_numbers: true,
            line_index: LineIndex::new(&[]),
            too_large: None,
        }
    }
}

impl Diff {
    pub fn select_first_hunk(&mut self, len: usize) {
        self.selected_hunk = (len > 0).then_some(0);
        self.selected_line = 0;
        if self.selected_hunk.is_none() {
            self.scroll_offset = 0;
        }
    }

    pub const fn select_next_hunk(&mut self, len: usize) {
        if len == 0 {
            self.selected_hunk = None;
            self.selected_line = 0;
            return;
        }
        match self.selected_hunk {
            None => self.selected_hunk = Some(0),
            Some(i) => self.selected_hunk = Some((i + 1) % len),
        }
        self.selected_line = 0;
    }

    pub const fn select_previous_hunk(&mut self, len: usize) {
        if len == 0 {
            self.selected_hunk = None;
            self.selected_line = 0;
            return;
        }

        match self.selected_hunk {
            None => self.selected_hunk = Some(0),
            Some(i) => self.selected_hunk = Some((i + len - 1) % len),
        }
        self.selected_line = 0;
    }

    pub const fn select_hunk_line(&mut self, hunk: usize, line: usize) {
        self.selected_hunk = Some(hunk);
        self.selected_line = line;
    }

    pub const fn toggle_line_numbers(&mut self) {
        self.show_line_numbers = !self.show_line_numbers;
    }

    pub const fn set_scroll_offset(&mut self, offset: usize) {
        self.scroll_offset = offset;
    }

    pub fn set_viewport_height(&mut self, height: usize) {
        self.viewport_height = height.max(1);
    }
}
