#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Focus {
    Files,
    Branches,
    Stash,
    Diff,
    History,
    Details,
}

impl Focus {
    pub fn next(self) -> Self {
        match self {
            Self::Files => Self::Branches,
            Self::Branches => Self::Stash,
            Self::Stash => Self::Diff,
            Self::Diff => Self::History,
            Self::History => Self::Details,
            Self::Details => Self::Files,
        }
    }

    pub fn previous(self) -> Self {
        match self {
            Self::Files => Self::Details,
            Self::Branches => Self::Files,
            Self::Stash => Self::Branches,
            Self::Diff => Self::Stash,
            Self::History => Self::Diff,
            Self::Details => Self::History,
        }
    }
}
