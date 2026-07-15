use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use git2::Repository;

use crate::git::repository::{self, DiffHunk, DiffSource, FileEntry};
use crate::state::{
    ContinuousDiff, DiffLoadState, FileKey, FileSlot, LineIndex, ReviewIndex, WorkerResult,
};

pub struct DiffStore {
    pub continuous_diff: ContinuousDiff,
    worker_tx: std::sync::mpsc::Sender<WorkerResult>,
    worker_rx: std::sync::mpsc::Receiver<WorkerResult>,
}

impl DiffStore {
    pub fn new(entries: Vec<FileEntry>) -> Self {
        let (tx, rx) = std::sync::mpsc::channel::<WorkerResult>();
        Self {
            continuous_diff: build_continuous_diff(entries, 0),
            worker_tx: tx,
            worker_rx: rx,
        }
    }

    pub fn reload(&mut self, entries: Vec<FileEntry>) {
        let next_generation = self.continuous_diff.generation + 1;
        let (tx, rx) = std::sync::mpsc::channel::<WorkerResult>();
        self.worker_tx = tx;
        self.worker_rx = rx;
        self.continuous_diff = build_continuous_diff(entries, next_generation);
        self.continuous_diff.rebuild_index();
    }

    pub fn poll_workers(&mut self) -> bool {
        let mut changed = false;
        while let Ok(msg) = self.worker_rx.try_recv() {
            if msg.generation != self.continuous_diff.generation {
                continue;
            }
            let slot = &mut self.continuous_diff.files[msg.file_idx];
            slot.load = match msg.result {
                Ok(Some((sections, hunks, index))) => DiffLoadState::Loaded {
                    sections,
                    hunks,
                    index,
                },
                Ok(None) => DiffLoadState::Binary,
                Err(e) => DiffLoadState::Error(e),
            };
            self.continuous_diff.index_dirty = true;
            changed = true;
        }
        changed
    }

    pub fn spawn_workers(&self, diff_source: &DiffSource) {
        let generation = self.continuous_diff.generation;
        let jobs: Vec<_> = self
            .continuous_diff
            .files
            .iter()
            .enumerate()
            .map(|(file_idx, file)| (file_idx, file.entry.path.clone(), file.entry.status))
            .collect();
        let queue = Arc::new(Mutex::new(jobs.into_iter()));
        let worker_count = std::thread::available_parallelism()
            .map_or(4, |n| n.get())
            .min(8);

        for _ in 0..worker_count {
            let tx = self.worker_tx.clone();
            let queue = Arc::clone(&queue);
            let diff_source = diff_source.clone();
            std::thread::spawn(move || {
                // Repository is !Send; open a fresh handle per worker thread.
                let Ok(repo) = Repository::discover(".") else {
                    return;
                };
                loop {
                    let job = queue.lock().unwrap().next();
                    let Some((file_idx, path, status)) = job else {
                        break;
                    };
                    let result =
                        repository::file_diff_for_source(&repo, &diff_source, &path, status)
                            .map(|maybe_sections| {
                                maybe_sections.map(|sections| {
                                    let hunks: Vec<DiffHunk> =
                                        sections.iter().flat_map(|s| s.hunks.clone()).collect();
                                    let index = LineIndex::new(&sections);
                                    (sections, hunks, index)
                                })
                            })
                            .map_err(|e| e.to_string());
                    if tx
                        .send(WorkerResult {
                            generation,
                            file_idx,
                            result,
                        })
                        .is_err()
                    {
                        break;
                    }
                }
            });
        }
    }
}

pub fn sort_entries(entries: &mut [FileEntry]) {
    entries.sort_by(|a, b| tree_sort_key(&a.path).cmp(&tree_sort_key(&b.path)));
}

fn tree_sort_key(path: &str) -> Vec<(u8, &str)> {
    let segments: Vec<&str> = path.split('/').collect();
    let last = segments.len().saturating_sub(1);
    segments
        .into_iter()
        .enumerate()
        .map(|(i, seg)| (if i == last { 1u8 } else { 0u8 }, seg))
        .collect()
}

fn build_continuous_diff(mut entries: Vec<FileEntry>, generation: u64) -> ContinuousDiff {
    sort_entries(&mut entries);

    let mut file_slots = Vec::new();
    let mut by_key = HashMap::new();

    for (idx, entry) in entries.into_iter().enumerate() {
        let file_key = FileKey {
            path: entry.path.clone(),
            status: entry.status,
        };
        by_key.insert(file_key, idx);
        file_slots.push(FileSlot {
            entry,
            load: DiffLoadState::NotLoaded,
        });
    }

    ContinuousDiff {
        files: file_slots,
        by_key,
        index: ReviewIndex::default(),
        index_dirty: false,
        generation,
    }
}
