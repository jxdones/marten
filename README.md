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

## Configuration

Marten reads its configuration from `~/.config/marten/config.toml` on macOS and Linux. The file is optional; when it is missing or empty, Marten uses its defaults.

```toml
[ui]
theme = "marten"
show_sidebar = true
```

`theme` supports `marten` (dark) and `ermine` (light). When the setting is omitted, Marten uses the dark theme by default. Choosing a theme from the in-app theme picker updates this setting.

`show_sidebar` controls whether the sidebar is visible at startup. When omitted, Marten shows it automatically when the terminal is wider than 120 columns. The sidebar can still be toggled while Marten is running.

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
