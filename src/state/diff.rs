pub const SIDE_BY_SIDE_MIN_WIDTH: usize = 100;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiffLayout {
    Unified,
    SideBySide,
}

impl DiffLayout {
    pub const fn for_width(width: usize) -> Self {
        if width >= SIDE_BY_SIDE_MIN_WIDTH {
            Self::SideBySide
        } else {
            Self::Unified
        }
    }
}

#[derive(Debug)]
pub struct Diff {
    pub viewport_width: usize,
    pub viewport_height: usize,
    pub show_line_numbers: bool,
    pub horizontal_scroll: usize,
    max_horizontal_scroll: usize,
    layout_override: Option<DiffLayout>,
}

impl Default for Diff {
    fn default() -> Self {
        Self {
            viewport_width: 1,
            viewport_height: 1,
            show_line_numbers: true,
            horizontal_scroll: 0,
            max_horizontal_scroll: 0,
            layout_override: None,
        }
    }
}

impl Diff {
    pub const fn toggle_line_numbers(&mut self) {
        self.show_line_numbers = !self.show_line_numbers;
    }

    pub fn scroll_left(&mut self, amount: usize) {
        self.horizontal_scroll = self.horizontal_scroll.saturating_sub(amount);
    }

    pub fn scroll_right(&mut self, amount: usize) {
        self.horizontal_scroll = self
            .horizontal_scroll
            .saturating_add(amount)
            .min(self.max_horizontal_scroll);
    }

    pub fn set_viewport_height(&mut self, height: usize) {
        self.viewport_height = height.max(1);
    }

    pub fn set_viewport_width(&mut self, width: usize) -> bool {
        let width = width.max(1);
        if self.viewport_width == width {
            return false;
        }
        self.viewport_width = width;
        true
    }

    pub fn set_max_horizontal_scroll(&mut self, max: usize) {
        self.max_horizontal_scroll = max;
        self.horizontal_scroll = self.horizontal_scroll.min(max);
    }

    pub fn layout_for_width(&self, width: usize) -> DiffLayout {
        self.layout_override
            .unwrap_or_else(|| DiffLayout::for_width(width))
    }

    pub fn toggle_layout_override(&mut self, current: DiffLayout) -> DiffLayout {
        let layout = match current {
            DiffLayout::Unified => DiffLayout::SideBySide,
            DiffLayout::SideBySide => DiffLayout::Unified,
        };
        self.layout_override = Some(layout);
        layout
    }
}
