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
}
