use crate::state::Focus;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    Noop,
    Quit,
    NextFocus,
    PreviousFocus,
    MoveDown,
    MoveUp,
    NextHunk,
    PreviousHunk,
    ToggleDiffLineNumbers,
    Refresh,
    GoToFirst,
    GoToLast,
    FocusPanel(Focus),
    ToggleCollapsed,
    ToggleViewMode,
    ForceLoadDiff,
    NextFile,
    PreviousFile,
}
