pub mod repository;

#[derive(Debug)]
pub enum GitError {
    Git(git2::Error),
}

pub type GitResult<T> = Result<T, GitError>;

impl From<git2::Error> for GitError {
    fn from(error: git2::Error) -> Self {
        Self::Git(error)
    }
}
