# Changelog

All notable changes to Marten will be documented in this file.

## [Unreleased]

### Added

- Filter the theme picker to dark, light, or all themes with `tab`.
- Control whether line numbers are shown at startup with `[diff] show_line_numbers` in the config file. It defaults to `true`; press `L` while Marten is running to toggle it.
- Create `~/.config/marten/config.toml` on first run, with every setting commented out and documented alongside its default value.

### Fixed

- Picking a theme no longer strips comments and formatting from `config.toml`.

## [0.1.14] - 2026-08-23

### Added

- Add Rose Pine, Rose Pine Dawn, Ayu Dark, Ayu Light, Flexoki Dark, Gruvbox Light, Nord, and Tokyo Night Light themes.

### Fixed

- Viewing a staged file's diff no longer errors with `reference 'refs/heads/main' not found` in a repository with no commits yet.

## [0.1.13] - 2026-08-21

### Added

- Show the repository name and current branch (or the revision/range being viewed) in the terminal tab title, with a leading `*` when the working tree has uncommitted changes. The title reverts on exit.

### Changed

- `[ui] nerd_fonts` now defaults to `true`. Set it to `false` if your terminal font isn't a [Nerd Font](https://www.nerdfonts.com/).

### Fixed

- Hiding the sidebar always moves focus to the diff panel, instead of sometimes leaving it stuck on the now-hidden files panel.

## [0.1.12] - 2026-08-20

### Added

- Show the terminal's own background through Marten's panels, sidebar, popups, and file/hunk headers with `[ui] transparent_background = true` in the config file. Diff add/delete backgrounds and selection highlights stay opaque for legibility.
- Cycle through auto, side-by-side, and unified diff layouts with `v` instead of only toggling between the last two. Set the startup layout with `[diff] layout = "auto" | "split" | "unified"` in the config file.
- Show the current diff layout in the top bar, with Nerd Font glyphs available via `[ui] nerd_fonts = true`.

## [0.1.11] - 2026-08-18

### Added

- Configure how many spaces each tab occupies in diff content. It defaults to `4` and must be greater than zero.

### Fixed

- Align related changed lines by content similarity when side-by-side change blocks contain unequal additions and deletions, keeping unrelated lines in gap rows and inline highlights focused on the actual edits.

## [0.1.10] - 2026-08-11

### Added

- Browse and search commit history with `H`, load a selected commit's diff, and return to current working-tree changes with `r`.

### Fixed

- Show the current branch name instead of "unknown" for repositories with no commits yet.

## [0.1.9] - 2026-08-10

### Changed

- Navigate diff lines with a visible selection cursor.
- Improve sidebar size for narrower and wider terminals.
- Give the selected file name its own accent color per theme, starting with a distinct pink for Dracula, instead of reusing the general accent color.
- Dim the "hunk" and "line" labels in the file header to match the counter color.

### Fixed

- Toggle the sidebar on terminals narrower than 120 columns. It now shows down to 80 columns.
- Soften the deleted-line and deleted-word background colors in the GitHub dark theme.

## [0.1.8] - 2026-08-08

### Added

- Ignore whitespace-only changes in diff hunks and statistics with `[diff] ignore_whitespace = true` in the config file.

## [0.1.7] - 2026-08-05

### Added

- Ignore noisy files from a review with `[review] ignore` glob patterns in the config file. Matching files are collapsed and skipped by the diff loaders but remain listed in the sidebar with a `~` marker.

### Fixed

- Don't crash when a working-tree change is an untracked symlink pointing to a directory.

## [0.1.6] - 2026-08-04

### Added

- Mark files as reviewed with `m` to collapse them in the diff view and jump to the next unreviewed file. The state is kept in memory only and doesn't persist across restarts.
- Show a `M/N files reviewed` count in the top bar once at least one file is marked reviewed.

### Changed

- Update modals to have a rounded border. 
- Improve top bar stats colors for when there's no changes in the repo

### Fixed

- Show the staged/unstaged/etc. status label for binary and type-changed files in the diff header, matching regular files.

## [0.1.5] - 2026-07-30

### Added

- Find and jump to changed files with `Ctrl+P`, with case-insensitive path filtering.

### Changed

- Hide the sidebar when the working tree has no changes.
- Show file change types when inspecting revisions and revision ranges instead of staging-state marks:
  - `A` - added
  - `M` - modified
  - `D` - deleted
  - `R` - renamed
  - `T` - file type changed
- Show both the old and new paths for renamed files in diff headers.
- Show the old and new file types for `T` entries (e.g: `file → symlink`).

## [0.1.4] - 2026-07-29

### Added

- Select files and directories in the sidebar with a left mouse click.

### Fixed

- Fix `[` call from the sidebar to go to the previous hunk.
- Make mouse-wheel and trackpad scrolling target the diff under the pointer without changing keyboard focus.
- Preserve queued keyboard and mouse input while coalescing terminal resize events.

## [0.1.3] - 2026-07-28

### Fixed

- Dim the `/` separator in the `[/]` hunk shortcut hint so it doesn't read as a valid keystroke.

## [0.1.2] - 2026-07-28

### Added

- Open the current diff line (or a hunk) in your editor with `e`, using `$VISUAL`/`$EDITOR` (falls back to `vi`).

### Changed

- Clarified `marten diff --help` with supported revision types, examples, and the difference between two-dot and three-dot comparisons.

### Fixed

- Ensured hunk header backgrounds fill the entire width of the diff panel.
- Adjust file header color for ermine theme.

## [0.1.1] - 2026-07-23

### Added

- Revision range diffing with `marten diff FROM..TO` (direct diff) and `marten diff FROM...TO` (merge-base diff).

## [0.1.0] - 2026-07-23

Initial release.

### Added

- Terminal interface for reviewing staged, unstaged, untracked, and partially staged changes.
- Tree-based file sidebar with directory collapsing and file and hunk navigation.
- Continuous diff view with syntax highlighting, inline word-level changes, and side-by-side line pairing.
- Revision inspection with `marten show <revision>` for commits, branches, tags, and other Git revisions.
- Optional TOML configuration stored at `~/.config/marten/config.toml`.

[0.1.0]: https://github.com/jxdones/marten/releases/tag/v0.1.0
