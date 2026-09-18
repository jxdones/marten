#[derive(Debug)]
pub enum AppError {
    Config {
        source: crate::config::ConfigError,
    },
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
    InvalidRange {
        range: String,
    },
    Git {
        operation: &'static str,
        source: git2::Error,
    },
    Io {
        operation: &'static str,
        source: std::io::Error,
    },
    Editor {
        editor: String,
        origin: crate::editor::Source,
        source: std::io::Error,
    },
}

pub type AppResult<T> = Result<T, AppError>;

impl AppError {
    pub const fn git(operation: &'static str, source: git2::Error) -> Self {
        Self::Git { operation, source }
    }

    pub const fn editor(
        editor: String,
        origin: crate::editor::Source,
        source: std::io::Error,
    ) -> Self {
        Self::Editor {
            editor,
            origin,
            source,
        }
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

impl From<crate::config::ConfigError> for AppError {
    fn from(source: crate::config::ConfigError) -> Self {
        Self::Config { source }
    }
}

impl std::fmt::Display for AppError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Config { source } => write!(formatter, "{source}"),
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
            Self::InvalidRange { range } => write!(
                formatter,
                "invalid revision range '{range}' (expected FROM..TO or FROM...TO)"
            ),
            Self::Git { operation, source } => {
                write!(formatter, "could not {operation}: {}", source.message())
            }
            Self::Io { operation, source } => {
                write!(formatter, "could not {operation}: {source}")
            }
            Self::Editor {
                editor,
                origin,
                source,
            } if source.kind() == std::io::ErrorKind::NotFound => match origin {
                crate::editor::Source::Fallback => write!(
                    formatter,
                    "could not open editor: $VISUAL and $EDITOR are not set and the fallback \
                     '{editor}' was not found in PATH"
                ),
                crate::editor::Source::Variable(variable) => write!(
                    formatter,
                    "could not open editor: '{editor}' from ${variable} was not found in PATH"
                ),
            },
            Self::Editor { editor, source, .. } => {
                write!(formatter, "could not open editor '{editor}': {source}")
            }
        }
    }
}

impl std::error::Error for AppError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Config { source } => Some(source),
            Self::NotRepository { source }
            | Self::RevisionNotFound { source, .. }
            | Self::RevisionNotCommit { source, .. }
            | Self::Git { source, .. } => Some(source),
            Self::InvalidRange { .. } => None,
            Self::Io { source, .. } | Self::Editor { source, .. } => Some(source),
        }
    }
}
