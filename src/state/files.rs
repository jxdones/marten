use crate::git::repository::{FileEntry, FileStatus};

#[derive(Debug, Default)]
pub struct Files {
    pub selected: Option<usize>,
}

#[derive(Debug, Clone, Copy)]
pub enum FilePanelRow<'a> {
    Header { status: FileStatus, count: usize },
    File { entry: &'a FileEntry },
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

pub fn file_panel_rows(files: &[FileEntry]) -> Vec<FilePanelRow<'_>> {
    let mut rows = Vec::new();

    for status in STATUS_ORDER {
        let matching: Vec<_> = files.iter().filter(|file| file.status == status).collect();
        if matching.is_empty() {
            continue;
        }

        rows.push(FilePanelRow::Header {
            status,
            count: matching.len(),
        });

        for entry in matching {
            rows.push(FilePanelRow::File { entry });
        }
    }

    rows
}
