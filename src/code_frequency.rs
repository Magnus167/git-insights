use crate::visualize::collect_commit_timestamps;
use std::time::{SystemTime, UNIX_EPOCH};

/// Code-frequency visualizations.
pub enum Group {
    HourOfDay,
    DayOfWeek,
    DayOfMonth,
}

pub enum HeatmapKind {
    DowByHod,
    DomByHod,
}

/// Filter to last N weeks.
fn filter_by_weeks(timestamps: &[u64], weeks: Option<usize>, now: u64) -> Vec<u64> {
    if let Some(w) = weeks {
        if w == 0 {
            return Vec::new();
        }
        const DAY: u64 = 86_400;
        const WEEK: u64 = 7 * DAY;
        let start_of_week = now - (now % WEEK);
        let aligned_end = start_of_week.saturating_add(WEEK - 1);
        let span = (w as u64).saturating_mul(WEEK);
        let min_ts = aligned_end.saturating_sub(span.saturating_sub(1));
        timestamps
            .iter()
            .copied()
            .filter(|&t| t <= aligned_end && t >= min_ts)
            .collect()
    } else {
        timestamps.to_vec()
    }
}

/// Histograms.

pub fn histogram_hour_of_day(timestamps: &[u64]) -> [usize; 24] {
    let mut bins = [0usize; 24];
    for &t in timestamps {
        let hour = ((t / 3_600) % 24) as usize;
        bins[hour] += 1;
    }
    bins
}

pub fn histogram_day_of_week(timestamps: &[u64]) -> [usize; 7] {
    let mut bins = [0usize; 7];
    for &t in timestamps {
        let day = t / 86_400;
        let weekday = ((day + 4) % 7) as usize; // 1970-01-01 = Thu (4)
        bins[weekday] += 1;
    }
    bins
}

/// Day-of-month histogram.
pub fn histogram_day_of_month(timestamps: &[u64]) -> [usize; 31] {
    let mut bins = [0usize; 31];
    for &t in timestamps {
        let (_, _, d) = ymd_from_unix(t);
        if d >= 1 && d <= 31 {
            bins[(d - 1) as usize] += 1;
        }
    }
    bins
}

/// Heatmaps

/// 7x24 heatmap.
pub fn heatmap_dow_by_hod(timestamps: &[u64]) -> [[usize; 24]; 7] {
    let mut grid = [[0usize; 24]; 7];
    for &t in timestamps {
        let day = t / 86_400;
        let weekday = ((day + 4) % 7) as usize;
        let hour = ((t / 3_600) % 24) as usize;
        grid[weekday][hour] += 1;
    }
    grid
}

/// 31x24 heatmap.
pub fn heatmap_dom_by_hod(timestamps: &[u64]) -> [[usize; 24]; 31] {
    let mut grid = [[0usize; 24]; 31];
    for &t in timestamps {
        let (_, _, d) = ymd_from_unix(t);
        if d >= 1 && d <= 31 {
            let row = (d - 1) as usize;
            let hour = ((t / 3_600) % 24) as usize;
            grid[row][hour] += 1;
        }
    }
    grid
}

/// Rendering.

const ANSI_RESET: &str = "\x1b[0m";

/// Map value to intensity index.
fn intensity_index(v: usize, max: usize, levels: usize) -> usize {
    if max == 0 || v == 0 || levels <= 1 {
        return 0;
    }
    let l = levels - 1;
    let idx = ((v - 1) * l) / max + 1;
    idx.min(l)
}

/// 12-step color palette.
fn color_for_level_rich(idx: usize, levels: usize) -> &'static str {
    // 12-color ramp from dim through cool to warm hues.
    // Using standard ANSI codes (widely supported); bright/dim variants to increase steps.
    const PALETTE: [&str; 12] = [
        "\x1b[90m", // 0: dim (should not be used for non-zero, but safe fallback)
        "\x1b[34m", // blue
        "\x1b[94m", // bright blue
        "\x1b[36m", // cyan
        "\x1b[96m", // bright cyan
        "\x1b[32m", // green
        "\x1b[92m", // bright green
        "\x1b[33m", // yellow
        "\x1b[93m", // bright yellow
        "\x1b[35m", // magenta
        "\x1b[95m", // bright magenta
        "\x1b[91m", // bright red
    ];
    let n = PALETTE.len();
    if levels <= 1 {
        return PALETTE[0];
    }
    // Scale idx (0..levels-1) into PALETTE indices (0..n-1)
    let k = if idx >= levels - 1 {
        n - 1
    } else {
        (idx * (n - 1)) / (levels - 1)
    };
    PALETTE[k]
}

/// Legend (rich palette).
fn print_ramp_legend_rich(color: bool, unit: &str) {
    if color {
        print!("\x1b[90mLegend (low→high, blank=0 {}):\x1b[0m ", unit);
        let levels = 10;
        for lvl in 1..levels {
            let code = color_for_level_rich(lvl, levels);
            print!(" {}█{}", code, ANSI_RESET);
        }
        println!();
    } else {
        // ASCII legend consistent with existing ramp
        println!("Legend (low→high, blank=' ' 0 {}):  .:-=+*#%@", unit);
    }
}

fn render_histogram_labeled(labels: &[&str], counts: &[usize], color: bool, unit: &str) {
    let max = counts.iter().copied().max().unwrap_or(0);
    let label_width = labels.iter().map(|s| s.len()).max().unwrap_or(0).max(3);
    if color {
        print!("\x1b[90m");
    }
    println!("Histogram — unit: {}", unit);
    if color {
        print!("\x1b[0m");
    }
    print_ramp_legend_rich(color, unit);

    if max == 0 {
        println!("(no commits)");
        return;
    }

    // Target bar width
    let width = 40usize;
    for (i, &c) in counts.iter().enumerate() {
        let bar_len = (c * width + max - 1) / max; // ceil
        let mut line = String::new();
        line.push_str(&format!("{:>width$} | ", labels[i], width = label_width));
        if color {
            let idx = intensity_index(c, max, 10);
            line.push_str(color_for_level_rich(idx, 10));
            for _ in 0..bar_len {
                line.push('█');
            }
            line.push_str(ANSI_RESET);
            line.push_str(&format!(" {}", c));
        } else {
            for _ in 0..bar_len {
                line.push('#');
            }
            line.push_str(&format!(" {}", c));
        }
        println!("{}", line);
    }
}

/// Build histogram table.
fn build_histogram_table(labels: &[&str], counts: &[usize]) -> String {
    use std::fmt::Write as _;
    let n = counts.len().min(labels.len());
    let max_count = counts.iter().copied().max().unwrap_or(0);

    // Compute data-driven widths
    let label_w_data = labels
        .iter()
        .take(n)
        .map(|s| s.len())
        .max()
        .unwrap_or(0)
        .max(3);
    let count_w_data = max_count.to_string().len().max(1);
    let bar_w_data = 20usize;

    // Ensure headers fit within column widths (avoid lines of different lengths)
    let label_hdr = "Label".len();
    let count_hdr = "Count".len();
    let bar_hdr = "Bar".len();

    let label_w = label_w_data.max(label_hdr);
    let count_w = count_w_data.max(count_hdr);
    let bar_w = bar_w_data.max(bar_hdr);

    let mut out = String::new();

    let push_sep = |s: &mut String| {
        s.push('+');
        for _ in 0..(label_w + 2) {
            s.push('-');
        }
        s.push('+');
        for _ in 0..(count_w + 2) {
            s.push('-');
        }
        s.push('+');
        for _ in 0..(bar_w + 2) {
            s.push('-');
        }
        s.push_str("+\n");
    };

    // Header
    push_sep(&mut out);
    let _ = write!(
        out,
        "| {:>lw$} | {:>cw$} | {:>bw$} |\n",
        "Label",
        "Count",
        "Bar",
        lw = label_w,
        cw = count_w,
        bw = bar_w
    );
    push_sep(&mut out);

    // Rows
    for i in 0..n {
        let lab = labels[i];
        let c = counts[i];
        let filled = if max_count == 0 {
            0
        } else {
            (c * bar_w + max_count - 1) / max_count
        }; // ceil
        let mut bar = String::with_capacity(bar_w);
        for _ in 0..filled {
            bar.push('#');
        }
        for _ in filled..bar_w {
            bar.push(' ');
        }
        let _ = write!(
            out,
            "| {:>lw$} | {:>cw$} | {} |\n",
            lab,
            c,
            bar,
            lw = label_w,
            cw = count_w
        );
    }
    push_sep(&mut out);
    out
}

/// Render histogram table.
fn render_histogram_table(labels: &[&str], counts: &[usize], color: bool) {
    if !color {
        let s = build_histogram_table(labels, counts);
        print!("{}", s);
        return;
    }

    use std::fmt::Write as _;

    let n = counts.len().min(labels.len());
    let max_count = counts.iter().copied().max().unwrap_or(0);

    // Compute data-driven widths (ensuring headers fit)
    let label_w_data = labels
        .iter()
        .take(n)
        .map(|s| s.len())
        .max()
        .unwrap_or(0)
        .max(3);
    let count_w_data = max_count.to_string().len().max(1);
    let bar_w_data = 20usize;
    let label_w = label_w_data.max("Label".len());
    let count_w = count_w_data.max("Count".len());
    let bar_w = bar_w_data.max("Bar".len());

    let push_sep = |s: &mut String| {
        s.push('+');
        for _ in 0..(label_w + 2) {
            s.push('-');
        }
        s.push('+');
        for _ in 0..(count_w + 2) {
            s.push('-');
        }
        s.push('+');
        for _ in 0..(bar_w + 2) {
            s.push('-');
        }
        s.push_str("+\n");
    };

    let mut out = String::new();
    // Top border
    push_sep(&mut out);
    // Header (plain text)
    let _ = write!(
        out,
        "| {:>lw$} | {:>cw$} | {:>bw$} |\n",
        "Label",
        "Count",
        "Bar",
        lw = label_w,
        cw = count_w,
        bw = bar_w
    );
    // Header separator
    push_sep(&mut out);
    print!("{}", out);
    out.clear();

    // Data rows
    for i in 0..n {
        let lab = labels[i];
        let c = counts[i];

        // Bar content using full block '█' for better color appearance
        let filled = if max_count == 0 {
            0
        } else {
            (c * bar_w + max_count - 1) / max_count
        }; // ceil
        let mut bar = String::with_capacity(bar_w);
        for _ in 0..filled {
            bar.push('█');
        }
        for _ in filled..bar_w {
            bar.push(' ');
        }

        // Shade/color mapping; zero uses dim
        let shade = if c == 0 {
            0
        } else {
            intensity_index(c, max_count, 10)
        };
        let code = if shade == 0 {
            "\x1b[90m"
        } else {
            color_for_level_rich(shade, 10)
        };

        // Print row: keep widths applied to digits only, wrap with ANSI to preserve alignment
        print!("| {:>lw$} ", lab, lw = label_w);
        print!("| {}{:>cw$}{} ", code, c, ANSI_RESET, cw = count_w);
        print!("| {}{}{} |\n", code, bar, ANSI_RESET);

        out.clear();
        push_sep(&mut out);
        print!("{}", out);
    }
}

/// Build hour axis (24 columns).
pub(super) fn build_hour_axis_24(indent: usize, cell_w: usize) -> String {
    let mut s = String::with_capacity(indent + 24 * cell_w);
    for _ in 0..indent {
        s.push(' ');
    }
    for h in 0..24 {
        let hh = format!("{:02}", h);
        // Left-align the 2-digit hour within the cell width to align with cell's left edge
        s.push_str(&format!("{:<w$}", hh, w = cell_w));
    }
    s
}

/// Render heatmap grid (rows x 24).
fn render_heatmap_rows_x_24(rows: &[Vec<usize>], row_labels: &[String], color: bool) {
    let cols = 24usize;
    // Compute global max
    let mut max = 0usize;
    for r in rows {
        for &v in r.iter().take(cols) {
            if v > max {
                max = v;
            }
        }
    }
    // Header (centered hours per fixed-width column)
    println!("{}", build_hour_axis_24(4, 3));
    for (ri, lab) in row_labels.iter().enumerate() {
        print!("{:<3} ", &lab);
        for h in 0..cols {
            let v = rows[ri][h];
            if color {
                if max == 0 || v == 0 {
                    // 3 spaces for an empty cell (width 3)
                    print!("   ");
                } else {
                    // Double bricks for clearer alignment: two blocks + one space (width 3)
                    let idx = intensity_index(v, max, 10);
                    let code = color_for_level_rich(idx, 10);
                    print!("{}██{} ", code, ANSI_RESET);
                }
            } else {
                // ASCII: double the ramp char for same width (2 chars + 1 space)
                let ch = if max == 0 {
                    ' '
                } else {
                    let ramp: &[u8] = b" .:-=+*#%@";
                    let idx = (v.saturating_mul(ramp.len() - 1)) / max;
                    ramp[idx] as char
                };
                print!("{}{} ", ch, ch);
            }
        }
        println!();
    }
    println!("{}", build_hour_axis_24(4, 3));
}

/// Build heatmap table.
fn build_heatmap_table_rows_x_24(rows: &[Vec<usize>], row_labels: &[String]) -> String {
    use std::fmt::Write as _;
    // Compute max value to determine width (min width 2)
    let mut max_val = 0usize;
    for r in rows {
        for &v in r.iter().take(24) {
            if v > max_val {
                max_val = v;
            }
        }
    }
    let cell_w = max_val.to_string().len().max(2);
    let rlw = row_labels.iter().map(|s| s.len()).max().unwrap_or(3).max(3);

    let mut out = String::new();

    // Helper to draw a horizontal separator line
    let push_sep = |s: &mut String| {
        s.push('+');
        for _ in 0..(rlw + 2) {
            s.push('-');
        }
        for _ in 0..24 {
            s.push('+');
            for _ in 0..(cell_w + 2) {
                s.push('-');
            }
        }
        s.push_str("+\n");
    };

    // Top border
    push_sep(&mut out);

    // Header row
    let _ = write!(out, "| {:>rlw$} ", "", rlw = rlw);
    for h in 0..24 {
        let _ = write!(out, "| {:>w$} ", format!("{:02}", h), w = cell_w);
    }
    out.push_str("|\n");

    // Header separator
    push_sep(&mut out);

    // Data rows
    for (ri, lab) in row_labels.iter().enumerate() {
        let _ = write!(out, "| {:>rlw$} ", lab, rlw = rlw);
        for h in 0..24 {
            let v = rows[ri][h];
            let _ = write!(out, "| {:>w$} ", v, w = cell_w);
        }
        out.push_str("|\n");
        push_sep(&mut out);
    }

    out
}

/// Render heatmap table.
fn render_heatmap_table_rows_x_24(rows: &[Vec<usize>], row_labels: &[String]) {
    let s = build_heatmap_table_rows_x_24(rows, row_labels);
    print!("{}", s);
}

/// Render colored heatmap table.
fn render_heatmap_table_rows_x_24_colored(
    rows: &[Vec<usize>],
    row_labels: &[String],
    _color: bool,
) {
    use std::fmt::Write as _;

    // Compute max to determine widths and intensities
    let mut max_val = 0usize;
    for r in rows {
        for &v in r.iter().take(24) {
            if v > max_val {
                max_val = v;
            }
        }
    }
    let cell_w = max_val.to_string().len().max(2);
    let rlw = row_labels.iter().map(|s| s.len()).max().unwrap_or(3).max(3);

    let mut out = String::new();

    // Separator
    let push_sep = |s: &mut String| {
        s.push('+');
        for _ in 0..(rlw + 2) {
            s.push('-');
        }
        for _ in 0..24 {
            s.push('+');
            for _ in 0..(cell_w + 2) {
                s.push('-');
            }
        }
        s.push_str("+\n");
    };

    // Top border
    push_sep(&mut out);

    // Header row
    let _ = write!(out, "| {:>rlw$} ", "", rlw = rlw);
    for h in 0..24 {
        let _ = write!(out, "| {:>w$} ", format!("{:02}", h), w = cell_w);
    }
    out.push_str("|\n");

    // Header separator
    push_sep(&mut out);
    print!("{}", out);
    out.clear();

    // Data rows with colored counts
    for (ri, lab) in row_labels.iter().enumerate() {
        // Row label
        print!("| {:>rlw$} ", lab, rlw = rlw);

        for h in 0..24 {
            let v = rows[ri][h];
            let shade = if v == 0 || max_val == 0 {
                0
            } else {
                intensity_index(v, max_val, 10)
            };
            let code = if shade == 0 {
                "\x1b[90m"
            } else {
                color_for_level_rich(shade, 10)
            };
            print!("| {}{:>w$}{} ", code, v, ANSI_RESET, w = cell_w);
        }
        println!("|");

        out.clear();
        push_sep(&mut out);
        print!("{}", out);
    }
}

/// Runner.

pub fn run_code_frequency_with_options(
    group: Option<Group>,
    heatmap: Option<HeatmapKind>,
    weeks: Option<usize>,
    color: bool,
    table: bool,
) -> Result<(), String> {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| format!("clock error: {e}"))?
        .as_secs();
    let ts_all = collect_commit_timestamps()?;
    let ts = filter_by_weeks(&ts_all, weeks, now);

    match heatmap {
        Some(HeatmapKind::DowByHod) => {
            let grid = heatmap_dow_by_hod(&ts);
            let labels = ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"];
            if color && !table {
                print!("\x1b[90m");
            }
            println!("Heatmap: Day-of-Week x Hour-of-Day (UTC), unit: commits/hour");
            if color && !table {
                print!("\x1b[0m");
            }
            if !table {
                print_ramp_legend_rich(color, "commits/hour");
                println!();
            }

            let rows: Vec<Vec<usize>> = (0..7).map(|r| grid[r].to_vec()).collect();
            let row_labels: Vec<String> = labels.iter().map(|s| s.to_string()).collect();
            if table {
                if color {
                    render_heatmap_table_rows_x_24_colored(&rows, &row_labels, true);
                } else {
                    render_heatmap_table_rows_x_24(&rows, &row_labels);
                }
            } else {
                render_heatmap_rows_x_24(&rows, &row_labels, color);
            }
        }
        Some(HeatmapKind::DomByHod) => {
            let grid = heatmap_dom_by_hod(&ts);
            if color && !table {
                print!("\x1b[90m");
            }
            println!("Heatmap: Day-of-Month x Hour-of-Day (UTC), unit: commits/hour");
            if color && !table {
                print!("\x1b[0m");
            }
            if !table {
                print_ramp_legend_rich(color, "commits/hour");
                println!();
            }

            let rows: Vec<Vec<usize>> = (0..31).map(|r| grid[r].to_vec()).collect();
            let row_labels: Vec<String> = (1..=31).map(|d| format!("{:02}", d)).collect();
            if table {
                if color {
                    render_heatmap_table_rows_x_24_colored(&rows, &row_labels, true);
                } else {
                    render_heatmap_table_rows_x_24(&rows, &row_labels);
                }
            } else {
                render_heatmap_rows_x_24(&rows, &row_labels, color);
            }
        }
        None => {
            // Histogram mode
            let grp = group.unwrap_or(Group::HourOfDay);
            match grp {
                Group::HourOfDay => {
                    let bins = histogram_hour_of_day(&ts);
                    let labels: Vec<String> = (0..24).map(|h| format!("{:02}", h)).collect();
                    let lab_refs: Vec<&str> = labels.iter().map(|s| s.as_str()).collect();
                    if table {
                        render_histogram_table(&lab_refs, &bins, color);
                    } else {
                        render_histogram_labeled(&lab_refs, &bins, color, "commits/hour");
                    }
                }
                Group::DayOfWeek => {
                    let bins = histogram_day_of_week(&ts);
                    let labels = ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"];
                    if table {
                        render_histogram_table(&labels, &bins, color);
                    } else {
                        render_histogram_labeled(&labels, &bins, color, "commits/day");
                    }
                }
                Group::DayOfMonth => {
                    let bins = histogram_day_of_month(&ts);
                    let labels: Vec<String> = (1..=31).map(|d| format!("{:02}", d)).collect();
                    let lab_refs: Vec<&str> = labels.iter().map(|s| s.as_str()).collect();
                    if table {
                        render_histogram_table(&lab_refs, &bins, color);
                    } else {
                        render_histogram_labeled(&lab_refs, &bins, color, "commits/day");
                    }
                }
            }
        }
    }
    Ok(())
}

/// Convert Unix seconds to (y,m,d) UTC.
fn ymd_from_unix(t: u64) -> (i32, u32, u32) {
    let days = (t / 86_400) as i64;
    civil_from_days(days)
}

/// Howard Hinnant's algorithm from
/// https://howardhinnant.github.io/date_algorithms.html#civil_from_days
fn civil_from_days(z: i64) -> (i32, u32, u32) {
    let z = z + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097; // [0, 146096]
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365; // [0, 399]
    let y = (yoe as i32) + era as i32 * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100); // [0, 365]
    let mp = (5 * doy + 2) / 153; // [0, 11]
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32; // [1, 31]
    let m = (mp + 2) % 12 + 1; // [1, 12]
    let y = y + ((mp >= 10) as i32); // year increment if Jan/Feb
    (y, m as u32, d)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
    use std::fs;
    use std::io::Write;
    use std::path::PathBuf;
    use std::process::{Command, Stdio};
    use std::sync::MutexGuard;
    use std::time::{SystemTime, UNIX_EPOCH};

    // Temp repo pattern (auto-cleaned; clean git config; no pager)
    struct TempRepo {
        _guard: MutexGuard<'static, ()>,
        old_dir: PathBuf,
        path: PathBuf,
    }

    impl TempRepo {
        fn new(prefix: &str) -> Self {
            let guard = crate::test_sync::test_lock();

            let old_dir = env::current_dir().unwrap();
            let base = env::temp_dir();
            let ts = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos();
            let path = base.join(format!("{}-{}", prefix, ts));
            fs::create_dir_all(&path).unwrap();
            env::set_current_dir(&path).unwrap();

            assert!(Command::new("git")
                .args(["--no-pager", "init", "-q"])
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status()
                .unwrap()
                .success());
            assert!(Command::new("git")
                .args(["config", "commit.gpgsign", "false"])
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status()
                .unwrap()
                .success());
            assert!(Command::new("git")
                .args(["config", "core.hooksPath", "/dev/null"])
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status()
                .unwrap()
                .success());
            assert!(Command::new("git")
                .args(["config", "user.name", "Test"])
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status()
                .unwrap()
                .success());
            assert!(Command::new("git")
                .args(["config", "user.email", "test@example.com"])
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status()
                .unwrap()
                .success());

            // initial commit
            fs::write("INIT", "init\n").unwrap();
            let _ = Command::new("git")
                .args(["--no-pager", "add", "."])
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status();
            let mut c = Command::new("git");
            c.args(["-c", "commit.gpgsign=false"])
                .arg("--no-pager")
                .arg("commit")
                .arg("--no-verify")
                .arg("-q")
                .arg("-m")
                .arg("chore: init");
            c.env("GIT_AUTHOR_NAME", "Init");
            c.env("GIT_AUTHOR_EMAIL", "init@example.com");
            c.env("GIT_COMMITTER_NAME", "Init");
            c.env("GIT_COMMITTER_EMAIL", "init@example.com");
            c.stdout(Stdio::null()).stderr(Stdio::null());
            assert!(c.status().unwrap().success());

            Self {
                _guard: guard,
                old_dir,
                path,
            }
        }

        fn commit_with_epoch(&self, name: &str, email: &str, file: &str, content: &str, ts: u64) {
            // write/append file
            let mut f = fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(file)
                .unwrap();
            f.write_all(content.as_bytes()).unwrap();

            // add and commit with explicit dates
            let add_ok = Command::new("git")
                .args(["add", "."])
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status()
                .map(|s| s.success())
                .unwrap_or(false)
                || Command::new("git")
                    .args(["add", "-A", "."])
                    .stdout(Stdio::null())
                    .stderr(Stdio::null())
                    .status()
                    .map(|s| s.success())
                    .unwrap_or(false);
            assert!(add_ok, "git add failed");

            let mut c = Command::new("git");
            c.args(["-c", "commit.gpgsign=false"])
                .args(["-c", "core.hooksPath=/dev/null"])
                .args(["-c", "user.name=Test"])
                .args(["-c", "user.email=test@example.com"])
                .arg("commit")
                .arg("--no-verify")
                .arg("-q")
                .arg("--allow-empty")
                .arg("-m")
                .arg("test");
            let date = format!("{ts} +0000");
            c.env("GIT_AUTHOR_NAME", name);
            c.env("GIT_AUTHOR_EMAIL", email);
            c.env("GIT_COMMITTER_NAME", name);
            c.env("GIT_COMMITTER_EMAIL", email);
            c.env("GIT_AUTHOR_DATE", &date);
            c.env("GIT_COMMITTER_DATE", &date);
            c.stdout(Stdio::null()).stderr(Stdio::null());
            assert!(c.status().unwrap().success());
        }
    }

    impl Drop for TempRepo {
        fn drop(&mut self) {
            let _ = env::set_current_dir(&self.old_dir);
            let _ = fs::remove_dir_all(&self.path);
        }
    }

    #[test]
    fn test_histogram_hour_of_day_basic() {
        // 00:00, 01:00, 01:59:59, 23:00
        let ts = vec![0, 3600, 7199, 23 * 3600];
        let bins = histogram_hour_of_day(&ts);
        assert_eq!(bins[0], 1);
        assert_eq!(bins[1], 2);
        assert_eq!(bins[23], 1);
    }

    #[test]
    fn test_histogram_day_of_week_basic() {
        // 1970-01-04 00:00:00 UTC is Sunday (index 0)
        let sun = 3 * 86_400;
        // 1970-01-05 00:00:00 UTC is Monday (index 1)
        let mon = 4 * 86_400;
        let bins = histogram_day_of_week(&[sun, mon, mon]);
        assert_eq!(bins[0], 1);
        assert_eq!(bins[1], 2);
    }

    #[test]
    fn test_ymd_from_unix_dom() {
        // 1970-01-31
        let jan31 = 30 * 86_400;
        let (_, m, d) = ymd_from_unix(jan31);
        assert_eq!((m, d), (1, 31));
    }

    #[test]
    fn test_histogram_day_of_month_basic() {
        // 1970-01-01 is day 1, 1970-01-31 is day 31
        let d1 = 0;
        let d31 = 30 * 86_400;
        let bins = histogram_day_of_month(&[d1, d31, d31]);
        assert_eq!(bins[0], 1); // day 1
        assert_eq!(bins[30], 2); // day 31
    }

    #[test]
    #[ignore]
    fn test_end_to_end_from_temp_repo_histogram_hod() {
        let repo = TempRepo::new("git-insights-freq");
        // create commits at 00:00 and 13:00 on consecutive days
        let base_day = 20 * 86_400; // arbitrary epoch day
        repo.commit_with_epoch("Alice", "a@x", "a.txt", "a\n", base_day + 0);
        repo.commit_with_epoch("Bob", "b@y", "b.txt", "b\n", base_day + 13 * 3_600);
        // Should run without error
        run_code_frequency_with_options(Some(Group::HourOfDay), None, None, false, false)
            .expect("ok");
    }

    #[test]
    fn test_heatmap_dow_by_hod_known_points() {
        // Verify exact binning without premature rounding:
        // 1970-01-04 00:00:00 UTC is Sunday 00h (row 0, col 0)
        let sun_00 = 3 * 86_400;
        // 1970-01-04 13:00:00 UTC Sunday 13h (row 0, col 13)
        let sun_13 = sun_00 + 13 * 3_600;
        // 1970-01-05 05:00:00 UTC Monday 05h (row 1, col 5)
        let mon_05 = 4 * 86_400 + 5 * 3_600;
        let grid = heatmap_dow_by_hod(&[sun_00, sun_13, mon_05]);
        assert_eq!(grid[0][0], 1);
        assert_eq!(grid[0][13], 1);
        assert_eq!(grid[1][5], 1);
    }

    #[test]
    fn test_build_hour_axis_24_widths() {
        let s = super::build_hour_axis_24(4, 3);
        // Starts with 4 spaces (row label indent)
        assert!(s.starts_with("    "));
        // Total visible width = indent + 24 columns * 3 chars each
        assert_eq!(s.len(), 4 + 24 * 3);
        // Digits in order 00..23 when spaces are removed
        let digits: String = s.chars().filter(|c| c.is_ascii_digit()).collect();
        let expect: String = (0..24).map(|h| format!("{:02}", h)).collect();
        assert_eq!(digits, expect);
    }

    #[test]
    fn test_heatmap_table_no_panic() {
        // Build a tiny 3-row table and ensure no panic
        let rows = vec![
            {
                let mut r = vec![0usize; 24];
                r[0] = 1;
                r
            },
            vec![0usize; 24],
            {
                let mut r = vec![0usize; 24];
                r[23] = 2;
                r
            },
        ];
        let labels = vec!["A".to_string(), "B".to_string(), "C".to_string()];
        super::render_heatmap_table_rows_x_24(&rows, &labels);
    }

    #[test]
    fn test_build_histogram_table_alignment() {
        let labels: Vec<&str> = vec!["A", "BB", "CCC"];
        let counts = vec![0usize, 3, 10];
        let s = super::build_histogram_table(&labels, &counts);
        let lines: Vec<&str> = s.lines().collect();
        // Collect data lines starting with '|' (including header)
        let pipe_lines: Vec<&str> = lines
            .iter()
            .copied()
            .filter(|l| l.starts_with('|'))
            .collect();
        assert!(!pipe_lines.is_empty());
        let len0 = pipe_lines[0].len();
        let pipes0 = pipe_lines[0].chars().filter(|&c| c == '|').count();
        for l in pipe_lines {
            assert_eq!(l.len(), len0, "all '|' lines must be equal length");
            let pipes = l.chars().filter(|&c| c == '|').count();
            assert_eq!(
                pipes, pipes0,
                "all '|' lines must have equal number of separators"
            );
        }
    }

    #[test]
    fn test_build_heatmap_table_alignment() {
        let mut rows = vec![vec![0usize; 24]; 2];
        rows[0][0] = 1;
        rows[1][23] = 12; // ensure multi-digit width
        let labels = vec!["R1".to_string(), "R2".to_string()];
        let s = super::build_heatmap_table_rows_x_24(&rows, &labels);
        let lines: Vec<&str> = s.lines().collect();
        let pipe_lines: Vec<&str> = lines
            .iter()
            .copied()
            .filter(|l| l.starts_with('|'))
            .collect();
        assert!(!pipe_lines.is_empty());
        let len0 = pipe_lines[0].len();
        let pipes0 = pipe_lines[0].chars().filter(|&c| c == '|').count();
        for l in pipe_lines {
            assert_eq!(l.len(), len0, "all '|' lines must be equal length");
            let pipes = l.chars().filter(|&c| c == '|').count();
            assert_eq!(
                pipes, pipes0,
                "all '|' lines must have equal number of separators"
            );
        }
    }

    #[test]
    #[ignore]
    fn test_end_to_end_histogram_table_from_temp_repo() {
        let _repo = TempRepo::new("git-insights-freq-table-hod");
        // two commits at different hours
        let base_day = 30 * 86_400;
        _repo.commit_with_epoch("A", "a@x", "a.txt", "a\n", base_day + 0);
        _repo.commit_with_epoch("B", "b@y", "b.txt", "b\n", base_day + 13 * 3_600);
        super::run_code_frequency_with_options(Some(Group::HourOfDay), None, None, false, true)
            .expect("ok");
    }

    #[test]
    #[ignore]
    fn test_end_to_end_heatmap_table_from_temp_repo() {
        let _repo = TempRepo::new("git-insights-freq-table-heat");
        let base_day = 40 * 86_400;
        _repo.commit_with_epoch("C", "c@z", "c.txt", "c\n", base_day + 5 * 3_600);
        _repo.commit_with_epoch("D", "d@z", "d.txt", "d\n", base_day + 23 * 3_600);
        super::run_code_frequency_with_options(
            None,
            Some(HeatmapKind::DowByHod),
            None,
            false,
            true,
        )
        .expect("ok");
    }

    #[test]
    fn test_heatmap_shapes() {
        let ts = vec![0, 3600, 86_400, 100_000, 200_000];
        let dow = heatmap_dow_by_hod(&ts);
        assert_eq!(dow.len(), 7);
        assert_eq!(dow[0].len(), 24);
        let dom = heatmap_dom_by_hod(&ts);
        assert_eq!(dom.len(), 31);
        assert_eq!(dom[0].len(), 24);
    }

    #[test]
    fn test_filter_by_weeks_empty_when_zero() {
        let now = 10 * 7 * 86_400;
        let ts = vec![now - 1000];
        let out = filter_by_weeks(&ts, Some(0), now);
        assert!(out.is_empty());
    }
}
