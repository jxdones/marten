<div align="center">
  <p>
    <h2>marten</h2>
  </p>
  <p>A terminal diff viewer for reviewing your work before it becomes a commit or pull request.</p>

![License](https://img.shields.io/badge/license-MIT-blue.svg)
![Rust](https://img.shields.io/badge/rust-1.85%2B-orange.svg)
[![Built With Ratatui](https://ratatui.rs/built-with-ratatui/badge.svg)](https://ratatui.rs/)
![Status](https://img.shields.io/badge/status-early%20development-yellow.svg)
</div>

## Install

Marten requires Rust 1.85 or newer.

```bash
git clone https://github.com/jxdones/marten.git
cd marten
make install
```

Run it from inside a Git repository:

```bash
marten
```

To inspect the changes introduced by a revision:

```bash
marten show HEAD~1
```

`show` accepts a commit, branch, tag, or other Git revision.

## Keybindings

| Key | Action |
| --- | --- |
| `tab` / `shift+tab` | Move focus between panels |
| `0` / `1` | Focus the diff or files panel |
| `j` / `k` | Navigate files or scroll the diff |
| `n` / `p` | Select the next or previous changed file |
| `g` / `G` | Select the first or last file while the files panel is focused |
| `enter` / `space` | Collapse or expand a directory while the files panel is focused |
| `[` / `]` | Move between diff hunks |
| `l` | Toggle diff line numbers while the diff is focused |
| `s` | Toggle the files sidebar |
| `r` | Reload repository state and diffs |
| `?` | Open the command palette |
| `q` / `ctrl+c` | Quit |

## Development

```bash
make run
make check
make test
make lint
make ci
```

Run `make help` for the complete list of targets.

To create a temporary repository with sample changes for UI work:

```bash
make dev-files
make clean-dev-files
```

## License

[MIT](LICENSE)
