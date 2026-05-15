use std::{collections::HashMap, path::Path};

use crate::git::GitResult;
use git2::{self, Diff, Patch, Reference, Repository, Status, StatusOptions};

const DEFAULT_HEAD: &str = "HEAD";

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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileStatus {
    Staged,
    Partial,
    Unstaged,
    Untracked,
    Conflicted,
}

impl FileStatus {
    pub fn label(self) -> &'static str {
        match self {
            Self::Staged => "STAGED",
            Self::Partial => "PARTIAL",
            Self::Conflicted => "CONFLICTED",
            Self::Unstaged => "UNSTAGED",
            Self::Untracked => "UNTRACKED",
        }
    }
}

pub fn status(path: impl AsRef<Path>) -> GitResult<RepositoryStatus> {
    let repo = Repository::discover(path)?;
    let name = repository_name(&repo);
    let (head, ahead, behind) = head_status(&repo)?;
    let changes = change_counts(&repo)?;

    Ok(RepositoryStatus {
        name,
        head,
        ahead,
        behind,
        changes,
    })
}

pub fn files(path: impl AsRef<Path>) -> GitResult<Vec<FileEntry>> {
    let repo = Repository::discover(path)?;

    let head = repo.head()?;
    let head_commit = head.peel_to_commit()?;
    let head_tree = head_commit.tree()?;

    let staged_diff = repo.diff_tree_to_index(Some(&head_tree), None, None)?;
    let unstaged_diff = repo.diff_index_to_workdir(None, None)?;

    let staged_map: HashMap<String, (usize, usize)> = diff_stats(&staged_diff)?;
    let unstaged_map: HashMap<String, (usize, usize)> = diff_stats(&unstaged_diff)?;

    let mut opts = StatusOptions::new();
    opts.include_untracked(true);

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
            FileStatus::Untracked | FileStatus::Conflicted => (0, 0),
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
    let commit_id = head
        .target()
        .map(|oid| oid.to_string().chars().take(7).collect())
        .unwrap_or_else(|| DEFAULT_HEAD.to_string());

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
    opts.include_untracked(true);

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
    let mut stats = HashMap::new();

    for (idx, delta) in diff.deltas().enumerate() {
        let Some(patch) = Patch::from_diff(diff, idx)? else {
            continue;
        };

        let (_, insertions, deletions) = patch.line_stats()?;
        let Some(path) = delta
            .new_file()
            .path()
            .or_else(|| delta.old_file().path()) // in case the file is deleted
            .and_then(|p| p.to_str())
        else {
            continue;
        };

        stats.insert(path.to_string(), (insertions, deletions));
    }

    Ok(stats)
}

// TODO: Add unit tests for status, head_status, repository_name, ahead_behind, and change_counts.
