use std::{collections::HashMap, fs, path::Path};

use crate::git::GitResult;
use git2::{self, Diff, DiffOptions, Patch, Reference, Repository, Status, StatusOptions};

const DEFAULT_HEAD: &str = "HEAD";
const SHORT_COMMIT_LEN: usize = 7;
pub const DIFF_LINE_THRESHOLD: usize = 15_000;

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

#[derive(Debug, Clone)]
pub enum Head {
    Branch(String),
    Detached(String),
    Unknown,
}

#[derive(Debug, Clone)]
pub struct FileEntry {
    pub path: String,
    pub status: FileStatus,
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
    pub insertions: usize,
    pub deletions: usize,
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

pub fn status(repo: &Repository) -> GitResult<RepositoryStatus> {
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

pub fn files(repo: &Repository) -> GitResult<Vec<FileEntry>> {
    let staged_diff = if let Ok(head) = repo.head() {
        // repo has at least one commit
        let tree = head.peel_to_commit()?.tree()?;
        repo.diff_tree_to_index(Some(&tree), None, None)?
    } else {
        // no commits yet, diff against empty tree
        repo.diff_tree_to_index(None, None, None)?
    };

    let unstaged_diff = repo.diff_index_to_workdir(None, None)?;

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
            status: file_status,
            insertions,
            deletions,
        });
    }

    Ok(entries)
}

pub fn file_diff_line_count(repo: &Repository, path: &str, status: FileStatus) -> GitResult<usize> {
    if status == FileStatus::Untracked {
        return Ok(untracked_file_content(repo, path)?.lines().count());
    }

    let head = repo.head()?;
    let mut opts = DiffOptions::new();
    opts.pathspec(path);

    let diff = match status {
        FileStatus::Staged | FileStatus::Partial => {
            let head_commit = head.peel_to_commit()?;
            let head_tree = head_commit.tree()?;
            repo.diff_tree_to_index(Some(&head_tree), None, Some(&mut opts))?
        }
        FileStatus::Unstaged => repo.diff_index_to_workdir(None, Some(&mut opts))?,
        _ => return Ok(0),
    };

    let mut count = 0;
    diff.foreach(
        &mut |_delta, _progress| true,
        None,
        None,
        Some(&mut |_delta, _hunk, line| {
            if [' ', '+', '-'].contains(&line.origin()) {
                count += 1;
            }
            true
        }),
    )?;

    Ok(count)
}

pub fn file_diff(repo: &Repository, path: &str, status: FileStatus) -> GitResult<Vec<DiffSection>> {
    match status {
        FileStatus::Staged => Ok(vec![DiffSection {
            kind: DiffSectionKind::Staged,
            hunks: staged_file_diff(repo, path)?,
        }]),
        FileStatus::Partial => Ok(vec![
            DiffSection {
                kind: DiffSectionKind::Staged,
                hunks: staged_file_diff(repo, path)?,
            },
            DiffSection {
                kind: DiffSectionKind::Unstaged,
                hunks: unstaged_file_diff(repo, path)?,
            },
        ]),
        FileStatus::Unstaged => Ok(vec![DiffSection {
            kind: DiffSectionKind::Unstaged,
            hunks: unstaged_file_diff(repo, path)?,
        }]),
        FileStatus::Untracked => Ok(vec![DiffSection {
            kind: DiffSectionKind::Unstaged,
            hunks: untracked_file_diff(repo, path)?,
        }]),
        FileStatus::Conflicted => Ok(vec![]),
    }
}

fn diff_hunks(diff: Diff<'_>) -> GitResult<Vec<DiffHunk>> {
    let Some(patch) = Patch::from_diff(&diff, 0)? else {
        return Ok(vec![]);
    };

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
        let (insertions, deletions) =
            diff_lines
                .iter()
                .fold((0, 0), |(insertion, deletion), line| match line.origin {
                    '+' => (insertion + 1, deletion),
                    '-' => (insertion, deletion + 1),
                    _ => (insertion, deletion),
                });
        hunks.push(DiffHunk {
            header: String::from_utf8_lossy(hunk.header()).to_string(),
            lines: diff_lines,
            insertions,
            deletions,
        });
    }

    Ok(hunks)
}

fn staged_file_diff(repo: &Repository, path: &str) -> GitResult<Vec<DiffHunk>> {
    let head = repo.head()?;
    let mut opts = DiffOptions::new();
    opts.pathspec(path);

    let head_commit = head.peel_to_commit()?;
    let head_tree = head_commit.tree()?;
    let diff = repo.diff_tree_to_index(Some(&head_tree), None, Some(&mut opts))?;

    let hunks = diff_hunks(diff)?;
    Ok(hunks)
}

fn unstaged_file_diff(repo: &Repository, path: &str) -> GitResult<Vec<DiffHunk>> {
    let mut opts = DiffOptions::new();
    opts.pathspec(path);

    let diff = repo.diff_index_to_workdir(None, Some(&mut opts))?;
    let hunks = diff_hunks(diff)?;
    Ok(hunks)
}

fn untracked_file_diff(repo: &Repository, path: &str) -> GitResult<Vec<DiffHunk>> {
    let content = untracked_file_content(repo, path)?;
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
    Ok(vec![DiffHunk {
        header: format!("@@ -0,0 +1,{insertions} @@ {path}"),
        lines,
        insertions,
        deletions: 0,
    }])
}

fn untracked_line_count(repo: &Repository, path: &str) -> GitResult<usize> {
    Ok(untracked_file_content(repo, path)?.lines().count())
}

fn untracked_file_content(repo: &Repository, path: &str) -> GitResult<String> {
    let path = repo.workdir().unwrap_or_else(|| Path::new(".")).join(path);
    let bytes = fs::read(path)?;
    Ok(String::from_utf8_lossy(&bytes).to_string())
}

fn head_status(repo: &Repository) -> GitResult<(Head, usize, usize)> {
    let head = match repo.head() {
        Ok(head) => head,
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
) -> GitResult<(usize, usize)> {
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

fn change_counts(repo: &Repository) -> GitResult<ChangeCounts> {
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

// TODO: Add unit tests for status, head_status, repository_name, ahead_behind, and change_counts.
