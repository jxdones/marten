use crate::state::Focus;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScrollDirection {
    Up,
    Down,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    Noop,
    Quit,
    NextFocus,
    PreviousFocus,
    MoveDown,
    MoveUp,
    ScrollDiffLeft,
    ScrollDiffRight,
    ScrollDiff {
        direction: ScrollDirection,
        lines: usize,
    },
    NextHunk,
    PreviousHunk,
    ToggleDiffLineNumbers,
    ToggleDiffLayout,
    Refresh,
    GoToFirst,
    GoToLast,
    FocusPanel(Focus),
    ToggleCollapsed,
    NextFile,
    PreviousFile,
    ToggleSidebar,
    ToggleCommandPalette,
    ToggleThemeSelector,
    ToggleFileFinder,
    SelectFileFinderResult,
    RunSelectedCommand,
    SelectTheme,
    OpenEditor,
    SelectTreeRow(usize),
    FileFinderInput(char),
    FileFinderBackspace,
    FileFinderClear,
    ToggleReviewed,
}
