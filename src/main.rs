use std::collections::{HashMap, HashSet};
use std::io::{self, Write};
use std::process::{Command, Stdio};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Instant;

/// Represents the statistics for a single author.
#[derive(Default, Debug, Clone)]
struct AuthorStats {
    loc: usize,
    commits: usize,
    files: HashSet<String>,
}

// A type alias for our map of statistics for readability.
type StatsMap = HashMap<String, AuthorStats>;

/// The main entry point of the application.
fn main() {
    if !is_git_installed() {
        eprintln!(
            "Error: 'git' command not found. Please ensure Git is installed and in your PATH."
        );
        std::process::exit(1);
    }
    if !is_in_git_repo() {
        eprintln!("Error: Not a git repository.");
        std::process::exit(1);
    }
    run_insights();
}

/// Executes a Git command and returns its stdout if successful.
fn run_command(args: &[&str]) -> Result<String, String> {
    let output = Command::new("git").args(args).output();
    match output {
        Ok(output) => {
            if output.status.success() {
                Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
            } else {
                Err(String::from_utf8_lossy(&output.stderr).to_string())
            }
        }
        Err(e) => Err(format!("Failed to execute git command: {}", e)),
    }
}

/// Gathers historical commit counts for each author from `git log`.
fn gather_commit_stats() -> Result<StatsMap, String> {
    let mut stats: StatsMap = HashMap::new();
    let log_output = run_command(&["log", "--no-merges", "--pretty=format:--%aN--"])?;

    for author in log_output.split("--").filter(|s| !s.is_empty()) {
        // check if is empty, if not then increment commits
        let trimmed_author = author.trim().to_string();

        if !trimmed_author.is_empty() {
            stats.entry(trimmed_author).or_default().commits += 1;
        }
    }
    Ok(stats)
}

/// Gathers LOC and file stats by running `git blame` in parallel.
fn gather_loc_and_file_stats() -> Result<StatsMap, String> {
    let files_to_blame: Vec<String> = run_command(&["ls-files"])?
        .lines()
        .map(String::from)
        .collect();

    let stats = Arc::new(Mutex::new(StatsMap::new()));
    let total_files = files_to_blame.len();
    let processed_files = Arc::new(Mutex::new(0));
    let start_time = Instant::now();

    thread::scope(|s| {
        for file in files_to_blame {
            let stats_clone = Arc::clone(&stats);
            let processed_clone = Arc::clone(&processed_files);

            s.spawn(move || {
                if let Ok(blame_output) =
                    run_command(&["blame", "-w", "-C", "-C", "--line-porcelain", &file])
                {
                    let mut current_author = String::new();
                    let mut author_loc_for_file = HashMap::new();

                    for line in blame_output.lines() {
                        if line.starts_with("author ") {
                            // crop "author " from blame
                            current_author = line[7..].trim().to_string();
                        } else if line.starts_with('\t') {
                            if !current_author.is_empty() {
                                *author_loc_for_file
                                    .entry(current_author.clone())
                                    .or_insert(0) += 1;
                            }
                        }
                    }

                    let mut stats_guard = stats_clone.lock().unwrap();
                    for (author, loc) in author_loc_for_file {
                        if !author.is_empty() {
                            let author_stats = stats_guard.entry(author).or_default();
                            author_stats.loc += loc;
                            author_stats.files.insert(file.clone());
                        }
                    }
                }

                let mut processed_count = processed_clone.lock().unwrap();
                *processed_count += 1;
                print_progress(*processed_count, total_files, start_time);
            });
        }
    });

    println!(); // Newline after progress bar finishes.
    let final_stats = Arc::try_unwrap(stats).unwrap().into_inner().unwrap();
    Ok(final_stats)
}

/// Main logic to orchestrate the gathering and presentation of statistics.
fn run_insights() {
    // get commit stats
    let mut commit_stats = gather_commit_stats().expect("Failed to gather commit stats.");

    // get loc and file stats
    let loc_and_file_stats = gather_loc_and_file_stats().expect("Failed to gather LOC stats.");

    // merge the two stats maps
    let mut final_stats = loc_and_file_stats;
    for (author, data) in commit_stats.drain() {
        final_stats.entry(author).or_default().commits = data.commits;
    }

    // create final totals from the merged data
    let total_loc: usize = final_stats.values().map(|s| s.loc).sum();
    let total_commits: usize = final_stats.values().map(|s| s.commits).sum();

    let mut all_files = HashSet::new();
    for stats in final_stats.values() {
        all_files.extend(stats.files.iter().cloned());
    }
    let total_files = all_files.len();

    println!("Total commits: {}", total_commits);
    println!("Total files: {}", total_files);
    println!("Total loc: {}", total_loc);

    // sort authors by loc in descending order
    let mut sorted_stats: Vec<_> = final_stats.into_iter().collect();
    sorted_stats.sort_by(|a, b| b.1.loc.cmp(&a.1.loc));

    print_table(sorted_stats, total_loc, total_commits, total_files);
}

/// Prints a formatted table of author statistics.
fn print_table(
    data: Vec<(String, AuthorStats)>,
    total_loc: usize,
    total_commits: usize,
    total_files: usize,
) {
    println!(
        "| {:<28} | {:>7} | {:>7} | {:>7} | {:<15} |",
        "Author", "loc", "coms", "fils", "distribution"
    );
    println!(
        "|:{:-<28}|{:->8}|{:->8}|{:->8}|:{:-<16}|",
        "", "", "", "", ""
    );

    for (author, stats) in &data {
        let loc_dist = if total_loc > 0 {
            (stats.loc as f32 / total_loc as f32) * 100.0
        } else {
            0.0
        };
        let coms_dist = if total_commits > 0 {
            (stats.commits as f32 / total_commits as f32) * 100.0
        } else {
            0.0
        };
        let fils_dist = if total_files > 0 {
            (stats.files.len() as f32 / total_files as f32) * 100.0
        } else {
            0.0
        };

        let distribution_str = format!("{:.1}/{:.1}/{:.1}", loc_dist, coms_dist, fils_dist);

        println!(
            "| {:<28} | {:>7} | {:>7} | {:>7} | {:<15} |",
            author,
            stats.loc,
            stats.commits,
            stats.files.len(),
            distribution_str
        );
    }
}

/// Renders a progress bar to the console.
fn print_progress(processed: usize, total: usize, start_time: Instant) {
    const BAR_WIDTH: usize = 50;
    let percentage = processed as f32 / total as f32;
    let filled_width = (percentage * BAR_WIDTH as f32) as usize;
    let elapsed = start_time.elapsed().as_secs_f32();
    let files_per_second = if elapsed > 0.0 {
        processed as f32 / elapsed
    } else {
        0.0
    };
    let bar: String = (0..BAR_WIDTH)
        .map(|i| if i < filled_width { '█' } else { ' ' })
        .collect();
    print!(
        "\rProcessing: {:3.0}%|{}| {}/{} [{:.2} file/s]",
        percentage * 100.0,
        bar,
        processed,
        total,
        files_per_second
    );
    io::stdout().flush().unwrap();
}

/// Checks if the `git` command is available in the system's PATH.
fn is_git_installed() -> bool {
    Command::new("git")
        .arg("--version")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map_or(false, |s| s.success())
}

/// Checks if the current directory is within a Git repository.
fn is_in_git_repo() -> bool {
    Command::new("git")
        .arg("rev-parse")
        .arg("--is-inside-work-tree")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map_or(false, |s| s.success())
}
