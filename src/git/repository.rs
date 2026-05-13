use std::path::Path;

use crate::git::GitResult;
use git2::{self, Reference, Repository, Status, StatusOptions};

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

// TODO: Add unit tests for status, head_status, repository_name, ahead_behind, and change_counts.
