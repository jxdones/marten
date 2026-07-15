#[derive(Debug, Default)]
pub struct Files {
    pub selected: Option<usize>,
    pub tree_row_count: usize,
}

impl Files {
    pub const fn select_first(&mut self) {
        self.selected = if self.tree_row_count == 0 {
            None
        } else {
            Some(0)
        };
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

#[cfg(test)]
mod tests {
    use super::Files;

    #[test]
    fn select_first_clears_selection_when_there_are_no_rows() {
        let mut files = Files {
            selected: Some(0),
            tree_row_count: 0,
        };

        files.select_first();

        assert_eq!(files.selected, None);
    }

    #[test]
    fn select_first_selects_the_first_available_row() {
        let mut files = Files {
            selected: None,
            tree_row_count: 2,
        };

        files.select_first();

        assert_eq!(files.selected, Some(0));
    }
}
