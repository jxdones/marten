use ratatui::widgets::ListState;

use crate::git::repository::{FileEntry, FileStatus};

#[derive(Debug, Default)]
pub struct Files {
    pub list: ListState,
}

#[derive(Debug, Clone, Copy)]
pub enum FilePanelRow<'a> {
    Header {
        status: FileStatus,
        count: usize,
    },
    File {
        entry: &'a FileEntry,
        stats_width: usize,
    },
}

const STATUS_ORDER: [FileStatus; 5] = [
    FileStatus::Staged,
    FileStatus::Partial,
    FileStatus::Conflicted,
    FileStatus::Unstaged,
    FileStatus::Untracked,
];

impl Files {
    pub fn select_first(&mut self, rows: &[FilePanelRow<'_>]) {
        if let Some(row) = selectable_file_rows(rows).first() {
            self.list.select(Some(*row));
        }
    }

    pub fn select_next(&mut self, rows: &[FilePanelRow<'_>]) {
        let selectable = selectable_file_rows(rows);
        let Some(next) = next_selected_row(&selectable, self.list.selected()) else {
            self.list.select(None);
            return;
        };

        self.list.select(Some(next));
    }

    pub fn select_previous(&mut self, rows: &[FilePanelRow<'_>]) {
        let selectable = selectable_file_rows(rows);
        let Some(previous) = previous_selected_row(&selectable, self.list.selected()) else {
            self.list.select(None);
            return;
        };

        self.list.select(Some(previous));
    }
}

pub fn file_panel_rows(files: &[FileEntry]) -> Vec<FilePanelRow<'_>> {
    let mut rows = Vec::new();

    for status in STATUS_ORDER {
        let matching: Vec<_> = files.iter().filter(|file| file.status == status).collect();
        if matching.is_empty() {
            continue;
        }

        let stats_width = matching
            .iter()
            .map(|file| format!("+{} -{} ", file.insertions, file.deletions).len())
            .max()
            .unwrap_or(0);

        rows.push(FilePanelRow::Header {
            status,
            count: matching.len(),
        });

        for entry in matching {
            rows.push(FilePanelRow::File { entry, stats_width });
        }
    }

    rows
}

pub fn selectable_file_rows(rows: &[FilePanelRow<'_>]) -> Vec<usize> {
    rows.iter()
        .enumerate()
        .filter_map(|(index, row)| match row {
            FilePanelRow::Header { .. } => None,
            FilePanelRow::File { .. } => Some(index),
        })
        .collect()
}

fn next_selected_row(rows: &[usize], selected: Option<usize>) -> Option<usize> {
    if rows.is_empty() {
        return None;
    }

    let Some(selected) = selected else {
        return rows.first().copied();
    };

    rows.iter()
        .copied()
        .find(|row| *row > selected)
        .or_else(|| rows.first().copied())
}

fn previous_selected_row(rows: &[usize], selected: Option<usize>) -> Option<usize> {
    if rows.is_empty() {
        return None;
    }

    let Some(selected) = selected else {
        return rows.first().copied();
    };

    rows.iter()
        .rev()
        .copied()
        .find(|row| *row < selected)
        .or_else(|| rows.last().copied())
}

// TODO: Add tests for row grouping, selectable row indexes, and selection.
