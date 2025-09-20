# git-insights

A cli tool to generate Git repo stats and insights.

## Features

- [x] Commit count
- [x] Contributor stats
- [ ] Heatmaps/Hotspots
- [ ] Per user stats
- [ ] File-owner estimations
- [ ] Export to CSV/JSON
- [ ] ...

## Installation

### Installing from crates.io

```bash
cargo install git-insights
```

### Installing via cargo + git

```bash
cargo install --git https://github.com/Magnus167/git-insights.git
```

### Building from source:

```bash
git clone https://github.com/Magnus167/git-insights.git
cd git-insights
cargo install --path .
```

## Usage

For now, there are no arguments or options. Just run:

```bash
git-insights
```

Future versions will have more options and features, and a help command.

## License

MIT License. See [`LICENSE`](./LICENSE) file for details.
