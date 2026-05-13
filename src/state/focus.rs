#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Focus {
    Files,
    Diff,
    History,
    Details,
}

impl Focus {
    pub fn next(self) -> Self {
        match self {
            Self::Files => Self::Diff,
            Self::Diff => Self::History,
            Self::History => Self::Details,
            Self::Details => Self::Files,
        }
    }

    pub fn previous(self) -> Self {
        match self {
            Self::Files => Self::Details,
            Self::Diff => Self::Files,
            Self::History => Self::Diff,
            Self::Details => Self::History,
        }
    }
}
