use crate::stats::{AuthorStats, UserStats};
use std::io::{self, Write};
use std::time::Instant;

/// Prints a formatted table of author statistics.
pub fn print_table(
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

/// Prints a file ownership table for a user.
/// Rows: (file, user_loc, file_loc, pct)
pub fn print_user_ownership(rows: &[(String, usize, usize, f32)]) {
    println!(
        "| {:>4} | {:<60} | {:>7} | {:>7} | {:>6} |",
        "No.", "File", "userLOC", "fileLOC", "%own"
    );
    println!("|{:->6}|:{:-<60}|{:->9}|{:->9}|{:->8}|", "", "", "", "", "");
    for (i, (file, u, f, pct)) in rows.iter().enumerate() {
        println!(
            "| {:>4} | {:<60} | {:>7} | {:>7} | {:>5.1} |",
            i + 1,
            truncate(file, 60),
            u,
            f,
            pct
        );
    }
}

// helper to truncate long file paths for table display
fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else if max > 3 {
        let cut = max - 3;
        // Prefer to keep a trailing '-' if we cut right before it
        // e.g., "this-is-long", max=10 -> "this-is-..."
        let mut end = cut;
        if end > 0 && end < s.len() {
            let prev = &s[end.saturating_sub(1)..end];
            let cur = &s[end..end + 1];
            if prev != "-" && cur == "-" {
                end += 1;
            }
        }
        let mut out = s[..end].to_string();
        out.push_str("...");
        out
    } else {
        s[..max].to_string()
    }
}

/// Renders a progress bar to the console.
pub fn print_progress(processed: usize, total: usize, start_time: Instant) {
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
        .map(|i| if i < filled_width { '#' } else { ' ' })
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

/// Prints a formatted table of user statistics.
pub fn print_user_stats(username: &str, stats: &UserStats) {
    println!("\nStatistics for user: {}", username);
    println!("---------------------------------");
    println!("Merged Pull Requests: {}", stats.pull_requests);

    if !stats.tags.is_empty() {
        println!("\nAuthored in the following tags:");
        let mut sorted_tags: Vec<_> = stats.tags.iter().collect();
        sorted_tags.sort();

        let tag_count = sorted_tags.len();
        if tag_count <= 6 {
            for tag in sorted_tags {
                println!("  - {}", tag);
            }
        } else {
            for tag in sorted_tags.iter().take(5) {
                println!("  - {}", tag);
            }
            println!("  ... ({} more tags)", tag_count - 6);
            if let Some(last_tag) = sorted_tags.last() {
                println!("  - {}", last_tag);
            }
        }
    } else {
        println!("\nNo tags found where this user is an author.");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::stats::{AuthorStats, UserStats};
    use std::collections::HashSet;
    use std::time::Instant;

    #[test]
    fn test_print_table() {
        let mut data = Vec::new();
        let mut files = HashSet::new();
        files.insert("file1.rs".to_string());
        data.push((
            "test_author".to_string(),
            AuthorStats {
                loc: 100,
                commits: 10,
                files,
            },
        ));
        // Should not panic
        print_table(data, 100, 10, 1);
    }

    #[test]
    fn test_print_progress() {
        let start_time = Instant::now();
        // Should not panic
        print_progress(50, 100, start_time);
    }

    #[test]
    fn test_print_user_stats() {
        let mut tags = HashSet::new();
        tags.insert("v1.0".to_string());
        tags.insert("v1.1".to_string());
        let stats = UserStats {
            pull_requests: 5,
            tags,
        };
        // Should not panic
        print_user_stats("test_user", &stats);
    }

    #[test]
    fn test_print_user_stats_no_tags() {
        let stats = UserStats {
            pull_requests: 2,
            tags: HashSet::new(),
        };
        // Should not panic
        print_user_stats("test_user_no_tags", &stats);
    }

    #[test]
    fn test_print_user_ownership() {
        let rows = vec![
            ("src/lib.rs".to_string(), 10, 20, 50.0),
            ("README.md".to_string(), 5, 5, 100.0),
        ];
        // Should not panic
        super::print_user_ownership(&rows);
    }

    #[test]
    fn test_truncate() {
        assert_eq!(super::truncate("short", 10), "short");
        assert_eq!(super::truncate("exactlyten", 10), "exactlyten");
        assert_eq!(super::truncate("this-is-long", 10), "this-is-...");
    }
}
