#[derive(Debug, Default)]
pub struct Files {
    pub selected: Option<usize>,
    pub tree_row_count: usize,
}

impl Files {
    pub const fn select_first(&mut self) {
        self.selected = Some(0);
    }

    pub const fn select_last(&mut self) {
        if self.tree_row_count == 0 {
            self.selected = None;
            return;
        }

        match self.selected {
            None => self.selected = Some(0),
            Some(_) => self.selected = Some(self.tree_row_count - 1),
        }
    }

    pub const fn select_next(&mut self) {
        if self.tree_row_count == 0 {
            self.selected = None;
            return;
        }
        match self.selected {
            None => self.selected = Some(0),
            Some(i) => self.selected = Some((i + 1) % self.tree_row_count),
        }
    }

    pub const fn select_previous(&mut self) {
        if self.tree_row_count == 0 {
            self.selected = None;
            return;
        }

        match self.selected {
            None => self.selected = Some(0),
            Some(i) => self.selected = Some((i + self.tree_row_count - 1) % self.tree_row_count),
        }
    }
}
