#[derive(Debug)]
pub struct Diff {
    pub viewport_height: usize,
    pub show_line_numbers: bool,
    pub too_large: Option<usize>,
}

impl Default for Diff {
    fn default() -> Self {
        Self {
            viewport_height: 1,
            show_line_numbers: true,
            too_large: None,
        }
    }
}

impl Diff {
    pub const fn toggle_line_numbers(&mut self) {
        self.show_line_numbers = !self.show_line_numbers;
    }

    pub fn set_viewport_height(&mut self, height: usize) {
        self.viewport_height = height.max(1);
    }
}
