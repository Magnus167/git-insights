use git_insights::{
    cli::{Cli, Commands, HelpTopic, render_help, version_string},
    git::{is_git_installed, is_in_git_repo},
    output::print_user_stats,
    stats::{gather_commit_stats, gather_loc_and_file_stats, gather_user_stats, run_stats},
};
use std::fs::File;
use std::io::Write;

fn main() {
    // Parse CLI first so help/version work anywhere (even outside a git repo)
    let cli = match Cli::parse() {
        Ok(cli) => cli,
        Err(e) => {
            eprintln!("{}", e);
            std::process::exit(1);
        }
    };

    // Handle help/version early and exit 0
    match &cli.command {
        Commands::Help { topic } => {
            println!(
                "{}",
                render_help(match topic {
                    HelpTopic::Top => HelpTopic::Top,
                    HelpTopic::Stats => HelpTopic::Stats,
                    HelpTopic::Json => HelpTopic::Json,
                    HelpTopic::User => HelpTopic::User,
                })
            );
            return;
        }
        Commands::Version => {
            println!("{}", version_string());
            return;
        }
        _ => {}
    }

    // For all other commands we require git and a repo
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

    match &cli.command {
        Commands::Stats { by_name } => {
            if let Err(e) = run_stats(*by_name) {
                eprintln!("Error: {}", e);
                std::process::exit(1);
            }
        }
        Commands::Json => {
            export_to_json();
        }
        Commands::User { username } => {
            get_user_insights(username);
        }
        // Help/Version already handled above
        _ => {}
    }
}

fn export_to_json() {
    let mut commit_stats = gather_commit_stats().expect("Failed to gather commit stats.");
    let loc_and_file_stats = gather_loc_and_file_stats().expect("Failed to gather LOC stats.");

    let mut final_stats = loc_and_file_stats;
    for (author, data) in commit_stats.drain() {
        final_stats.entry(author).or_default().commits = data.commits;
    }

    let mut json_parts = Vec::new();
    for (author, stats) in final_stats.iter() {
        json_parts.push(format!("\"{}\": {}", author, stats.to_json()));
    }
    let json_output = format!("{{\n{}\n}}", json_parts.join(",\n"));
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
