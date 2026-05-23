#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Focus {
    Files,
    Diff,
}

impl Focus {
    pub const fn next(self) -> Self {
        match self {
            Self::Files => Self::Diff,
            Self::Diff => Self::Files,
        }
    }

    pub const fn previous(self) -> Self {
        match self {
            Self::Files => Self::Diff,
            Self::Diff => Self::Files,
        }
    }
}
