use std::{collections::HashMap, fs, path::Path};

use crate::error::{AppError, AppResult};
use git2::{
    self, Diff, DiffFindOptions, DiffOptions, ErrorCode, Oid, Patch, Reference, Repository, Status,
    StatusOptions,
};

const DEFAULT_HEAD: &str = "HEAD";
const SHORT_COMMIT_LEN: usize = 7;
const COMMIT_HISTORY_LIMIT: usize = 1_000;
const MAX_ALIGNMENT_COMPARISONS: usize = 4096;
const MIN_IMPROVEMENT_PER_PAIR: f32 = 0.5;

#[derive(Debug, Clone)]
pub struct CommitInfo {
    pub oid: Oid,
    pub subject: String,
    pub committed_at: i64,
}

#[derive(Debug, Clone)]
pub struct RepositoryStatus {
    pub name: String,
    pub head: Head,
    pub ahead: usize,
    pub behind: usize,
    pub changes: ChangeCounts,
}

#[derive(Debug, Clone, Default)]
pub struct ChangeCounts {
    pub staged: usize,
    pub unstaged: usize,
    pub untracked: usize,
    pub conflicted: usize,
}

impl ChangeCounts {
    pub const fn is_dirty(&self) -> bool {
        self.staged + self.unstaged + self.untracked + self.conflicted > 0
    }
}

#[derive(Debug, Clone)]
pub enum Head {
    Branch(String),
    Detached(String),
    Unknown,
}

#[derive(Debug, Clone)]
pub struct FileEntry {
    pub path: String,
    pub previous_path: Option<String>,
    pub status: FileStatus,
    pub change: Option<FileChange>,
    pub type_change: Option<FileTypeChange>,
    pub insertions: usize,
    pub deletions: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FileStatus {
    Staged,
    Partial,
    Unstaged,
    Untracked,
    Conflicted,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileChange {
    Added,
    Modified,
    Deleted,
    Renamed,
    TypeChanged,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FileTypeChange {
    pub from: FileKind,
    pub to: FileKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileKind {
    File,
    Executable,
    Symlink,
    Submodule,
    Directory,
    Unknown,
}

impl FileChange {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Added => "ADDED",
            Self::Modified => "MODIFIED",
            Self::Deleted => "DELETED",
            Self::Renamed => "RENAMED",
            Self::TypeChanged => "TYPE CHANGED",
        }
    }
}

impl FileKind {
    pub const fn label(self) -> &'static str {
        match self {
            Self::File => "file",
            Self::Executable => "executable file",
            Self::Symlink => "symlink",
            Self::Submodule => "submodule",
            Self::Directory => "directory",
            Self::Unknown => "unknown",
        }
    }
}

impl From<git2::FileMode> for FileKind {
    fn from(mode: git2::FileMode) -> Self {
        match mode {
            git2::FileMode::Blob | git2::FileMode::BlobGroupWritable => Self::File,
            git2::FileMode::BlobExecutable => Self::Executable,
            git2::FileMode::Link => Self::Symlink,
            git2::FileMode::Commit => Self::Submodule,
            git2::FileMode::Tree => Self::Directory,
            git2::FileMode::Unreadable => Self::Unknown,
        }
    }
}

#[derive(Debug, Clone)]
pub struct DiffLine {
    pub old_lineno: Option<u32>,
    pub new_lineno: Option<u32>,
    pub origin: char,
    pub content: String,
}

#[derive(Debug, Clone)]
pub struct DiffHunk {
    pub header: String,
    pub lines: Vec<DiffLine>,
    pub comparison_rows: Vec<ComparisonRow>,
    pub insertions: usize,
    pub deletions: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ComparisonRow {
    pub old_line_idx: Option<usize>,
    pub new_line_idx: Option<usize>,
}

impl DiffHunk {
    pub fn new(header: String, lines: Vec<DiffLine>) -> Self {
        let (insertions, deletions) =
            lines
                .iter()
                .fold((0, 0), |(insertions, deletions), line| match line.origin {
                    '+' => (insertions + 1, deletions),
                    '-' => (insertions, deletions + 1),
                    _ => (insertions, deletions),
                });
        let comparison_rows = build_comparison_rows(&lines);

        Self {
            header,
            lines,
            comparison_rows,
            insertions,
            deletions,
        }
    }
}

fn build_comparison_rows(lines: &[DiffLine]) -> Vec<ComparisonRow> {
    let mut rows = Vec::new();
    let mut line_idx = 0;

    while line_idx < lines.len() {
        match lines[line_idx].origin {
            ' ' => {
                rows.push(ComparisonRow {
                    old_line_idx: Some(line_idx),
                    new_line_idx: Some(line_idx),
                });
                line_idx += 1;
            }
            '+' | '-' => {
                let mut old_lines = Vec::new();
                let mut new_lines = Vec::new();

                while line_idx < lines.len() && matches!(lines[line_idx].origin, '+' | '-') {
                    if lines[line_idx].origin == '-' {
                        old_lines.push(line_idx);
                    } else {
                        new_lines.push(line_idx);
                    }
                    line_idx += 1;
                }

                let offset = choose_alignment_offset(lines, &old_lines, &new_lines);
                rows.extend(comparison_rows_at_offset(&old_lines, &new_lines, offset));
            }
            '<' => {
                rows.push(ComparisonRow {
                    old_line_idx: Some(line_idx),
                    new_line_idx: None,
                });
                line_idx += 1;
            }
            '>' => {
                rows.push(ComparisonRow {
                    old_line_idx: None,
                    new_line_idx: Some(line_idx),
                });
                line_idx += 1;
            }
            _ => {
                rows.push(ComparisonRow {
                    old_line_idx: Some(line_idx),
                    new_line_idx: Some(line_idx),
                });
                line_idx += 1;
            }
        }
    }

    rows
}

fn comparison_rows_at_offset(
    old_lines: &[usize],
    new_lines: &[usize],
    offset: usize,
) -> Vec<ComparisonRow> {
    let row_count = old_lines.len().max(new_lines.len());
    let surplus = old_lines.len().abs_diff(new_lines.len());

    debug_assert!(offset <= surplus);

    let old_start = if new_lines.len() > old_lines.len() {
        offset
    } else {
        0
    };

    let new_start = if old_lines.len() > new_lines.len() {
        offset
    } else {
        0
    };

    (0..row_count)
        .map(|row_idx| ComparisonRow {
            old_line_idx: row_idx
                .checked_sub(old_start)
                .and_then(|idx| old_lines.get(idx))
                .copied(),
            new_line_idx: row_idx
                .checked_sub(new_start)
                .and_then(|idx| new_lines.get(idx))
                .copied(),
        })
        .collect()
}

fn line_similarity(old: &[char], new: &[char]) -> f32 {
    if old == new {
        return 1.0;
    }

    let min_len = old.len().min(new.len());
    let max_len = old.len().max(new.len());

    if min_len == 0 {
        return 0.0;
    }

    let prefix_len = old
        .iter()
        .zip(new.iter())
        .take_while(|(old_ch, new_ch)| old_ch == new_ch)
        .count();

    let suffix_len = old
        .iter()
        .rev()
        .zip(new.iter().rev())
        .take(min_len - prefix_len)
        .take_while(|(old_ch, new_ch)| old_ch == new_ch)
        .count();

    (prefix_len + suffix_len) as f32 / max_len as f32
}

fn normalized_line(line: &str) -> Vec<char> {
    line.chars().filter(|ch| !ch.is_whitespace()).collect()
}

fn choose_alignment_offset(lines: &[DiffLine], old_lines: &[usize], new_lines: &[usize]) -> usize {
    let pair_count = old_lines.len().min(new_lines.len());
    let surplus = old_lines.len().abs_diff(new_lines.len());

    let comparison_count = pair_count.saturating_mul(surplus.saturating_add(1));

    if pair_count == 0 || surplus == 0 || comparison_count > MAX_ALIGNMENT_COMPARISONS {
        return 0;
    }

    let normalized_old: Vec<_> = old_lines
        .iter()
        .map(|&idx| normalized_line(&lines[idx].content))
        .collect();
    let normalized_new: Vec<_> = new_lines
        .iter()
        .map(|&idx| normalized_line(&lines[idx].content))
        .collect();

    let longer_additions = new_lines.len() > old_lines.len();
    let score_offset = |offset: usize| {
        (0..pair_count)
            .map(|pair_idx| {
                let (old_idx, new_idx) = if longer_additions {
                    (pair_idx, pair_idx + offset)
                } else {
                    (pair_idx + offset, pair_idx)
                };

                line_similarity(&normalized_old[old_idx], &normalized_new[new_idx])
            })
            .sum::<f32>()
    };

    let mut best_offset = 0;
    let mut best_score = score_offset(0) + pair_count as f32 * MIN_IMPROVEMENT_PER_PAIR;

    for offset in 1..=surplus {
        let score = score_offset(offset);
        if score > best_score {
            best_offset = offset;
            best_score = score;
        }
    }

    best_offset
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiffSectionKind {
    Staged,
    Unstaged,
}

#[derive(Debug, Clone)]
pub struct DiffSection {
    pub kind: DiffSectionKind,
    pub hunks: Vec<DiffHunk>,
}

#[derive(Debug, Clone)]
pub enum DiffSource {
    Worktree,
    Revision(RevisionData),
    Range(RangeData),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RangeKind {
    Direct,
    MergeBase,
}

#[derive(Debug, Clone)]
pub struct RangeData {
    pub from_label: String,
    pub to_label: String,
    pub old_oid: Oid,
    pub new_oid: Oid,
    pub kind: RangeKind,
}

#[derive(Debug, Clone)]
pub struct RevisionData {
    pub oid: git2::Oid,
    pub short_oid: String,
    pub subject: String,
}

impl FileStatus {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Staged => "STAGED",
            Self::Partial => "PARTIAL",
            Self::Conflicted => "CONFLICTED",
            Self::Unstaged => "UNSTAGED",
            Self::Untracked => "UNTRACKED",
        }
    }
}

pub fn status(repo: &Repository) -> AppResult<RepositoryStatus> {
    let name = repository_name(repo);
    let (head, ahead, behind) = head_status(repo)?;
    let changes = change_counts(repo)?;

    Ok(RepositoryStatus {
        name,
        head,
        ahead,
        behind,
        changes,
    })
}

pub fn commits(repo: &Repository) -> AppResult<Vec<CommitInfo>> {
    let head = match repo.head() {
        Ok(head) => head,
        Err(error) if is_unknown_head_error(&error) => return Ok(Vec::new()),
        Err(error) => return Err(error.into()),
    };
    let head_oid = head.peel_to_commit()?.id();

    let mut revwalk = repo.revwalk()?;
    revwalk.push(head_oid)?;
    revwalk.set_sorting(git2::Sort::TOPOLOGICAL | git2::Sort::TIME)?;

    revwalk
        .take(COMMIT_HISTORY_LIMIT)
        .map(|oid| {
            let oid = oid?;
            let commit = repo.find_commit(oid)?;

            Ok(CommitInfo {
                oid,
                subject: commit.summary()?.unwrap_or_default().to_string(),
                committed_at: commit.time().seconds(),
            })
        })
        .collect()
}

pub fn files(repo: &Repository, ignore_whitespace: bool) -> AppResult<Vec<FileEntry>> {
    let mut staged_options = diff_options(ignore_whitespace);
    let staged_diff = match repo.head() {
        Ok(head) => {
            let tree = head.peel_to_commit()?.tree()?;
            repo.diff_tree_to_index(Some(&tree), None, Some(&mut staged_options))?
        }
        Err(error) if is_unknown_head_error(&error) => {
            // no commits yet, diff against empty tree
            repo.diff_tree_to_index(None, None, Some(&mut staged_options))?
        }
        Err(error) => return Err(error.into()),
    };

    let mut unstaged_options = diff_options(ignore_whitespace);
    let unstaged_diff = repo.diff_index_to_workdir(None, Some(&mut unstaged_options))?;

    let staged_map: HashMap<String, (usize, usize)> = diff_stats(&staged_diff)?;
    let unstaged_map: HashMap<String, (usize, usize)> = diff_stats(&unstaged_diff)?;

    let mut opts = StatusOptions::new();
    opts.include_untracked(true).recurse_untracked_dirs(true);

    let statuses = repo.statuses(Some(&mut opts))?;
    let mut entries = Vec::new();

    for entry in statuses.iter() {
        let status = entry.status();
        let path = entry.path().unwrap_or("").to_string();

        let file_status = if status.contains(Status::CONFLICTED) {
            FileStatus::Conflicted
        } else if status.contains(Status::WT_NEW) {
            FileStatus::Untracked
        } else {
            let is_staged = status.intersects(
                Status::INDEX_NEW
                    | Status::INDEX_MODIFIED
                    | Status::INDEX_DELETED
                    | Status::INDEX_RENAMED
                    | Status::INDEX_TYPECHANGE,
            );

            let is_unstaged = status.intersects(
                Status::WT_MODIFIED
                    | Status::WT_DELETED
                    | Status::WT_RENAMED
                    | Status::WT_TYPECHANGE,
            );
            match (is_staged, is_unstaged) {
                (true, true) => FileStatus::Partial,
                (true, false) => FileStatus::Staged,
                (false, true) => FileStatus::Unstaged,
                (false, false) => continue,
            }
        };

        let (insertions, deletions) = match file_status {
            FileStatus::Staged => staged_map.get(&path).copied().unwrap_or((0, 0)),
            FileStatus::Unstaged => unstaged_map.get(&path).copied().unwrap_or((0, 0)),
            FileStatus::Partial => {
                let (si, sd) = staged_map.get(&path).copied().unwrap_or((0, 0));
                let (ui, ud) = unstaged_map.get(&path).copied().unwrap_or((0, 0));
                (si + ui, sd + ud)
            }
            FileStatus::Untracked => (untracked_line_count(repo, &path)?, 0),
            FileStatus::Conflicted => (0, 0),
        };

        entries.push(FileEntry {
            path,
            previous_path: None,
            status: file_status,
            change: None,
            type_change: None,
            insertions,
            deletions,
        });
    }

    Ok(entries)
}

pub fn resolve_revision(repo: &Repository, input: &str) -> AppResult<RevisionData> {
    let object = repo.revparse_single(input).map_err(|source| {
        if source.code() == ErrorCode::NotFound {
            AppError::RevisionNotFound {
                revision: input.to_string(),
                source,
            }
        } else {
            AppError::git("resolve revision", source)
        }
    })?;
    let commit = object
        .peel_to_commit()
        .map_err(|source| AppError::RevisionNotCommit {
            revision: input.to_string(),
            source,
        })?;
    let oid = commit.id();

    Ok(RevisionData {
        oid,
        short_oid: oid.to_string().chars().take(SHORT_COMMIT_LEN).collect(),
        subject: commit.summary()?.unwrap_or_default().to_string(),
    })
}

fn parse_range(input: &str) -> AppResult<(&str, &str, RangeKind)> {
    let (from_input, to_input, kind) = if let Some((from, to)) = input.split_once("...") {
        (from, to, RangeKind::MergeBase)
    } else if let Some((from, to)) = input.split_once("..") {
        (from, to, RangeKind::Direct)
    } else {
        return Err(AppError::InvalidRange {
            range: input.to_string(),
        });
    };

    if from_input.is_empty() || to_input.is_empty() {
        return Err(AppError::InvalidRange {
            range: input.to_string(),
        });
    }

    Ok((from_input, to_input, kind))
}

pub fn resolve_range(repo: &Repository, input: &str) -> AppResult<RangeData> {
    let (from_input, to_input, kind) = parse_range(input)?;
    let from = resolve_revision(repo, from_input)?;
    let to = resolve_revision(repo, to_input)?;

    let old_oid = match kind {
        RangeKind::Direct => from.oid,
        RangeKind::MergeBase => repo
            .merge_base(from.oid, to.oid)
            .map_err(|error| AppError::git("find merge base", error))?,
    };
    let new_oid = to.oid;

    Ok(RangeData {
        from_label: from_input.to_string(),
        to_label: to_input.to_string(),
        old_oid,
        new_oid,
        kind,
    })
}

pub fn files_for_source(
    repo: &Repository,
    source: &DiffSource,
    ignore_whitespace: bool,
) -> AppResult<Vec<FileEntry>> {
    match source {
        DiffSource::Worktree => files(repo, ignore_whitespace),
        DiffSource::Revision(revision) => files_from_commit(repo, revision.oid, ignore_whitespace),
        DiffSource::Range(range) => {
            files_between_commits(repo, Some(range.old_oid), range.new_oid, ignore_whitespace)
        }
    }
}

pub fn files_from_commit(
    repo: &Repository,
    oid: Oid,
    ignore_whitespace: bool,
) -> AppResult<Vec<FileEntry>> {
    let commit = repo.find_commit(oid)?;

    let parent_oid = if commit.parent_count() > 0 {
        Some(commit.parent_id(0)?)
    } else {
        None
    };

    files_between_commits(repo, parent_oid, oid, ignore_whitespace)
}

fn files_between_commits(
    repo: &Repository,
    old_oid: Option<Oid>,
    new_oid: Oid,
    ignore_whitespace: bool,
) -> AppResult<Vec<FileEntry>> {
    let old_tree = if let Some(oid) = old_oid {
        Some(repo.find_commit(oid)?.tree()?)
    } else {
        None
    };

    let new_tree = repo.find_commit(new_oid)?.tree()?;
    let mut diff_options = diff_options(ignore_whitespace);
    diff_options.include_typechange(true);
    let mut diff =
        repo.diff_tree_to_tree(old_tree.as_ref(), Some(&new_tree), Some(&mut diff_options))?;
    find_renames(&mut diff)?;
    let staged_map: HashMap<String, (usize, usize)> = diff_stats(&diff)?;
    let mut entries = Vec::new();

    for delta in diff.deltas() {
        let path = delta
            .new_file()
            .path()
            .or_else(|| delta.old_file().path())
            .and_then(|p| p.to_str())
            .unwrap_or_default();

        let (insertions, deletions) = staged_map.get(path).copied().unwrap_or((0, 0));

        let change = match delta.status() {
            git2::Delta::Added | git2::Delta::Copied => FileChange::Added,
            git2::Delta::Deleted => FileChange::Deleted,
            git2::Delta::Renamed => FileChange::Renamed,
            git2::Delta::Typechange => FileChange::TypeChanged,
            _ => FileChange::Modified,
        };
        let previous_path = if change == FileChange::Renamed {
            delta
                .old_file()
                .path()
                .and_then(|path| path.to_str())
                .filter(|old_path| *old_path != path)
                .map(str::to_string)
        } else {
            None
        };
        let type_change = (change == FileChange::TypeChanged).then(|| FileTypeChange {
            from: delta.old_file().mode().into(),
            to: delta.new_file().mode().into(),
        });

        entries.push(FileEntry {
            path: path.to_string(),
            previous_path,
            status: FileStatus::Staged,
            change: Some(change),
            type_change,
            insertions,
            deletions,
        });
    }

    Ok(entries)
}

pub fn file_diff(
    repo: &Repository,
    path: &str,
    status: FileStatus,
    ignore_whitespace: bool,
) -> AppResult<Option<Vec<DiffSection>>> {
    match status {
        FileStatus::Staged => {
            let Some(hunks) = staged_file_diff(repo, path, ignore_whitespace)? else {
                return Ok(None);
            };
            Ok(Some(vec![DiffSection {
                kind: DiffSectionKind::Staged,
                hunks,
            }]))
        }
        FileStatus::Partial => {
            let Some(staged) = staged_file_diff(repo, path, ignore_whitespace)? else {
                return Ok(None);
            };
            let Some(unstaged) = unstaged_file_diff(repo, path, ignore_whitespace)? else {
                return Ok(None);
            };
            Ok(Some(vec![
                DiffSection {
                    kind: DiffSectionKind::Staged,
                    hunks: staged,
                },
                DiffSection {
                    kind: DiffSectionKind::Unstaged,
                    hunks: unstaged,
                },
            ]))
        }
        FileStatus::Unstaged => {
            let Some(hunks) = unstaged_file_diff(repo, path, ignore_whitespace)? else {
                return Ok(None);
            };
            Ok(Some(vec![DiffSection {
                kind: DiffSectionKind::Unstaged,
                hunks,
            }]))
        }
        FileStatus::Untracked => {
            let Some(hunks) = untracked_file_diff(repo, path)? else {
                return Ok(None);
            };
            Ok(Some(vec![DiffSection {
                kind: DiffSectionKind::Unstaged,
                hunks,
            }]))
        }
        FileStatus::Conflicted => Ok(Some(vec![])),
    }
}

pub fn files_diff_from_commit(
    repo: &Repository,
    oid: Oid,
    path: &str,
    previous_path: Option<&str>,
    ignore_whitespace: bool,
) -> AppResult<Option<Vec<DiffSection>>> {
    let commit = repo.find_commit(oid)?;

    let parent_oid = if commit.parent_count() > 0 {
        Some(commit.parent_id(0)?)
    } else {
        None
    };
    file_diff_between_commits(
        repo,
        parent_oid,
        oid,
        path,
        previous_path,
        ignore_whitespace,
    )
}

fn file_diff_between_commits(
    repo: &Repository,
    old_oid: Option<Oid>,
    new_oid: Oid,
    path: &str,
    previous_path: Option<&str>,
    ignore_whitespace: bool,
) -> AppResult<Option<Vec<DiffSection>>> {
    let old_tree = if let Some(oid) = old_oid {
        Some(repo.find_commit(oid)?.tree()?)
    } else {
        None
    };
    let new_tree = repo.find_commit(new_oid)?.tree()?;

    let mut opts = diff_options(ignore_whitespace);
    opts.include_typechange(true).pathspec(path);
    if let Some(previous_path) = previous_path {
        opts.pathspec(previous_path);
    }
    let mut diff = repo.diff_tree_to_tree(old_tree.as_ref(), Some(&new_tree), Some(&mut opts))?;
    find_renames(&mut diff)?;

    let Some(hunks) = diff_hunks(diff)? else {
        return Ok(None);
    };

    Ok(Some(vec![DiffSection {
        kind: DiffSectionKind::Staged,
        hunks,
    }]))
}

pub fn file_diff_for_source(
    repo: &Repository,
    source: &DiffSource,
    path: &str,
    previous_path: Option<&str>,
    status: FileStatus,
    ignore_whitespace: bool,
) -> AppResult<Option<Vec<DiffSection>>> {
    match source {
        DiffSource::Worktree => file_diff(repo, path, status, ignore_whitespace),
        DiffSource::Revision(revision) => {
            files_diff_from_commit(repo, revision.oid, path, previous_path, ignore_whitespace)
        }
        DiffSource::Range(range) => file_diff_between_commits(
            repo,
            Some(range.old_oid),
            range.new_oid,
            path,
            previous_path,
            ignore_whitespace,
        ),
    }
}

fn find_renames(diff: &mut Diff<'_>) -> AppResult<()> {
    let mut options = DiffFindOptions::new();
    options.renames(true);
    diff.find_similar(Some(&mut options))?;
    Ok(())
}

fn diff_hunks(diff: Diff<'_>) -> AppResult<Option<Vec<DiffHunk>>> {
    let Some(patch) = Patch::from_diff(&diff, 0)? else {
        return Ok(Some(vec![]));
    };

    if patch.delta().flags().is_binary() {
        return Ok(None);
    }

    let mut hunks: Vec<DiffHunk> = Vec::new();
    for hunk_idx in 0..patch.num_hunks() {
        let mut diff_lines: Vec<DiffLine> = Vec::new();
        let (hunk, num_lines) = patch.hunk(hunk_idx)?;

        for line_idx in 0..num_lines {
            let line = patch.line_in_hunk(hunk_idx, line_idx)?;
            diff_lines.push(DiffLine {
                old_lineno: line.old_lineno(),
                new_lineno: line.new_lineno(),
                origin: line.origin(),
                content: String::from_utf8_lossy(line.content()).to_string(),
            });
        }
        hunks.push(DiffHunk::new(
            String::from_utf8_lossy(hunk.header()).to_string(),
            diff_lines,
        ));
    }

    Ok(Some(hunks))
}

fn staged_file_diff(
    repo: &Repository,
    path: &str,
    ignore_whitespace: bool,
) -> AppResult<Option<Vec<DiffHunk>>> {
    let mut opts = diff_options(ignore_whitespace);
    opts.pathspec(path);

    let diff = match repo.head() {
        Ok(head) => {
            let head_tree = head.peel_to_commit()?.tree()?;
            repo.diff_tree_to_index(Some(&head_tree), None, Some(&mut opts))?
        }
        Err(error) if is_unknown_head_error(&error) => {
            // no commits yet, diff against empty tree
            repo.diff_tree_to_index(None, None, Some(&mut opts))?
        }
        Err(error) => return Err(error.into()),
    };

    let hunks = diff_hunks(diff)?;
    Ok(hunks)
}

fn unstaged_file_diff(
    repo: &Repository,
    path: &str,
    ignore_whitespace: bool,
) -> AppResult<Option<Vec<DiffHunk>>> {
    let mut opts = diff_options(ignore_whitespace);
    opts.pathspec(path);

    let diff = repo.diff_index_to_workdir(None, Some(&mut opts))?;
    let hunks = diff_hunks(diff)?;
    Ok(hunks)
}

fn untracked_file_diff(repo: &Repository, path: &str) -> AppResult<Option<Vec<DiffHunk>>> {
    let file_path = repo.workdir().unwrap_or_else(|| Path::new(".")).join(path);
    let bytes = fs::read(&file_path)?;
    if bytes.contains(&0u8) {
        return Ok(None);
    }
    let content = String::from_utf8_lossy(&bytes).into_owned();
    let lines = content
        .lines()
        .enumerate()
        .map(|(idx, line)| DiffLine {
            old_lineno: None,
            new_lineno: Some(u32::try_from(idx + 1).expect("line count exceeded u32::MAX")),
            origin: '+',
            content: line.to_string(),
        })
        .collect::<Vec<_>>();

    let insertions = lines.len();
    Ok(Some(vec![DiffHunk::new(
        format!("@@ -0,0 +1,{insertions} @@ {path}"),
        lines,
    )]))
}

fn untracked_line_count(repo: &Repository, path: &str) -> AppResult<usize> {
    Ok(untracked_file_content(repo, path)?
        .map(|content| content.lines().count())
        .unwrap_or(0))
}

fn untracked_file_content(repo: &Repository, path: &str) -> AppResult<Option<String>> {
    let path = repo.workdir().unwrap_or_else(|| Path::new(".")).join(path);
    if !path.is_file() {
        return Ok(None);
    }
    let bytes = fs::read(path)?;
    Ok(Some(String::from_utf8_lossy(&bytes).to_string()))
}

fn head_status(repo: &Repository) -> AppResult<(Head, usize, usize)> {
    let head = match repo.head() {
        Ok(head) => head,
        Err(error) if error.code() == git2::ErrorCode::UnbornBranch => {
            let name = repo
                .find_reference(DEFAULT_HEAD)?
                .symbolic_target()?
                .map(|target| {
                    target
                        .strip_prefix("refs/heads/")
                        .unwrap_or(target)
                        .to_string()
                })
                .unwrap_or_else(|| DEFAULT_HEAD.to_string());
            return Ok((Head::Branch(name), 0, 0));
        }
        Err(error) if is_unknown_head_error(&error) => {
            return Ok((Head::Unknown, 0, 0));
        }
        Err(error) => return Err(error.into()),
    };

    let branch = head.shorthand().unwrap_or(DEFAULT_HEAD);
    let detached = repo.head_detached()?;
    let (ahead, behind) = ahead_behind(repo, &head, branch, detached)?;
    let head = if detached {
        detached_head(&head)
    } else {
        Head::Branch(branch.to_string())
    };

    Ok((head, ahead, behind))
}

fn is_unknown_head_error(error: &git2::Error) -> bool {
    matches!(
        error.code(),
        git2::ErrorCode::UnbornBranch | git2::ErrorCode::NotFound
    )
}

fn detached_head(head: &Reference) -> Head {
    let commit_id = head.target().map_or_else(
        || DEFAULT_HEAD.to_string(),
        |oid| oid.to_string().chars().take(SHORT_COMMIT_LEN).collect(),
    );

    Head::Detached(commit_id)
}

fn repository_name(repo: &Repository) -> String {
    repo.workdir()
        .and_then(|path| path.file_name())
        .and_then(|name| name.to_str())
        .unwrap_or("unknown")
        .to_string()
}

fn ahead_behind(
    repo: &Repository,
    head: &Reference,
    branch: &str,
    detached: bool,
) -> AppResult<(usize, usize)> {
    if detached {
        return Ok((0, 0));
    }

    let branch = repo.find_branch(branch, git2::BranchType::Local)?;
    if let Ok(upstream) = branch.upstream() {
        let Some(local_oid) = head.target() else {
            return Ok((0, 0));
        };
        let Some(upstream_oid) = upstream.get().target() else {
            return Ok((0, 0));
        };

        repo.graph_ahead_behind(local_oid, upstream_oid)
            .map_err(Into::into)
    } else {
        Ok((0, 0))
    }
}

fn change_counts(repo: &Repository) -> AppResult<ChangeCounts> {
    let mut opts = StatusOptions::new();
    opts.include_untracked(true).recurse_untracked_dirs(true);

    let statuses = repo.statuses(Some(&mut opts))?;
    let mut changes = ChangeCounts::default();

    for entry in statuses.iter() {
        let status = entry.status();

        if status.contains(Status::CONFLICTED) {
            changes.conflicted += 1;
        } else {
            if status.intersects(
                Status::INDEX_NEW
                    | Status::INDEX_MODIFIED
                    | Status::INDEX_DELETED
                    | Status::INDEX_RENAMED
                    | Status::INDEX_TYPECHANGE,
            ) {
                changes.staged += 1;
            }

            if status.contains(Status::WT_NEW) {
                changes.untracked += 1;
            }

            if status.intersects(
                Status::WT_MODIFIED
                    | Status::WT_DELETED
                    | Status::WT_RENAMED
                    | Status::WT_TYPECHANGE,
            ) {
                changes.unstaged += 1;
            }
        }
    }

    Ok(changes)
}

fn diff_stats(diff: &Diff<'_>) -> Result<HashMap<String, (usize, usize)>, git2::Error> {
    let mut stats: HashMap<String, (usize, usize)> = HashMap::new();

    diff.foreach(
        &mut |_delta, _progress| true,
        None,
        None,
        Some(&mut |delta, _hunk, line| {
            let path = delta
                .new_file()
                .path()
                .or_else(|| delta.old_file().path())
                .and_then(|p| p.to_str())
                .unwrap_or_default();

            let entry = stats.entry(path.to_string()).or_insert((0, 0));
            match line.origin() {
                '+' => entry.0 += 1,
                '-' => entry.1 += 1,
                _ => {}
            }
            true
        }),
    )?;

    Ok(stats)
}

fn diff_options(ignore_whitespace: bool) -> DiffOptions {
    let mut options = DiffOptions::new();
    options.ignore_whitespace(ignore_whitespace);
    options
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::Path;

    use git2::{Repository, Signature};
    use tempfile::TempDir;

    use super::*;

    fn init_repo(dir_name: &str) -> (TempDir, Repository) {
        let parent = TempDir::new().unwrap();
        let repo_path = parent.path().join(dir_name);
        let mut opts = git2::RepositoryInitOptions::new();
        opts.initial_head("main");
        let repo = Repository::init_opts(&repo_path, &opts).unwrap();
        (parent, repo)
    }

    fn signature() -> Signature<'static> {
        Signature::now("Test User", "test@example.com").unwrap()
    }

    fn diff_line(origin: char, content: &str) -> DiffLine {
        DiffLine {
            old_lineno: None,
            new_lineno: None,
            origin,
            content: content.to_string(),
        }
    }

    fn write_file(repo: &Repository, path: &str, content: &str) {
        let workdir = repo.workdir().expect("bare repos are unsupported in tests");
        let full = workdir.join(path);
        if let Some(parent) = full.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(full, content).unwrap();
    }

    fn stage_path(repo: &Repository, path: &str) {
        let mut index = repo.index().unwrap();
        index
            .add_path(Path::new(path))
            .expect("failed to stage path");
        index.write().unwrap();
    }

    fn commit_index(repo: &Repository, message: &str) -> git2::Oid {
        let mut index = repo.index().unwrap();
        let tree_id = index.write_tree_to(repo).unwrap();
        let tree = repo.find_tree(tree_id).unwrap();
        let sig = signature();

        let parent = repo.head().ok().and_then(|head| head.target());
        if let Some(parent_id) = parent {
            let parent_commit = repo.find_commit(parent_id).unwrap();
            repo.commit(Some("HEAD"), &sig, &sig, message, &tree, &[&parent_commit])
                .unwrap()
        } else {
            repo.commit(Some("HEAD"), &sig, &sig, message, &tree, &[])
                .unwrap()
        }
    }

    fn write_and_commit(repo: &Repository, path: &str, content: &str, message: &str) -> git2::Oid {
        write_file(repo, path, content);
        stage_path(repo, path);
        commit_index(repo, message)
    }

    fn set_upstream(repo: &Repository, remote: &Repository, upstream_oid: git2::Oid) {
        let branch_name = repo
            .head()
            .unwrap()
            .shorthand()
            .expect("detached HEAD is unsupported in upstream tests")
            .to_string();

        if repo.find_remote("origin").is_err() {
            repo.remote("origin", remote.path().to_str().unwrap())
                .unwrap();
        }

        repo.reference(
            &format!("refs/remotes/origin/{branch_name}"),
            upstream_oid,
            true,
            "test upstream",
        )
        .unwrap();

        let mut branch = repo
            .find_branch(&branch_name, git2::BranchType::Local)
            .unwrap();
        branch
            .set_upstream(Some(&format!("origin/{branch_name}")))
            .unwrap();
    }

    fn stage_partial_file(repo: &Repository, path: &str, staged: &str, working: &str) {
        write_file(repo, path, staged);
        stage_path(repo, path);
        write_file(repo, path, working);
    }

    fn entry<'a>(entries: &'a [FileEntry], path: &str) -> &'a FileEntry {
        entries
            .iter()
            .find(|entry| entry.path == path)
            .unwrap_or_else(|| panic!("missing file entry for {path}"))
    }

    #[test]
    fn repository_name_uses_workdir_basename() {
        let (_dir, repo) = init_repo("my-fixture-repo");
        let status = status(&repo).unwrap();
        assert_eq!(status.name, "my-fixture-repo");
    }

    #[test]
    fn head_status_is_current_branch_on_unborn_branch() {
        let (_dir, repo) = init_repo("unborn");
        let status = status(&repo).unwrap();

        match status.head {
            Head::Branch(name) => assert_eq!(name, "main"),
            other => panic!("expected Head::Branch, got {other:?}"),
        }
        assert_eq!(status.ahead, 0);
        assert_eq!(status.behind, 0);
    }

    #[test]
    fn head_status_reports_branch_after_initial_commit() {
        let (_dir, repo) = init_repo("branch");
        write_and_commit(&repo, "README.md", "hello\n", "init");

        let status = status(&repo).unwrap();
        assert!(matches!(status.head, Head::Branch(_)));
        if let Head::Branch(name) = status.head {
            assert!(!name.is_empty());
        }
    }

    #[test]
    fn change_counts_are_zero_on_clean_repo() {
        let (_dir, repo) = init_repo("clean");
        write_and_commit(&repo, "README.md", "hello\n", "init");

        let status = status(&repo).unwrap();
        assert_eq!(status.changes.staged, 0);
        assert_eq!(status.changes.unstaged, 0);
        assert_eq!(status.changes.untracked, 0);
        assert_eq!(status.changes.conflicted, 0);
    }

    #[test]
    fn change_counts_track_staged_unstaged_and_untracked_files() {
        let (_dir, repo) = init_repo("counts");
        write_and_commit(&repo, "tracked.txt", "base\n", "init");

        write_file(&repo, "staged.txt", "staged\n");
        stage_path(&repo, "staged.txt");

        write_file(&repo, "tracked.txt", "base\nmodified\n");
        write_file(&repo, "untracked.txt", "new\n");

        let status = status(&repo).unwrap();
        assert_eq!(status.changes.staged, 1);
        assert_eq!(status.changes.unstaged, 1);
        assert_eq!(status.changes.untracked, 1);
    }

    #[test]
    fn ahead_behind_is_zero_without_upstream() {
        let (_dir, repo) = init_repo("no-upstream");
        write_and_commit(&repo, "README.md", "one\n", "first");
        write_and_commit(&repo, "README.md", "one\ntwo\n", "second");

        let status = status(&repo).unwrap();
        assert_eq!(status.ahead, 0);
        assert_eq!(status.behind, 0);
    }

    #[test]
    fn ahead_behind_counts_local_and_remote_divergence() {
        let parent = TempDir::new().unwrap();
        let remote = Repository::init_bare(parent.path().join("remote.git")).unwrap();
        let repo = Repository::init(parent.path().join("local")).unwrap();

        let first = write_and_commit(&repo, "README.md", "one\n", "first");
        let second = write_and_commit(&repo, "README.md", "one\ntwo\n", "second");

        set_upstream(&repo, &remote, first);
        let ahead = status(&repo).unwrap();
        assert_eq!(ahead.ahead, 1);
        assert_eq!(ahead.behind, 0);

        repo.reset(
            &repo.find_commit(first).unwrap().into_object(),
            git2::ResetType::Hard,
            None,
        )
        .unwrap();
        set_upstream(&repo, &remote, second);
        let behind = status(&repo).unwrap();
        assert_eq!(behind.ahead, 0);
        assert_eq!(behind.behind, 1);

        repo.reset(
            &repo.find_commit(second).unwrap().into_object(),
            git2::ResetType::Hard,
            None,
        )
        .unwrap();
        set_upstream(&repo, &remote, second);
        let even = status(&repo).unwrap();
        assert_eq!(even.ahead, 0);
        assert_eq!(even.behind, 0);
    }

    #[test]
    fn files_classifies_staged_unstaged_untracked_and_partial_entries() {
        let (_dir, repo) = init_repo("files");
        write_and_commit(&repo, "partial.txt", "base\n", "init");

        write_file(&repo, "staged.txt", "staged\n");
        stage_path(&repo, "staged.txt");

        stage_partial_file(
            &repo,
            "partial.txt",
            "base\nstaged\n",
            "base\nstaged\nworking\n",
        );

        write_file(&repo, "untracked.txt", "new\n");

        let entries = files(&repo, false).unwrap();
        assert_eq!(entry(&entries, "staged.txt").status, FileStatus::Staged);
        assert_eq!(
            entry(&entries, "untracked.txt").status,
            FileStatus::Untracked
        );
        assert_eq!(entry(&entries, "partial.txt").status, FileStatus::Partial);
    }

    #[test]
    fn file_diff_returns_staged_and_unstaged_sections_for_partial_files() {
        let (_dir, repo) = init_repo("partial-diff");
        write_and_commit(&repo, "partial.txt", "base\n", "init");
        stage_partial_file(
            &repo,
            "partial.txt",
            "base\nstaged\n",
            "base\nstaged\nworking\n",
        );

        let sections = file_diff(&repo, "partial.txt", FileStatus::Partial, false)
            .unwrap()
            .expect("not binary");
        assert_eq!(sections.len(), 2);
        assert_eq!(sections[0].kind, DiffSectionKind::Staged);
        assert_eq!(sections[1].kind, DiffSectionKind::Unstaged);
        assert!(!sections[0].hunks.is_empty());
        assert!(!sections[1].hunks.is_empty());
    }

    #[test]
    fn file_diff_renders_untracked_lines_as_insertions() {
        let (_dir, repo) = init_repo("untracked-diff");
        write_and_commit(&repo, "README.md", "hello\n", "init");
        write_file(&repo, "new.txt", "alpha\nbeta\n");

        let sections = file_diff(&repo, "new.txt", FileStatus::Untracked, false)
            .unwrap()
            .expect("not binary");
        assert_eq!(sections.len(), 1);
        assert_eq!(sections[0].kind, DiffSectionKind::Unstaged);

        let hunk = &sections[0].hunks[0];
        assert_eq!(hunk.insertions, 2);
        assert_eq!(hunk.deletions, 0);
        assert!(hunk.lines.iter().all(|line| line.origin == '+'));
    }

    #[test]
    fn file_diff_can_ignore_whitespace_only_worktree_changes() {
        let (_dir, repo) = init_repo("ignore-whitespace");
        write_and_commit(
            &repo,
            "main.rs",
            "fn main() {\n    println!(\"hello\");\n}\n",
            "init",
        );
        write_file(&repo, "main.rs", "fn main() {\n\tprintln!(\"hello\");\n}\n");

        let visible = file_diff(&repo, "main.rs", FileStatus::Unstaged, false)
            .unwrap()
            .expect("not binary");
        assert!(visible.iter().any(|section| !section.hunks.is_empty()));

        let ignored = file_diff(&repo, "main.rs", FileStatus::Unstaged, true)
            .unwrap()
            .expect("not binary");
        assert!(ignored.iter().all(|section| section.hunks.is_empty()));

        let visible_entries = files(&repo, false).unwrap();
        let visible_entry = entry(&visible_entries, "main.rs");
        assert!(visible_entry.insertions > 0);
        assert!(visible_entry.deletions > 0);

        let ignored_entries = files(&repo, true).unwrap();
        let ignored_entry = entry(&ignored_entries, "main.rs");
        assert_eq!((ignored_entry.insertions, ignored_entry.deletions), (0, 0));

        stage_path(&repo, "main.rs");
        let ignored_staged = file_diff(&repo, "main.rs", FileStatus::Staged, true)
            .unwrap()
            .expect("not binary");
        assert!(
            ignored_staged
                .iter()
                .all(|section| section.hunks.is_empty())
        );
    }

    #[test]
    fn comparison_rows_pair_a_deletion_with_the_similar_later_addition() {
        let lines = vec![
            diff_line('-', " return total(items, tax_rate);\n"),
            diff_line('+', " let items = get_items();\n"),
            diff_line('+', " let tax_rate = get_tax_rate();\n"),
            diff_line('+', " return total(items, tax_rate, discount);\n"),
        ];

        let rows = build_comparison_rows(&lines);

        assert_eq!(
            rows,
            vec![
                ComparisonRow {
                    old_line_idx: None,
                    new_line_idx: Some(1),
                },
                ComparisonRow {
                    old_line_idx: None,
                    new_line_idx: Some(2),
                },
                ComparisonRow {
                    old_line_idx: Some(0),
                    new_line_idx: Some(3),
                },
            ]
        );
    }

    #[test]
    fn realigns_when_deletions_are_longer() {
        let lines = vec![
            diff_line('-', "assert_ready(order);\n"),
            diff_line('-', "return compute_total(items, tax_rate);\n"),
            diff_line('-', "record_total(order);\n"),
            diff_line('+', "return compute_total(items, tax_rate, discount);\n"),
        ];

        assert_eq!(
            build_comparison_rows(&lines),
            vec![
                ComparisonRow {
                    old_line_idx: Some(0),
                    new_line_idx: None,
                },
                ComparisonRow {
                    old_line_idx: Some(1),
                    new_line_idx: Some(3),
                },
                ComparisonRow {
                    old_line_idx: Some(2),
                    new_line_idx: None,
                },
            ]
        );
    }
}
