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
  - [x] Commit heatmap
  - [x] Code-frequency histograms (hour-of-day, day-of-week, day-of-month)
  - [x] Code-frequency heatmaps (day-of-week x hour-of-day, day-of-month x hour-of-day)
  - [ ] Hotspot analysis
  - [x] Timeline charts
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

### Installing via pip (Python)

This project provides an optional Python package built with maturin/pyo3. Requirements:
- Rust toolchain (for building the extension)
- Python with pip
- maturin (recommended): `pip install maturin`

Install the Python package:
```bash
pip install .
```

Alternatively for local dev:
```bash
maturin develop --features python
```

### Python usage

- CLI via Python:
```bash
python -m git_insights --help
```

- CLI via console script (installed by pip):
```bash
git-insights --help
```

- Programmatic (advanced):
```python
from git_insights import _git_insights
code = _git_insights.run(["git-insights", "--version"])
```
## Usage

`git-insights` provides several commands to analyze your repository.
To see the available commands and options, run:

```bash
git-insights --help
```

### Code Frequency

Analyze commit activity frequency using histograms and heatmaps derived from commit timestamps (no dependencies).

Style and color
- Same visual style as the existing timeline and heatmap features:
  - ASCII mode uses the same 10-char ramp for cells and `#` bars for multi-line bars.
  - Color mode uses the same 6-level ANSI color ramp and solid `█` blocks.
  - Legends and dim headings are consistent across features.
- Disable colors with `--no-color`. Color is ON by default.

Time basis and windowing
- All groupings are computed in UTC (same as heatmap).
- When `--weeks N` is provided, the time window aligns to the end of the current week (Sun..Sat), mirroring `timeline` and `heatmap`.
- If `--weeks` is not provided, all repository history is considered.

Groupings (histograms)
- `--group hod` (Hour-of-day): 24 bins 00..23. Good for daily rhythm.
- `--group dow` (Day-of-week): 7 bins Sun..Sat. Useful to see weekday patterns.
- `--group dom` (Day-of-month): 31 bins 01..31. Captures monthly cycles (e.g., release cadence).
- Default grouping is `hod` if no `--heatmap` is specified.

Heatmaps
- `--heatmap dow-hod`: Day-of-week (rows) x Hour-of-day (columns), 7x24.
- `--heatmap dom-hod`: Day-of-month (rows 01..31) x Hour-of-day (columns 00..23), 31x24.
- Headers and legends match the standard heatmap output.

Examples
```bash
# Default (histogram, hour-of-day), full history, color ON
git-insights code-frequency

# Histogram by day-of-week (ASCII only)
git-insights code-frequency --group dow --no-color

# Histogram by day-of-month over the last 26 weeks, color ON
git-insights code-frequency --group dom --weeks 26

# Heatmap: Day-of-week x Hour-of-day (7x24), last 12 weeks, ASCII
git-insights code-frequency --heatmap dow-hod --weeks 12 --no-color

# Heatmap: Day-of-month x Hour-of-day (31x24), color ON
git-insights code-frequency --heatmap dom-hod
```

Interpreting output
- Histograms:
  - A dim header line indicates the unit (e.g., commits/hour).
  - A legend shows the low→high ramp. In color mode, filled cells/bars reflect intensity.
  - Bars are scaled to the global max value within the selected window.
- Heatmaps:
  - Column header shows hours 00..23.
  - Row labels are either Sun..Sat or 01..31 depending on the heatmap kind.
  - Cells are scaled relative to the global max across all cells.

## License

MIT License. See [`LICENSE`](./LICENSE) file for details.
