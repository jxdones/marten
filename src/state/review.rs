use std::collections::HashMap;

use crate::git::repository::{DiffHunk, FileEntry, FileStatus};
use crate::state::{LineIndex, ViewMode};

const HEADER_ROW: usize = 1;
const CONTENT_ROW: usize = 1;
const HUNK_HEADER_ROW: usize = 0;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct FileKey {
    pub path: String,
    pub status: FileStatus,
}

#[derive(Debug)]
pub enum DiffLoadState {
    NotLoaded,
    Loading,
    Loaded {
        hunks: Vec<DiffHunk>,
        index: LineIndex,
    },
    TooLarge {
        lines: usize,
        forced: bool,
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

#[derive(Debug, Default)]
pub struct LoadingProgress {
    pub total: usize,
    pub completed: usize,
    pub active: bool,
    pub errors: usize,
}

#[derive(Debug)]
pub struct ReviewDoc {
    pub files: Vec<FileSlot>,
    pub by_key: HashMap<FileKey, usize>,
    pub index: ReviewIndex,
    pub index_dirty: bool,
    pub generation: u64,
    pub loading: LoadingProgress,
}

#[derive(Debug, Default)]
pub struct ReviewState {
    pub mode: ViewMode,
    pub continuous_scroll: usize,
    pub single_scroll: usize,
    pub selected_file: usize,
}

#[derive(Debug)]
pub enum RenderedRow {
    FileHeader {
        file_idx: usize,
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
    Loading {
        file_idx: usize,
    },
    TooLarge {
        file_idx: usize,
        lines: usize,
    },
    Error {
        file_idx: usize,
        msg: String,
    },
}

#[derive(Debug)]
pub struct WorkerResult {
    pub generation: u64,
    pub file_idx: usize,
    pub result: Result<(Vec<DiffHunk>, LineIndex), String>,
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
            DiffLoadState::NotLoaded
            | DiffLoadState::Loading
            | DiffLoadState::Error(_)
            | DiffLoadState::TooLarge { .. } => CONTENT_ROW + HEADER_ROW,
            DiffLoadState::Loaded { index, .. } => index.total_rows + HEADER_ROW,
        }
    }
}

impl ReviewDoc {
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
            Some(RenderedRow::FileHeader { file_idx })
        } else {
            let diff_row = local_row - 1;
            match &self.files[file_idx].load {
                DiffLoadState::Loaded { index, .. } => {
                    let (hunk_idx, line_in_hunk) = index.lookup(diff_row)?;
                    if line_in_hunk == HUNK_HEADER_ROW {
                        Some(RenderedRow::HunkHeader { file_idx, hunk_idx })
                    } else {
                        Some(RenderedRow::DiffLine {
                            file_idx,
                            hunk_idx,
                            line_idx: line_in_hunk - 1,
                        })
                    }
                }
                DiffLoadState::Loading | DiffLoadState::NotLoaded => {
                    Some(RenderedRow::Loading { file_idx })
                }
                DiffLoadState::TooLarge { lines, .. } => Some(RenderedRow::TooLarge {
                    file_idx,
                    lines: *lines,
                }),
                DiffLoadState::Error(msg) => Some(RenderedRow::Error {
                    file_idx,
                    msg: msg.clone(),
                }),
            }
        }
    }
}
