use crate::action::Action;
use crate::state::Overlay;

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

#[derive(Debug, Clone, Copy)]
pub enum Section {
    Navigation,
    View,
    General,
}

impl Section {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Navigation => "Navigation",
            Self::View => "View",
            Self::General => "General",
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

pub fn command_groups() -> &'static [CommandGroup] {
    &[
        CommandGroup {
            section: Section::Navigation,
            items: &[
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
            section: Section::View,
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
                    keybind: "l",
                    action: Action::ToggleDiffLineNumbers,
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
