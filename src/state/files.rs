use crate::git::repository::FileStatus;

#[derive(Debug, Default)]
pub struct Files {
    pub selected: Option<usize>,
}

pub const STATUS_ORDER: [FileStatus; 5] = [
    FileStatus::Staged,
    FileStatus::Partial,
    FileStatus::Conflicted,
    FileStatus::Unstaged,
    FileStatus::Untracked,
];

impl Files {
    pub fn select_first(&mut self, len: usize) {
        self.selected = (len > 0).then_some(0);
    }

    pub const fn select_last(&mut self, len: usize) {
        if len == 0 {
            self.selected = None;
            return;
        }

        match self.selected {
            None => self.selected = Some(0),
            Some(_) => self.selected = Some(len - 1),
        }
    }

    pub const fn select_next(&mut self, len: usize) {
        if len == 0 {
            self.selected = None;
            return;
        }
        match self.selected {
            None => self.selected = Some(0),
            Some(i) => self.selected = Some((i + 1) % len),
        }
    }

    pub const fn select_previous(&mut self, len: usize) {
        if len == 0 {
            self.selected = None;
            return;
        }

        match self.selected {
            None => self.selected = Some(0),
            Some(i) => self.selected = Some((i + len - 1) % len),
        }
    }
}

