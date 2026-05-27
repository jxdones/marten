use std::collections::HashMap;

use crate::git::repository::{DiffHunk, FileEntry, FileStatus};
use crate::state::{LineIndex, ViewMode};

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

impl ReviewIndex {
    pub fn file_at_row(&self, _global_row: usize) -> Option<(usize, usize)> {
        todo!()
    }
}
