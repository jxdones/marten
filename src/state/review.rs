use std::collections::HashMap;

use crate::git::repository::{DiffHunk, DiffSectionKind, FileEntry, FileStatus};
use crate::state::line_index::IndexRow;
use crate::state::{DiffLayout, LineIndex};

const HEADER_ROW: usize = 1;
const CONTENT_ROW: usize = 1;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct FileKey {
    pub path: String,
    pub status: FileStatus,
}

#[derive(Debug)]
pub enum DiffLoadState {
    NotLoaded,
    #[allow(dead_code)]
    Loading,
    Loaded {
        hunks: Vec<DiffHunk>,
        index: LineIndex,
    },
    Binary,
    Error(String),
}

#[derive(Debug)]
pub struct FileSlot {
    pub entry: FileEntry,
    pub load: DiffLoadState,
    pub reviewed: bool,
    pub ignored: bool,
}

#[derive(Debug, Default)]
pub struct ReviewIndex {
    pub file_starts: Vec<usize>,
    pub total_rows: usize,
}

#[derive(Debug)]
pub struct ContinuousDiff {
    pub files: Vec<FileSlot>,
    pub by_key: HashMap<FileKey, usize>,
    pub index: ReviewIndex,
    pub index_dirty: bool,
    pub generation: u64,
    pub layout: DiffLayout,
}

#[derive(Debug, Default)]
pub struct ReviewState {
    pub continuous_scroll: usize,
}

#[derive(Debug)]
pub enum RenderedRow {
    FileHeader {
        file_idx: usize,
    },
    SectionHeader {
        kind: DiffSectionKind,
    },
    HunkHeader {
        file_idx: usize,
        hunk_idx: usize,
    },
    DiffRow {
        file_idx: usize,
        hunk_idx: usize,
        row_idx: usize,
    },
    Loading,
    Binary {
        file_idx: usize,
    },
    Error {
        msg: String,
    },
}

type DiffPayload = (Vec<DiffHunk>, LineIndex);

#[derive(Debug)]
pub struct WorkerResult {
    pub generation: u64,
    pub file_idx: usize,
    pub result: Result<Option<DiffPayload>, String>,
}

impl ReviewIndex {
    pub fn file_at_row(&self, global_row: usize) -> Option<(usize, usize)> {
        if self.file_starts.is_empty() || global_row >= self.total_rows {
            return None;
        }
        let file_idx = self
            .file_starts
            .partition_point(|&f| f <= global_row)
            .saturating_sub(1);
        let local_row = global_row - self.file_starts[file_idx];
        Some((file_idx, local_row))
    }
}

impl FileSlot {
    pub fn row_count(&self, layout: DiffLayout) -> usize {
        if self.reviewed || self.ignored {
            return HEADER_ROW;
        }
        match &self.load {
            DiffLoadState::Binary => HEADER_ROW,
            DiffLoadState::NotLoaded | DiffLoadState::Loading | DiffLoadState::Error(_) => {
                CONTENT_ROW + HEADER_ROW
            }
            DiffLoadState::Loaded { index, .. } => index.total_rows_for(layout) + HEADER_ROW,
        }
    }
}

impl ContinuousDiff {
    pub fn rebuild_index(&mut self) {
        let mut file_starts = Vec::new();
        let mut offset = 0;
        for file_slot in &self.files {
            file_starts.push(offset);
            offset += file_slot.row_count(self.layout);
        }
        self.index.file_starts = file_starts;
        self.index.total_rows = offset;
    }

    pub fn lookup_row(&self, global_row: usize) -> Option<RenderedRow> {
        let (file_idx, local_row) = self.index.file_at_row(global_row)?;

        if local_row == 0 {
            if matches!(self.files[file_idx].load, DiffLoadState::Binary) {
                Some(RenderedRow::Binary { file_idx })
            } else {
                Some(RenderedRow::FileHeader { file_idx })
            }
        } else {
            let diff_row = local_row - 1;
            match &self.files[file_idx].load {
                DiffLoadState::Loaded { hunks, index } => {
                    match index.lookup_for(diff_row, self.layout)? {
                        IndexRow::SectionHeader(section_idx) => {
                            let kind = index.section_header_rows[section_idx].1;
                            Some(RenderedRow::SectionHeader { kind })
                        }
                        IndexRow::HunkHeader(hunk_idx) => {
                            hunks.get(hunk_idx)?;
                            Some(RenderedRow::HunkHeader { file_idx, hunk_idx })
                        }
                        IndexRow::DiffLine(hunk_idx, row_idx) => {
                            let hunk = hunks.get(hunk_idx)?;
                            let row_count = match self.layout {
                                DiffLayout::Unified => hunk.lines.len(),
                                DiffLayout::SideBySide => hunk.comparison_rows.len(),
                            };
                            (row_idx < row_count).then_some(RenderedRow::DiffRow {
                                file_idx,
                                hunk_idx,
                                row_idx,
                            })
                        }
                    }
                }
                DiffLoadState::Loading | DiffLoadState::NotLoaded => Some(RenderedRow::Loading),
                DiffLoadState::Binary => {
                    debug_assert!(
                        false,
                        "binary slot has row_count=1, local_row>0 unreachable"
                    );
                    None
                }
                DiffLoadState::Error(msg) => Some(RenderedRow::Error { msg: msg.clone() }),
            }
        }
    }

    pub fn selected_line(&self, scroll: usize) -> Option<(&str, u32)> {
        let (file_idx, hunk_idx, row_idx) = match self.lookup_row(scroll)? {
            RenderedRow::DiffRow {
                file_idx,
                hunk_idx,
                row_idx,
            } => (file_idx, hunk_idx, Some(row_idx)),
            RenderedRow::HunkHeader { file_idx, hunk_idx } => (file_idx, hunk_idx, None),
            _ => return None,
        };

        let DiffLoadState::Loaded { hunks, .. } = &self.files[file_idx].load else {
            return None;
        };

        let hunk = hunks.get(hunk_idx)?;
        let new_lineno = match row_idx {
            Some(row_idx) => match self.layout {
                DiffLayout::Unified => hunk.lines.get(row_idx)?.new_lineno?,
                DiffLayout::SideBySide => {
                    let idx = hunk.comparison_rows.get(row_idx)?.new_line_idx?;
                    hunk.lines.get(idx)?.new_lineno?
                }
            },
            None => hunk
                .lines
                .iter()
                .find(|line| line.origin == '+')
                .and_then(|line| line.new_lineno)?,
        };

        Some((self.files[file_idx].entry.path.as_str(), new_lineno))
    }

    pub fn toggle_reviewed(&mut self, file_idx: usize) {
        if let Some(file) = self.files.get_mut(file_idx) {
            file.reviewed = !file.reviewed;
        }
    }

    pub fn next_unreviewed_after(&self, file_idx: usize) -> Option<usize> {
        self.files
            .get(file_idx + 1..)?
            .iter()
            .position(|file| !file.reviewed)
            .map(|offset| file_idx + 1 + offset)
    }
}
