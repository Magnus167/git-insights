# git-insights

A cli tool to generate Git repo stats and insights.

## Features

- [x] Overall repository statistics
  - [x] Total commit count
  - [x] Total file count
  - [x] Total lines of code (LOC)
- [ ] Individual user insights
  - [ ] Get file "ownership" list
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
