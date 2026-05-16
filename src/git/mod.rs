pub mod repository;

#[derive(Debug)]
pub enum GitError {
    Git(git2::Error),
    Io(std::io::Error),
}

pub type GitResult<T> = Result<T, GitError>;

impl From<git2::Error> for GitError {
    fn from(error: git2::Error) -> Self {
        Self::Git(error)
    }
}

impl From<std::io::Error> for GitError {
    fn from(error: std::io::Error) -> Self {
        Self::Io(error)
    }
}

impl std::fmt::Display for GitError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Git(error) => write!(formatter, "{error}"),
            Self::Io(error) => write!(formatter, "{error}"),
        }
    }
}

impl std::error::Error for GitError {}
