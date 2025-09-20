use crate::stats::{AuthorStats, UserStats};
use std::io::{self, Write};
use std::time::Instant;
use version_compare::{Cmp, Version};
use std::cmp::Ordering;

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

/// Prints a formatted table of user statistics.
pub fn print_user_stats(username: &str, stats: &UserStats) {
    println!("\nStatistics for user: {}", username);
    println!("---------------------------------");
    println!("Merged Pull Requests: {}", stats.pull_requests);

    if !stats.tags.is_empty() {
        println!("\nAuthored in the following tags:");
        let mut sorted_tags: Vec<_> = stats.tags.iter().collect();
        sorted_tags.sort_by(|a, b| {
            let a_ver = Version::from(a).unwrap_or(Version::from("0.0.0").unwrap());
            let b_ver = Version::from(b).unwrap_or(Version::from("0.0.0").unwrap());
            match a_ver.compare(&b_ver) {
                Cmp::Lt => Ordering::Less,
                Cmp::Eq => Ordering::Equal,
                Cmp::Gt => Ordering::Greater,
                _ => Ordering::Equal,
            }
        });

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
