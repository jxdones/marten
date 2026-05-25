<div align="center">
  <p>
    <h2>marten</h2>
  </p>
  <p>A fast terminal workspace for reviewing local Git changes before they become a PR/MR.</p>

![License](https://img.shields.io/badge/license-MIT-blue.svg)
![Rust](https://img.shields.io/badge/rust-1.85%2B-orange.svg)
![Status](https://img.shields.io/badge/status-early%20development-yellow.svg)
</div>

## What is marten?

marten is a Rust Git TUI for reviewing your own work without leaving the terminal.

It opens inside a Git repository and gives you a focused view of your working tree, changed files, and diffs so you can inspect your changes before committing, pushing, or opening a pull/merge request.

marten is currently focused on local working-tree review. Remote PR/MR review, additional Git views, and review-oriented workflows are planned.

## What it does

marten currently supports:

- changed files grouped by status in a collapsible tree
- insertion/deletion counts per file
- staged, unstaged, partial, untracked, and conflicted file states
- diff hunks for the selected file
- hunk navigation and diff scrolling
- optional diff line numbers
- repository, branch, ahead/behind, and change counts in the top bar
- repository refresh without leaving the TUI

## Why?

Code review often starts before a PR or MR exists. marten is meant for that moment when you want to quickly
inspect your own changes, move through files and hunks, and catch obvious mistakes while staying in your terminal.

## Install

```bash
make install
```

Requires Rust 1.85 or newer.

## Run locally

```bash
make run
```

Run `marten` from inside a Git repository.

```bash
marten
```

## Keybindings

| Key | Action |
| --- | --- |
| `tab` / `shift+tab` | Move focus between panels |
| `0` | Focus diff |
| `1` | Focus files |
| `j` / `k` | Navigate files or scroll the diff |
| `g` / `G` | Jump to first or last file |
| `enter` / `space` | Collapse or expand the selected directory |
| `[` / `]` | Move between diff hunks |
| `l` | Toggle diff line numbers |
| `r` | Refresh repository state |
| `q` / `ctrl+c` | Quit |

## Development

To see all the available options, run `make help`.

```bash
make build
make run-release
make check
make test
make lint
make fmt
make ci
make ci-full
```

For UI testing with local untracked files:

```bash
make dev-files
make clean-dev-files
```

## Built with

- [ratatui](https://github.com/ratatui/ratatui) — terminal UI framework
- [git2](https://github.com/rust-lang/git2-rs) — libgit2 bindings for Rust
- [crossterm](https://github.com/crossterm-rs/crossterm) — cross-platform terminal input/output

## License

MIT
