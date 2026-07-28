# Changelog

All notable changes to Marten will be documented in this file.

## [Unreleased]

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
