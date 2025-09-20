use clap::Parser;
use git_insights::{
    cli::{Cli, Commands},
    git::{is_git_installed, is_in_git_repo},
    output::{print_table, print_user_stats},
    stats::{gather_commit_stats, gather_loc_and_file_stats, gather_user_stats},
};
use std::collections::HashSet;
use std::fs::File;
use std::io::Write;

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

    let cli = Cli::parse();

    match &cli.command {
        Commands::Stats => {
            run_insights();
        }
        Commands::Json => {
            export_to_json();
        }
        Commands::User { username } => {
            get_user_insights(username);
        }
    }
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

fn export_to_json() {
    let mut commit_stats = gather_commit_stats().expect("Failed to gather commit stats.");
    let loc_and_file_stats = gather_loc_and_file_stats().expect("Failed to gather LOC stats.");

    let mut final_stats = loc_and_file_stats;
    for (author, data) in commit_stats.drain() {
        final_stats.entry(author).or_default().commits = data.commits;
    }

    let json_output = serde_json::to_string_pretty(&final_stats).expect("Failed to serialize stats to JSON.");
    let mut file = File::create("git-insights.json").expect("Failed to create JSON file.");
    file.write_all(json_output.as_bytes()).expect("Failed to write JSON to file.");
    println!("Successfully exported to git-insights.json");
}

fn get_user_insights(username: &str) {
    match gather_user_stats(username) {
        Ok(stats) => {
            print_user_stats(username, &stats);
        }
        Err(e) => {
            eprintln!("Error getting user insights: {}", e);
        }
    }
}
