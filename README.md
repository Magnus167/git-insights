# git-insights

[![Rust Tests on Main](https://github.com/Magnus167/git-insights/actions/workflows/rust-tests-main.yml/badge.svg?branch=main)](https://github.com/Magnus167/git-insights/actions/workflows/rust-tests-main.yml)
[![Crates.io](https://img.shields.io/crates/v/git-insights.svg)](https://crates.io/crates/git-insights)
[![Docs](https://docs.rs/git-insights/badge.svg)](https://docs.rs/git-insights)
[![Deps](https://deps.rs/repo/github/Magnus167/git-insights/status.svg)](https://deps.rs/repo/github/Magnus167/git-insights)

A cli tool to generate Git repo stats and insights.

## Features

- [x] Overall repository statistics
  - [x] Total commit count
  - [x] Total file count
  - [x] Total lines of code (LOC)
  - [x] Per-author breakdown (LOC/commits/files) with % distribution
- [ ] Individual user insights
  - [x] Get file "ownership" list
  - [x] Ownership table flags: `--top N`, `--sort loc|pct` and `--by-email` (default matches by name)
  - [ ] Total locs, inserts, updates, deletes
  - [ ] Past PRs/issues count
  - [x] Tags/releases count
- [ ] Data export
  - [x] Export to JSON
  - [ ] Export to CSV
- [ ] Visualizations
  - [ ] Commit heatmap
  - [ ] Hotspot analysis
  - [ ] Timeline charts
- [x] CLI/UX
  - [x] Fast, no-deps
  - [x] Helpful global and per-command help
  - [x] Version command
  - [x] Clean progress spinner while processing files
  - [x] Group by author name by default, or use --by-email for `"Name <email>"`
  - [x] Clean git calls (no pager)

## Installation

### Installing from crates.io

```bash
cargo install git-insights
```

### Installing via cargo + git

```bash
cargo install --git https://github.com/Magnus167/git-insights.git
```

### Building from source

```bash
git clone https://github.com/Magnus167/git-insights.git
cd git-insights
cargo install --path .
```

## Usage

`git-insights` provides several commands to analyze your repository.
To see the available commands and options, run:

```bash
git-insights --help
```

## License

MIT License. See [`LICENSE`](./LICENSE) file for details.
