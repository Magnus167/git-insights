use crate::git::run_command;
use std::time::{SystemTime, UNIX_EPOCH};

/// Collect commit timestamps (unix epoch seconds) in reverse chronological order.
/// Uses clean git invocation with --no-pager and no merges.
pub fn collect_commit_timestamps() -> Result<Vec<u64>, String> {
    let out = run_command(&["--no-pager", "log", "--no-merges", "--format=%ct"])?;
    let mut ts: Vec<u64> = Vec::new();
    for line in out.lines() {
        if let Ok(v) = line.trim().parse::<u64>() {
            ts.push(v);
        }
    }
    Ok(ts)
}

/// Bucket commit timestamps into week bins (7-day windows) ending at `now`.
/// Returns `weeks` bins, oldest -> newest (counts.len() == weeks).
pub fn compute_timeline_weeks(timestamps: &[u64], weeks: usize, now: u64) -> Vec<usize> {
    let mut counts = vec![0usize; weeks];
    if weeks == 0 {
        return counts;
    }
    const WEEK: u64 = 7 * 24 * 60 * 60; // 604800

    // Align to the end of the current epoch-week so bins are week-aligned, not relative to "now".
    // Current week is [start_of_week .. start_of_week+WEEK-1]; use that end boundary.
    let start_of_week = now - (now % WEEK);
    let aligned_end = start_of_week.saturating_add(WEEK - 1);

    for &t in timestamps {
        if t > aligned_end {
            continue;
        }
        let diff = aligned_end - t;
        let bin = (diff / WEEK) as usize;
        if bin < weeks {
            // newest bin is at the end
            let idx = weeks - 1 - bin;
            counts[idx] += 1;
        }
    }
    counts
}

/// Compute a 7x24 (weekday x hour) heatmap in UTC (kept for internal/tests).
/// Weekday index: 0=Sun,1=Mon,...,6=Sat
pub fn compute_heatmap_utc(timestamps: &[u64]) -> [[usize; 24]; 7] {
    let mut grid = [[0usize; 24]; 7];
    for &t in timestamps {
        let day = t / 86_400;
        // 1970-01-01 was a Thursday. With 0=Sun..6=Sat, Thursday corresponds to 4.
        let weekday = ((day + 4) % 7) as usize;
        let hour = ((t / 3_600) % 24) as usize;
        grid[weekday][hour] += 1;
    }
    grid
}

/// Compute a GitHub-style calendar heatmap (weekday x week-column).
/// Returns grid[7][weeks] as Vec<Vec<usize>> with rows=Sun..Sat, cols=old->new (weeks).
pub fn compute_calendar_heatmap(timestamps: &[u64], weeks: usize, now: u64) -> Vec<Vec<usize>> {
    let mut grid = vec![vec![0usize; weeks]; 7];
    if weeks == 0 {
        return grid;
    }
    const DAY: u64 = 86_400;
    const WEEK: u64 = DAY * 7;

    // Align to end of current week
    let start_of_week = now - (now % WEEK);
    let aligned_end = start_of_week.saturating_add(WEEK - 1);
    let span = (weeks as u64).saturating_mul(WEEK);
    let min_ts = aligned_end.saturating_sub(span.saturating_sub(1));

    for &t in timestamps {
        if t > aligned_end || t < min_ts {
            continue;
        }
        let day_index = (aligned_end - t) / DAY;      // 0.. spanning days
        let week_off = (day_index / 7) as usize;      // 0 = current week
        if week_off >= weeks {
            continue;
        }
        let col = weeks - 1 - week_off;               // oldest..newest left->right
        let day = t / DAY;
        let weekday = ((day + 4) % 7) as usize;       // 0=Sun..6=Sat
        grid[weekday][col] += 1;
    }
    grid
}

/// Render a compact single-line timeline using an ASCII ramp per bin.
/// Uses a small 10-char ramp to visualize relative intensity within the provided counts.
pub fn render_timeline_bars(counts: &[usize]) {
    let ramp: &[u8] = b" .:-=+*#%@"; // 10 levels
    let max = counts.iter().copied().max().unwrap_or(0);
    if max == 0 {
        println!("(no commits in selected window)");
        return;
    }
    let mut line = String::with_capacity(counts.len());
    for &c in counts {
        let idx = (c.saturating_mul(ramp.len() - 1)) / max;
        line.push(ramp[idx] as char);
    }
    println!("{}", line);
}

/// Render a 7x24 heatmap using an ASCII ramp. 0=Sun ... 6=Sat as rows.
/// Header shows hours 00..23; each cell is a character denoting relative intensity.
pub fn render_heatmap_ascii(grid: [[usize; 24]; 7]) {
    let ramp: &[u8] = b" .:-=+*#%@"; // 10 levels
    // Find global max for scaling
    let mut max = 0usize;
    for r in 0..7 {
        for h in 0..24 {
            if grid[r][h] > max {
                max = grid[r][h];
            }
        }
    }
    println!("    00  01  02  03  04  05  06  07  08  09  10  11  12  13  14  15  16  17  18  19  20  21  22  23");
    let labels = ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"];
    for (r, lbl) in labels.iter().enumerate() {
        print!("{:<3} ", lbl);
        for h in 0..24 {
            let c = grid[r][h];
            let ch = if max == 0 {
                ' '
            } else {
                let idx = (c.saturating_mul(ramp.len() - 1)) / max;
                ramp[idx] as char
            };
            print!(" {} ", ch);
        }
        println!();
    }
    // Bottom hour axis for reference
    println!("    00  01  02  03  04  05  06  07  08  09  10  11  12  13  14  15  16  17  18  19  20  21  22  23");
}

/// Render GitHub-style calendar heatmap (ASCII ramp)
pub fn render_calendar_heatmap_ascii(grid: &[Vec<usize>]) {
    let ramp: &[u8] = b" .:-=+*#%@"; // 10 levels
    // global max
    let mut max = 0usize;
    for r in 0..7 {
        for c in 0..grid[0].len() {
            if grid[r][c] > max {
                max = grid[r][c];
            }
        }
    }
    let labels = ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"];
    for r in 0..7 {
        print!("{:<3} ", labels[r]);
        for c in 0..grid[0].len() {
            let v = grid[r][c];
            let ch = if max == 0 {
                ' '
            } else {
                let idx = (v.saturating_mul(ramp.len() - 1)) / max;
                ramp[idx] as char
            };
            print!(" {} ", ch);
        }
        println!();
    }
    // bottom reference: week columns count
    print!("    ");
    for _ in 0..grid[0].len() {
        print!("^  ");
    }
    println!();
}

pub fn color_for_level(level: usize) -> &'static str {
    // Simple 6-step ANSI 8-color ramp (foreground)
    match level {
        0 => "\x1b[90m", // bright black / low intensity
        1 => "\x1b[94m", // blue
        2 => "\x1b[96m", // cyan
        3 => "\x1b[92m", // green
        4 => "\x1b[93m", // yellow
        _ => "\x1b[91m", // red (highest)
    }
}
const ANSI_RESET: &str = "\x1b[0m";

/// Map a value in [0..=max] into an intensity index 0..=levels-1.
/// 0 stays 0 (blank); any non-zero maps to at least 1 to ensure visible shading.
fn intensity_index(v: usize, max: usize, levels: usize) -> usize {
    if max == 0 || v == 0 || levels <= 1 {
        return 0;
    }
    let l = levels - 1;
    let idx = ((v - 1) * l) / max + 1;
    idx.min(l)
}

/// Rich color palette (12 steps) for smoother gradients.
/// Only call for non-zero intensity (idx >= 1). idx range: 0..=levels-1
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
    let k = if idx >= levels - 1 { n - 1 } else { (idx * (n - 1)) / (levels - 1) };
    PALETTE[k]
}

/// Print a color/ASCII legend using richer palette for color.
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
        let ramp = " .:-=+*#%@";
        println!(
            "Legend (low→high, blank=' ' 0 {}): {}",
            unit, ramp
        );
    }
}

/// Print a color/ASCII legend showing low→high intensity and the meaning of blank.
pub fn print_ramp_legend(color: bool, unit: &str) {
    if color {
        // Levels 1..5 colored blocks; blank = 0
        print!("\x1b[90mLegend (low→high, blank=0 {}):\x1b[0m ", unit);
        for lvl in 1..=5 {
            print!(" {}█{}", color_for_level(lvl), ANSI_RESET);
        }
        println!();
    } else {
        let ramp = " .:-=+*#%@";
        println!(
            "Legend (low→high, blank=' ' 0 {}): {}",
            unit, ramp
        );
    }
}

/// Render timeline as Unicode bars with optional color.
/// Uses unicode ramp " ▁▂▃▄▅▆▇█" (9 levels) + color ramp.
pub fn render_timeline_bars_colored(counts: &[usize], color: bool) {
    if !color {
        render_timeline_bars(counts);
        return;
    }
    let ramp: &[char] = &[' ', '▁', '▂', '▃', '▄', '▅', '▆', '▇', '█']; // 9 levels
    let max = counts.iter().copied().max().unwrap_or(0);
    if max == 0 {
        println!("(no commits in selected window)");
        return;
    }
    let mut out = String::with_capacity(counts.len() * 6);
    for &c in counts {
        let idx = (c.saturating_mul(ramp.len() - 1)) / max; // 0..=8 (shape)
        // richer color levels; any non-zero count gets at least level 1
        let shade = intensity_index(c, max, 10);
        if shade == 0 {
            out.push_str("\x1b[90m");
        } else {
            out.push_str(color_for_level_rich(shade, 10));
        }
        out.push(ramp[idx]);
    }
    out.push_str(ANSI_RESET);
    println!("{}", out);
}

/// Render timeline as multi-row bars with optional color.
/// height must be >= 1. Uses '█' for color mode and '#' for ASCII.
pub fn render_timeline_multiline(counts: &[usize], height: usize, color: bool) {
    let h = height.max(1);
    let max = counts.iter().copied().max().unwrap_or(0);
    if max == 0 || counts.is_empty() {
        println!("(no commits in selected window)");
        return;
    }

    // Y-axis reference: show labels at top (max), middle (~max/2), and bottom (0)
    let top_label = max;
    let mid_label = (max + 1) / 2;
    let bottom_label = 0usize;
    let label_width = top_label.to_string().len().max(3);
    let axis_char = if color { '│' } else { '|' };
    let dim_start = if color { "\x1b[90m" } else { "" };
    let dim_end = if color { "\x1b[0m" } else { "" };

    for row in (1..=h).rev() {
        // Determine label for this row
        let label_val = if row == h {
            Some(top_label)
        } else if row == ((h + 1) / 2) {
            Some(mid_label)
        } else if row == 1 {
            Some(bottom_label)
        } else {
            None
        };

        // Build left y-axis prefix " 123 |"
        let mut line = String::with_capacity(label_width + 2);
        match label_val {
            Some(v) => {
                if color {
                    line.push_str(dim_start);
                }
                line.push_str(&format!("{:>width$} {}", v, axis_char, width = label_width));
                if color {
                    line.push_str(dim_end);
                }
            }
            None => {
                if color {
                    line.push_str(dim_start);
                }
                line.push_str(&format!("{:>width$} {}", "", axis_char, width = label_width));
                if color {
                    line.push_str(dim_end);
                }
            }
        }

        // Build bars for this row
        let mut bars = String::with_capacity(counts.len() * 6);
        for &c in counts {
            let filled = ((c as usize) * h + max - 1) / max; // ceil to 1..=h
            if filled >= row {
                if color {
                    // richer color levels; any non-zero count gets at least level 1
                    let shade = intensity_index(c, max, 10);
                    bars.push_str(color_for_level_rich(shade, 10));
                    bars.push('█');
                } else {
                    bars.push('#');
                }
            } else {
                bars.push(' ');
            }
        }
        if color {
            bars.push_str(ANSI_RESET);
        }

        // Print y-axis + bars
        println!("{}{}", line, bars);
    }
}

///// Build timeline axis lines (ticks and labels) with explicit left padding.
/// Left padding must match the bar chart's y-axis prefix width (label_width + 2).
fn build_timeline_axis_lines(weeks: usize, left_pad: usize, major: char, minor: char) -> (String, String) {
    if weeks == 0 {
        let s = " ".repeat(left_pad);
        return (s.clone(), s);
    }

    // Ticks line
    let mut ticks = vec![' '; weeks];
    for col in 0..weeks {
        // rel=0 at newest (rightmost), rel=weeks-1 at oldest (leftmost)
        let rel = weeks - 1 - col;
        if rel % 12 == 0 {
            ticks[col] = major;
        } else if rel % 4 == 0 {
            ticks[col] = minor;
        }
    }

    // Labels line (major ticks only). Place numbers without overlaps.
    let mut labels = vec![' '; weeks];
    let mut occupied = vec![false; weeks];
    for col in 0..weeks {
        let rel = weeks - 1 - col;
        if rel % 12 == 0 {
            let s = rel.to_string();
            if col + s.len() <= weeks && (col..col + s.len()).all(|i| !occupied[i]) {
                for (i, ch) in s.chars().enumerate() {
                    labels[col + i] = ch;
                    occupied[col + i] = true;
                }
            }
        }
    }

    let mut ticks_line = " ".repeat(left_pad);
    ticks_line.push_str(&ticks.iter().collect::<String>());

    let mut labels_line = " ".repeat(left_pad);
    labels_line.push_str(&labels.iter().collect::<String>());

    (ticks_line, labels_line)
}

/// Render a compact reference axis below the timeline:
/// - Minor ticks every 4 weeks
/// - Major ticks every 12 weeks (labeled with remaining weeks from newest: 48,36,24,12,0)
fn render_timeline_axis(weeks: usize, color: bool, left_pad: usize) {
    if weeks == 0 {
        return;
    }
    let major = if color { '┼' } else { '+' };
    let minor = if color { '│' } else { '|' };
    let (ticks_line, labels_line) = build_timeline_axis_lines(weeks, left_pad, major, minor);

    if color {
        print!("\x1b[90m"); // dim
    }
    println!("{}", ticks_line);
    println!("{}", labels_line);
    if color {
        print!("\x1b[0m");
    }
}

/// Render heatmap with optional color using '█' blocks (space for zero).
pub fn render_heatmap_ascii_colored(grid: [[usize; 24]; 7], color: bool) {
    if !color {
        render_heatmap_ascii(grid);
        return;
    }
    // global max for scaling
    let mut max = 0usize;
    for r in 0..7 {
        for h in 0..24 {
            if grid[r][h] > max {
                max = grid[r][h];
            }
        }
    }
    println!("    00  01  02  03  04  05  06  07  08  09  10  11  12  13  14  15  16  17  18  19  20  21  22  23");
    let labels = ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"];
    for (r, lbl) in labels.iter().enumerate() {
        print!("{:<3} ", lbl);
        for h in 0..24 {
            let c = grid[r][h];
            if max == 0 || c == 0 {
                print!("   ");
            } else {
                // richer buckets for color with guaranteed non-zero shade
                let idx = intensity_index(c, max, 10);
                let code = color_for_level_rich(idx, 10);
                print!(" {}█{} ", code, ANSI_RESET);
            }
        }
        println!();
    }
    // Bottom hour axis for reference
    println!("    00  01  02  03  04  05  06  07  08  09  10  11  12  13  14  15  16  17  18  19  20  21  22  23");
}

/// Render GitHub-style calendar heatmap (colored)
pub fn render_calendar_heatmap_colored(grid: &[Vec<usize>]) {
    // global max
    let mut max = 0usize;
    for r in 0..7 {
        for c in 0..grid[0].len() {
            if grid[r][c] > max {
                max = grid[r][c];
            }
        }
    }
    let labels = ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"];
    for r in 0..7 {
        print!("{:<3} ", labels[r]);
        for c in 0..grid[0].len() {
            let v = grid[r][c];
            if max == 0 || v == 0 {
                print!("   ");
            } else {
                let idx = intensity_index(v, max, 10);
                let code = color_for_level_rich(idx, 10);
                print!(" {}█{} ", code, ANSI_RESET);
            }
        }
        println!();
    }
    // bottom week columns
    print!("    ");
    for _ in 0..grid[0].len() {
        print!("^  ");
    }
    println!();
}

/// Run the timeline visualization with options.
pub fn run_timeline_with_options(weeks: usize, color: bool) -> Result<(), String> {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| format!("clock error: {e}"))?
        .as_secs();
    let ts = collect_commit_timestamps()?;
    let counts = compute_timeline_weeks(&ts, weeks, now);
    println!("Weekly commits (old -> new), weeks={weeks}:");
    // Print Y-axis unit/scale reference
    let max = counts.iter().copied().max().unwrap_or(0);
    let mid = (max + 1) / 2;
    if color { print!("\x1b[90m"); }
    println!("Y-axis: commits/week (max={}, mid≈{})", max, mid);
    if color { print!("\x1b[0m"); }
    print_ramp_legend_rich(color, "commits/week");
    println!();
    // Default to a 7-line tall chart for better readability without flooding the screen
    render_timeline_multiline(&counts, 7, color);
    // Add axis reference (minor tick=4 weeks, major tick=12 weeks), aligned to the bars' left prefix
    let label_width = max.to_string().len().max(3);
    let left_pad = label_width + 2; // "{label:>width$} {axis}"
    render_timeline_axis(weeks, color, left_pad);
    Ok(())
}

/// Run the timeline visualization end-to-end with default `weeks` if needed.
pub fn run_timeline(weeks: usize) -> Result<(), String> {
    run_timeline_with_options(weeks, false)
}


/// Run the heatmap visualization with options.
pub fn run_heatmap_with_options(weeks: Option<usize>, color: bool) -> Result<(), String> {
    let ts_all = collect_commit_timestamps()?;
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| format!("clock error: {e}"))?
        .as_secs();

    // Default to 52 weeks if not specified to keep a reasonable width like GitHub
    let w = weeks.unwrap_or(52);
    let grid = compute_calendar_heatmap(&ts_all, w, now);

    // Unit and window line
    let mut max = 0usize;
    for r in 0..7 {
        for c in 0..grid[0].len() {
            if grid[r][c] > max {
                max = grid[r][c];
            }
        }
    }
    if color { print!("\x1b[90m"); }
    println!("Calendar heatmap (UTC) — rows: Sun..Sat, cols: weeks (old→new), unit: commits/day, window: last {} weeks, max={}", w, max);
    if color { print!("\x1b[0m"); }
    print_ramp_legend_rich(color, "commits/day");
    println!();

    if color {
        render_calendar_heatmap_colored(&grid);
    } else {
        render_calendar_heatmap_ascii(&grid);
    }
    Ok(())
}

/// Run the heatmap visualization end-to-end.
pub fn run_heatmap() -> Result<(), String> {
    run_heatmap_with_options(None, false)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
    use std::fs;
    use std::io::Write;
    use std::path::PathBuf;
    use std::process::{Command, Stdio};
    use std::time::{SystemTime, UNIX_EPOCH};
    use std::sync::{Mutex, OnceLock, MutexGuard};

    static TEST_DIR_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

    // Simple temp repo that lives under OS temp dir and is cleaned up on Drop.
    struct TempRepo {
        _guard: MutexGuard<'static, ()>,
        old_dir: PathBuf,
        path: PathBuf,
    }

    impl TempRepo {
        fn new(prefix: &str) -> Self {
            // Serialize temp repo creation and chdir to avoid races across parallel tests
            let guard = TEST_DIR_LOCK
                .get_or_init(|| Mutex::new(()))
                .lock()
                .unwrap_or_else(|e| e.into_inner());

            let old_dir = env::current_dir().unwrap();
            let base = env::temp_dir();
            let ts = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos();
            let path = base.join(format!("{}-{}", prefix, ts));
            fs::create_dir_all(&path).unwrap();
            env::set_current_dir(&path).unwrap();

            assert!(
                Command::new("git")
                    .args(["--no-pager", "init", "-q"])
                    .stdout(Stdio::null())
                    .stderr(Stdio::null())
                    .status()
                    .unwrap()
                    .success()
            );
            // Keep commands clean
            assert!(
                Command::new("git")
                    .args(["config", "commit.gpgsign", "false"])
                    .stdout(Stdio::null())
                    .stderr(Stdio::null())
                    .status()
                    .unwrap()
                    .success()
            );
            assert!(
                Command::new("git")
                    .args(["config", "core.hooksPath", "/dev/null"])
                    .stdout(Stdio::null())
                    .stderr(Stdio::null())
                    .status()
                    .unwrap()
                    .success()
            );
            assert!(
                Command::new("git")
                    .args(["config", "user.name", "Test"])
                    .stdout(Stdio::null())
                    .stderr(Stdio::null())
                    .status()
                    .unwrap()
                    .success()
            );
            assert!(
                Command::new("git")
                    .args(["config", "user.email", "test@example.com"])
                    .stdout(Stdio::null())
                    .stderr(Stdio::null())
                    .status()
                    .unwrap()
                    .success()
            );

            // Initial file and commit (for a valid repo)
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

            Self { _guard: guard, old_dir, path }
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
            assert!(add_ok, "git add failed in TempRepo::commit_with_epoch");

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
    fn test_compute_timeline_weeks_simple_bins() {
        // Choose a fixed "now" and create timestamps in two recent weeks.
        let week = 604_800u64;
        let now = 10 * week; // arbitrary multiple
        let ts = vec![
            now - (0 * week) + 1, // this week
            now - (1 * week) + 2, // last week
            now - (1 * week) + 3, // last week
            now - (3 * week),     // 3 weeks ago
        ];
        let counts = compute_timeline_weeks(&ts, 4, now);
        // oldest -> newest bins: weeks=4 => [3w,2w,1w,0w]
        // 3w: 1, 2w:0, 1w:2, 0w:1
        assert_eq!(counts, vec![1, 0, 2, 1]);
    }

    #[test]
    fn test_compute_heatmap_utc_known_points() {
        // 1970-01-04 00:00:00 UTC is a Sunday 00h -> index 0, hour 0
        let sun_00 = 3 * 86_400;
        // 1970-01-04 13:00:00 UTC Sunday 13h
        let sun_13 = sun_00 + 13 * 3_600;
        // 1970-01-05 05:00:00 UTC Monday 05h -> day=4 -> ((4+4)%7)=1 (Mon)
        let mon_05 = 4 * 86_400 + 5 * 3_600;
        let grid = compute_heatmap_utc(&[sun_00, sun_13, mon_05]);
        assert_eq!(grid[0][0], 1);  // Sun 00
        assert_eq!(grid[0][13], 1); // Sun 13
        assert_eq!(grid[1][5], 1);  // Mon 05
    }

    #[test]
    fn test_render_timeline_no_panic() {
        render_timeline_bars(&[0, 1, 2, 3, 0, 5, 5, 1]);
        render_timeline_bars(&[]);
        render_timeline_bars(&[0, 0, 0]);
    }

    #[test]
    fn test_render_heatmap_no_panic() {
        let mut grid = [[0usize; 24]; 7];
        grid[0][0] = 1;
        grid[6][23] = 5;
        render_heatmap_ascii(grid);
    }

    #[test]
    #[ignore]
    fn test_collect_commit_timestamps_from_temp_repo() {
        // Create one temp repo and keep it the current working directory
        // while collecting timestamps.
        let repo = TempRepo::new("git-insights-vis");
        // two commits with known epochs
        let t1 = 1_696_118_400u64; // 2023-10-01 00:00:00 UTC
        let t2 = 1_696_204_800u64; // 2023-10-02 00:00:00 UTC

        // Make commits in this repo
        repo.commit_with_epoch("Alice", "alice@example.com", "a.txt", "a\n", t1);
        repo.commit_with_epoch("Bob", "bob@example.com", "a.txt", "b\n", t2);

        // Validate via our collector (runs in CWD = repo)
        let ts = collect_commit_timestamps().expect("collect timestamps");
        assert!(ts.iter().any(|&x| x == t1), "missing t1");
        assert!(ts.iter().any(|&x| x == t2), "missing t2");
    }

    #[test]
    #[ignore]
    fn test_run_timeline_and_heatmap_end_to_end() {
        // Create a repo and ensure both runners do not error.
        let repo = TempRepo::new("git-insights-vis-run");
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let t_now = now - (now % 86_400); // align to midnight
        repo.commit_with_epoch("X", "x@example.com", "x.txt", "x\n", t_now);
        repo.commit_with_epoch("Y", "y@example.com", "x.txt", "y\n", t_now + 3_600);

        // These call git under the hood; should be fine
        run_timeline(4).expect("timeline ok");
        run_heatmap().expect("heatmap ok");
    }

    #[test]
    fn test_compute_calendar_heatmap_bins() {
        // Use a synthetic "now" for stable alignment
        const DAY: u64 = 86_400;
        const WEEK: u64 = 7 * DAY;
        let now = 10 * WEEK;

        // aligned_end computed same as production logic
        let start_of_week = now - (now % WEEK);
        let aligned_end = start_of_week + WEEK - 1;

        // Place 2 commits in current week, 1 commit in previous week
        let t_curr1 = aligned_end - (1 * DAY); // within current week
        let t_curr2 = aligned_end - (2 * DAY);
        let t_prev1 = aligned_end - (8 * DAY); // previous week
        let ts = vec![t_curr1, t_curr2, t_prev1];

        let grid = super::compute_calendar_heatmap(&ts, 2, now);
        assert_eq!(grid.len(), 7);
        assert_eq!(grid[0].len(), 2);

        // Sum per column: col 0 = older week, col 1 = current week
        let mut col0 = 0usize;
        let mut col1 = 0usize;
        for r in 0..7 {
            col0 += grid[r][0];
            col1 += grid[r][1];
        }
        assert_eq!(col0, 1, "older week should have 1 commit");
        assert_eq!(col1, 2, "current week should have 2 commits");
    }

    #[test]
    fn test_render_calendar_heatmap_no_panic() {
        // Build a small 7 x 4 grid with increasing intensity
        let mut grid = vec![vec![0usize; 4]; 7];
        grid[0][0] = 1;
        grid[1][1] = 2;
        grid[2][2] = 3;
        grid[3][3] = 4;
        // Should not panic in ASCII
        super::render_calendar_heatmap_ascii(&grid);
        // Should not panic in "colored" version (uses ANSI)
        super::render_calendar_heatmap_colored(&grid);
    }

    #[test]
    fn test_print_legends_no_panic() {
        super::print_ramp_legend(false, "commits/week");
        super::print_ramp_legend(true, "commits/day");
    }

    #[test]
    fn test_build_timeline_axis_lines_alignment() {
        // Weeks = 24 -> major ticks at rel=12 (col=11) and rel=0 (col=23)
        let weeks = 24usize;
        let left_pad = 5usize;
        let (ticks, labels) = super::build_timeline_axis_lines(weeks, left_pad, '+', '|');

        // Left padding present and lengths correct
        assert!(ticks.starts_with(&" ".repeat(left_pad)));
        assert!(labels.starts_with(&" ".repeat(left_pad)));
        assert_eq!(ticks.len(), left_pad + weeks);
        assert_eq!(labels.len(), left_pad + weeks);

        let t_body = &ticks[left_pad..];
        let l_body = &labels[left_pad..];

        // Verify tick characters at expected positions
        for (col, tc) in t_body.chars().enumerate() {
            let rel = weeks - 1 - col;
            let expected = if rel % 12 == 0 {
                '+'
            } else if rel % 4 == 0 {
                '|'
            } else {
                ' '
            };
            assert_eq!(tc, expected, "tick mismatch at col {}", col);
        }

        // Verify labels placed at major ticks without overlap
        // For 24 weeks, we expect "12" at col 11 and "0" at col 23
        assert_eq!(&l_body[11..13], "12");
        assert_eq!(&l_body[23..24], "0");
        for col in 0..weeks {
            if !(11..13).contains(&col) && col != 23 {
                assert_eq!(l_body.chars().nth(col).unwrap(), ' ');
            }
        }
    }

    #[test]
    fn test_timeline_axis_alignment_with_temp_repo() {
        // Integration-style check using a real temp git repo and our git collector.
        let _repo = TempRepo::new("git-insights-vis-axis");
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        // Create two commits: one in current week and one in previous week (UTC).
        const DAY: u64 = 86_400;
        let today_midnight = now - (now % DAY);
        let prev_week_same_day = today_midnight.saturating_sub(8 * DAY);

        _repo.commit_with_epoch("Alice", "alice@example.com", "axis.txt", "a\n", today_midnight);
        _repo.commit_with_epoch("Bob", "bob@example.com", "axis.txt", "b\n", prev_week_same_day);

        // Collect via our git path and compute bins.
        let ts = collect_commit_timestamps().expect("collect timestamps");
        let weeks = 8usize;
        let counts = compute_timeline_weeks(&ts, weeks, now);

        // Determine left padding as used by the chart and build axis lines.
        let max = counts.iter().copied().max().unwrap_or(0);
        let label_width = max.to_string().len().max(3);
        let left_pad = label_width + 2;

        let (ticks, labels) = super::build_timeline_axis_lines(weeks, left_pad, '+', '|');

        // Basic alignment checks: the axis lines must include the same left padding as the bars.
        assert!(ticks.starts_with(&" ".repeat(left_pad)));
        assert!(labels.starts_with(&" ".repeat(left_pad)));
        assert_eq!(ticks.len(), left_pad + weeks);
        assert_eq!(labels.len(), left_pad + weeks);
    }
}
