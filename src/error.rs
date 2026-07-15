#[derive(Debug)]
pub enum AppError {
    NotRepository {
        source: git2::Error,
    },
    RevisionNotFound {
        revision: String,
        source: git2::Error,
    },
    RevisionNotCommit {
        revision: String,
        source: git2::Error,
    },
    Git {
        operation: &'static str,
        source: git2::Error,
    },
    Io {
        operation: &'static str,
        source: std::io::Error,
    },
}

pub type AppResult<T> = Result<T, AppError>;

impl AppError {
    pub const fn git(operation: &'static str, source: git2::Error) -> Self {
        Self::Git { operation, source }
    }

    pub fn with_operation(self, operation: &'static str) -> Self {
        match self {
            Self::Git { source, .. } => Self::Git { operation, source },
            Self::Io { source, .. } => Self::Io { operation, source },
            error => error,
        }
    }
}

impl From<git2::Error> for AppError {
    fn from(source: git2::Error) -> Self {
        Self::Git {
            operation: "complete Git operation",
            source,
        }
    }
}

impl From<std::io::Error> for AppError {
    fn from(source: std::io::Error) -> Self {
        Self::Io {
            operation: "complete terminal operation",
            source,
        }
    }
}

impl std::fmt::Display for AppError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotRepository { .. } => {
                write!(
                    formatter,
                    "no Git repository found (run marten from inside one)"
                )
            }
            Self::RevisionNotFound { revision, .. } => write!(
                formatter,
                "revision '{revision}' not found (expected a SHA, branch, tag, or expression like HEAD~2)"
            ),
            Self::RevisionNotCommit { revision, .. } => {
                write!(
                    formatter,
                    "revision '{revision}' does not point to a commit"
                )
            }
            Self::Git { operation, source } => {
                write!(formatter, "could not {operation}: {}", source.message())
            }
            Self::Io { operation, source } => {
                write!(formatter, "could not {operation}: {source}")
            }
        }
    }
}

impl std::error::Error for AppError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::NotRepository { source }
            | Self::RevisionNotFound { source, .. }
            | Self::RevisionNotCommit { source, .. }
            | Self::Git { source, .. } => Some(source),
            Self::Io { source, .. } => Some(source),
        }
    }
}
