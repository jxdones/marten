use crate::state::Focus;

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
    RunSelectedCommand,
}
