use crate::git::{count_pull_requests, run_command};
use crate::output::print_progress;
use serde::Serialize;
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Instant;

/// Represents the statistics for a single author.
#[derive(Default, Debug, Clone, Serialize)]
pub struct AuthorStats {
    pub loc: usize,
    pub commits: usize,
    pub files: HashSet<String>,
}

#[derive(Default, Debug, Clone, Serialize)]
pub struct UserStats {
    pub tags: HashSet<String>,
    pub pull_requests: usize,
}

// A type alias for our map of statistics for readability.
pub type StatsMap = HashMap<String, AuthorStats>;

/// Gathers historical commit counts for each author from `git log`.
pub fn gather_commit_stats() -> Result<StatsMap, String> {
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
pub fn gather_loc_and_file_stats() -> Result<StatsMap, String> {
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

pub fn gather_user_stats(username: &str) -> Result<UserStats, String> {
    let mut user_stats = UserStats::default();

    let tags_output = run_command(&["tag", "--format=%(refname:short)"])?;
    for tag in tags_output.lines() {
        let log_output = run_command(&["log", tag, "--author", username, "--pretty=format:%an"])?;
        if !log_output.is_empty() {
            user_stats.tags.insert(tag.to_string());
        }
    }

    user_stats.pull_requests = count_pull_requests(username)?;

    Ok(user_stats)
}
