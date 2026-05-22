<div align="center">
  <p>
    <h2>marten</h2>
  </p>
  <p>A small Rust Git TUI for reviewing local changes without leaving the terminal.</p>

![License](https://img.shields.io/badge/license-MIT-blue.svg)
![Rust](https://img.shields.io/badge/rust-1.85%2B-orange.svg)
![Status](https://img.shields.io/badge/status-early%20development-yellow.svg)
</div>

## What it does

marten opens inside a Git repository and gives you a focused view of your working tree:

- changed files grouped by status
- insertion/deletion counts per file
- staged, unstaged, partial, untracked, and conflicted file states
- diff hunks for the selected file
- repository, branch, ahead/behind, and change counts in the top bar

Branch, stash, and history panels are present in the layout, but still early.

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

## Keybindings

| Key | Action |
| --- | --- |
| `tab` / `shift+tab` | Move focus between panels |
| `0` | Focus diff |
| `1` | Focus files |
| `2` | Focus branches |
| `3` | Focus stash |
| `4` | Focus history |
| `j` / `k` | Navigate files or scroll the diff |
| `g` / `G` | Jump to first or last file |
| `[` / `]` | Move between diff hunks |
| `l` | Toggle diff line numbers |
| `r` | Refresh repository state |
| `q` / `ctrl+c` | Quit |

## Development

To see all the available options, run `make help`.

```bash
make build
make check
make test
make lint
make fmt
make ci
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
