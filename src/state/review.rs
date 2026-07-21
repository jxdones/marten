use std::collections::HashMap;

use crate::git::repository::{DiffHunk, DiffSection, DiffSectionKind, FileEntry, FileStatus};
use crate::state::LineIndex;
use crate::state::line_index::IndexRow;

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
    TooLarge {
        lines: usize,
    },
    Error(String),
}

#[derive(Debug)]
pub struct FileSlot {
    pub entry: FileEntry,
    pub load: DiffLoadState,
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
    DiffLine {
        file_idx: usize,
        hunk_idx: usize,
        line_idx: usize,
    },
    Loading,
    Binary {
        file_idx: usize,
    },
    TooLarge {
        lines: usize,
    },
    Error {
        msg: String,
    },
}

type DiffPayload = (Vec<DiffSection>, Vec<DiffHunk>, LineIndex);

#[derive(Debug)]
pub struct WorkerResult {
    pub generation: u64,
    pub file_idx: usize,
    pub result: Result<Option<DiffPayload>, String>,
}

impl ReviewIndex {
    pub fn file_at_row(&self, global_row: usize) -> Option<(usize, usize)> {
        if self.file_starts.is_empty() {
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
    pub fn row_count(&self) -> usize {
        match &self.load {
            DiffLoadState::Binary => HEADER_ROW,
            DiffLoadState::NotLoaded
            | DiffLoadState::Loading
            | DiffLoadState::Error(_)
            | DiffLoadState::TooLarge { .. } => CONTENT_ROW + HEADER_ROW,
            DiffLoadState::Loaded { index, .. } => index.total_rows + HEADER_ROW,
        }
    }
}

impl ContinuousDiff {
    pub fn rebuild_index(&mut self) {
        let mut file_starts = Vec::new();
        let mut offset = 0;
        for file_slot in &self.files {
            file_starts.push(offset);
            offset += file_slot.row_count();
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
                DiffLoadState::Loaded { index, .. } => match index.lookup(diff_row)? {
                    IndexRow::SectionHeader(section_idx) => {
                        let kind = index.section_header_rows[section_idx].1;
                        Some(RenderedRow::SectionHeader { kind })
                    }
                    IndexRow::HunkHeader(hunk_idx) => {
                        Some(RenderedRow::HunkHeader { file_idx, hunk_idx })
                    }
                    IndexRow::DiffLine(hunk_idx, line_idx) => Some(RenderedRow::DiffLine {
                        file_idx,
                        hunk_idx,
                        line_idx,
                    }),
                },
                DiffLoadState::Loading | DiffLoadState::NotLoaded => Some(RenderedRow::Loading),
                DiffLoadState::Binary => {
                    debug_assert!(
                        false,
                        "binary slot has row_count=1, local_row>0 unreachable"
                    );
                    None
                }
                DiffLoadState::TooLarge { lines, .. } => {
                    Some(RenderedRow::TooLarge { lines: *lines })
                }
                DiffLoadState::Error(msg) => Some(RenderedRow::Error { msg: msg.clone() }),
            }
        }
    }
}
