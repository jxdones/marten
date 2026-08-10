# Changelog

All notable changes to Marten will be documented in this file.

## [Unreleased]

### Changed

- Navigate diff lines with a visible selection cursor.
- Improve sidebar size for narrower and wider terminals.

### Fixed

- Toggle the sidebar on terminals narrower than 120 columns. It now shows down to 80 columns.

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
