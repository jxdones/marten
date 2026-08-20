use crate::action::Action;
use crate::state::Overlay;

#[derive(Debug, Clone, Copy)]
pub enum Section {
    Navigation,
    Diff,
    Layout,
    General,
    Settings,
}

impl Section {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Navigation => "Navigation",
            Self::Diff => "Diff",
            Self::Layout => "Layout",
            Self::General => "General",
            Self::Settings => "Settings",
        }
    }
}

pub struct CommandItem {
    pub label: &'static str,
    pub description: &'static str,
    pub keybind: &'static str,
    pub action: Action,
}

pub struct CommandGroup {
    pub section: Section,
    pub items: &'static [CommandItem],
}

pub fn update(overlay: &mut Overlay, action: Action) {
    let Overlay::CommandPalette(state) = overlay else {
        return;
    };

    match action {
        Action::MoveDown => state.select_next(command_count()),
        Action::MoveUp => state.select_previous(command_count()),
        _ => {}
    }
}

pub fn command_groups() -> &'static [CommandGroup] {
    &[
        CommandGroup {
            section: Section::Navigation,
            items: &[
                CommandItem {
                    label: "find file",
                    description: "find and jump to a changed file",
                    keybind: "ctrl+p",
                    action: Action::ToggleFileFinder,
                },
                CommandItem {
                    label: "commit history",
                    description: "browse commits and open a historical diff",
                    keybind: "H",
                    action: Action::ToggleCommitsFinder,
                },
                CommandItem {
                    label: "next file",
                    description: "select the next changed file",
                    keybind: "n",
                    action: Action::NextFile,
                },
                CommandItem {
                    label: "previous file",
                    description: "select the previous changed file",
                    keybind: "p",
                    action: Action::PreviousFile,
                },
                CommandItem {
                    label: "next hunk",
                    description: "jump to the next diff hunk",
                    keybind: "]",
                    action: Action::NextHunk,
                },
                CommandItem {
                    label: "previous hunk",
                    description: "jump to the previous diff hunk",
                    keybind: "[",
                    action: Action::PreviousHunk,
                },
            ],
        },
        CommandGroup {
            section: Section::Diff,
            items: &[
                CommandItem {
                    label: "scroll diff left",
                    description: "show earlier columns in the diff",
                    keybind: "h / ←",
                    action: Action::ScrollDiffLeft,
                },
                CommandItem {
                    label: "scroll diff right",
                    description: "show later columns in the diff",
                    keybind: "l / →",
                    action: Action::ScrollDiffRight,
                },
                CommandItem {
                    label: "edit hunk",
                    description: "edit hunk on your editor",
                    keybind: "e",
                    action: Action::OpenEditor,
                },
                CommandItem {
                    label: "mark reviewed",
                    description: "mark file as reviewd and collapses it",
                    keybind: "m",
                    action: Action::ToggleReviewed,
                },
            ],
        },
        CommandGroup {
            section: Section::Layout,
            items: &[
                CommandItem {
                    label: "toggle sidebar",
                    description: "show or hide the files sidebar",
                    keybind: "s",
                    action: Action::ToggleSidebar,
                },
                CommandItem {
                    label: "toggle numbers",
                    description: "show or hide diff line numbers",
                    keybind: "L",
                    action: Action::ToggleDiffLineNumbers,
                },
                CommandItem {
                    label: "cycle diff layout",
                    description: "cycle between auto, side-by-side, and unified",
                    keybind: "v",
                    action: Action::ToggleDiffLayout,
                },
            ],
        },
        CommandGroup {
            section: Section::General,
            items: &[
                CommandItem {
                    label: "reload",
                    description: "reload repository status and diff",
                    keybind: "r",
                    action: Action::Refresh,
                },
                CommandItem {
                    label: "quit",
                    description: "exit marten",
                    keybind: "q",
                    action: Action::Quit,
                },
            ],
        },
        CommandGroup {
            section: Section::Settings,
            items: &[CommandItem {
                label: "change theme",
                description: "preview and select a theme",
                keybind: "t",
                action: Action::ToggleThemeSelector,
            }],
        },
    ]
}

pub fn command_count() -> usize {
    command_groups().iter().map(|group| group.items.len()).sum()
}

pub fn selected_action(overlay: &Overlay) -> Option<Action> {
    let Overlay::CommandPalette(state) = overlay else {
        return None;
    };

    command_groups()
        .iter()
        .flat_map(|group| group.items)
        .nth(state.selected)
        .map(|item| item.action)
}
