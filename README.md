<div align="center">
  <p>
    <h2>marten</h2>
  </p>
  <p>A terminal diff viewer for reviewing your work before it becomes a commit or pull request.</p>

  <p>
    <img src="https://raw.githubusercontent.com/jxdones/marten/main/assets/marten.gif" alt="Marten demo" width="830"/>
  </p>

![License](https://img.shields.io/badge/license-MIT-blue.svg)
![Rust](https://img.shields.io/badge/rust-1.85%2B-orange.svg)
[![Built With Ratatui](https://ratatui.rs/built-with-ratatui/badge.svg)](https://ratatui.rs/)
[![Release](https://img.shields.io/github/v/release/jxdones/marten?display_name=release&logo=github)](https://github.com/jxdones/marten/releases)
[![CI](https://img.shields.io/github/actions/workflow/status/jxdones/marten/ci.yml)](https://github.com/jxdones/marten/actions/workflows/ci.yml?branch=main)

[Highlights](#highlights) • [Quick Start](#quick-start) • [Install](#install) • [Usage](#usage) • [Configuration](#configuration)
</div>

## Highlights

- Review staged, unstaged, and untracked changes in one place
- Compare commits, branches, tags, and Git revision ranges
- Browse and search commit history without leaving the app
- Switch between unified and side-by-side diffs
- Find changed files and navigate between hunks quickly
- Open a hunk directly in your editor
- Use the mouse to select files and scroll through diffs
- Exclude noisy files from your review with configurable ignore patterns

## Quick Start

Install the latest release on macOS or Linux:

```bash
curl -fsSL https://raw.githubusercontent.com/jxdones/marten/main/install.sh | sh
```

Then run Marten inside a Git repository:

```bash
marten
```

## Install

To install a specific release, pass its version:

```bash
curl -fsSL https://raw.githubusercontent.com/jxdones/marten/main/install.sh | sh -s -- v0.1.9
```

The installer supports Intel and ARM64 systems. Set `BINDIR` to install somewhere other than `/usr/local/bin`:

```bash
curl -fsSL https://raw.githubusercontent.com/jxdones/marten/main/install.sh | BINDIR="$HOME/.local/bin" sh
```

### Using Cargo

Already have Rust 1.85 or newer? Install Marten directly from crates.io:

```bash
cargo install marten-diff --locked
```

The package is named `marten-diff`; once installed, launch it with `marten`. If you need a Rust toolchain, the recommended installer is [rustup](https://rustup.rs/).

### From source

To build from source, install Rust 1.85 or newer and run:

```bash
git clone https://github.com/jxdones/marten.git
cd marten
make install
```

## Usage

Run it from inside a Git repository:

```bash
marten
```

Review the changes introduced by a commit, branch, tag, or other revision:

```bash
marten show HEAD~1
```

`show` accepts a commit, branch, tag, or other Git revision.

Compare two revisions directly:

```bash
marten diff main..feature-branch
```

Compare the second revision with the merge base of both revisions:

```bash
marten diff main...feature-branch
```

## Configuration

Marten reads its configuration from `~/.config/marten/config.toml` on macOS and Linux. The file is optional; when it is missing or empty, Marten uses its defaults.

```toml
[ui]
theme = "marten"
show_sidebar = true
transparent_background = false
nerd_fonts = true

[review]
ignore = ["*.lock", "generated/**", "vendor/"]

[diff]
ignore_whitespace = false
tab_width = 4
layout = "auto"
```

`theme` supports `marten`, `ermine`, `catppuccin`, `dracula`, and much more. When the setting is omitted, Marten uses `marten` by default. Choosing a theme from the in-app theme picker updates this setting.

`show_sidebar` controls whether the sidebar is visible at startup. When omitted, Marten shows it automatically when the terminal is wider than 120 columns. The sidebar can still be toggled while Marten is running.

`transparent_background` lets your terminal's own background (and any background image or blur it applies) show through the base panels, sidebar, popups, and file/hunk headers. Diff add/delete backgrounds and selection highlights stay opaque for legibility. It defaults to `false`.

`nerd_fonts` swaps the diff layout indicator in the top bar for Nerd Font glyphs instead of plain ASCII/Unicode. It defaults to `true`; set it to `false` if your terminal font isn't a [Nerd Font](https://www.nerdfonts.com/).

`ignore` lists glob patterns for noisy files you'd rather not review (lockfiles, generated code, snapshots, vendored sources). When omitted, nothing is ignored.  
Matching files are collapsed to their header and skipped by the diff loaders. They remain listed in the sidebar (marked `~`) because they can still be visible as part of the changes.

- Patterns without `/` match the basename at any depth (`*.lock` matches `backend/Cargo.lock`)
- `**` crosses directory boundaries (`**/generated/**` matches at any depth)
- Trailing `/` matches everything beneath a directory (`vendor/`).

`ignore_whitespace` hides whitespace-only changes from diff hunks and statistics in working-tree, revision, and revision-range views. It defaults to `false`.  
Files containing only whitespace changes remain listed because Git still considers them modified, but their diff is empty while this option is enabled.

`tab_width` defines the width of `\t`. It defaults to `0` in case is omitted.

`layout` sets the diff view on startup: `"auto"` picks unified or side-by-side based on terminal width, `"split"` forces side-by-side, and `"unified"` forces unified. It defaults to `"auto"`. Press `v` while Marten is running to cycle through the three modes.

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
