use git_insights::{
    cli::{render_help, version_string, Cli, Commands},
    code_frequency::{run_code_frequency_with_options, Group, HeatmapKind},
    git::{is_git_installed, is_in_git_repo},
    output::{print_user_ownership, print_user_stats},
    stats::{
        gather_commit_stats, gather_loc_and_file_stats, gather_user_stats, get_user_file_ownership,
        run_stats,
    },
    visualize::{run_heatmap_with_options, run_timeline_with_options},
};
use std::fs::File;
use std::io::Write;

fn main() {
    let cli = match Cli::parse() {
        Ok(cli) => cli,
        Err(e) => {
            eprintln!("{}", e);
            std::process::exit(1);
        }
    };

    match &cli.command {
        Commands::Help { topic } => {
            println!("{}", render_help(topic.clone()));
            return;
        }
        Commands::Version => {
            println!("{}", version_string());
            return;
        }
        _ => {}
    }

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
        Commands::User {
            username,
            ownership,
            by_email,
            top,
            sort,
        } => {
            if *ownership {
                let top_n = top.unwrap_or(10);
                let sort_pct = sort.as_deref().map(|s| s == "pct").unwrap_or(false);
                match get_user_file_ownership(username, *by_email, top_n, sort_pct) {
                    Ok(rows) => print_user_ownership(&rows),
                    Err(e) => {
                        eprintln!("Error computing ownership: {}", e);
                        std::process::exit(1);
                    }
                }
            } else {
                get_user_insights(username);
            }
        }
        Commands::Timeline { weeks, color } => {
            let w = weeks.unwrap_or(26);
            if let Err(e) = run_timeline_with_options(w, *color) {
                eprintln!("Error: {}", e);
                std::process::exit(1);
            }
        }
        Commands::Heatmap { weeks, color } => {
            if let Err(e) = run_heatmap_with_options(*weeks, *color) {
                eprintln!("Error: {}", e);
                std::process::exit(1);
            }
        }
        Commands::CodeFrequency {
            group,
            heatmap,
            weeks,
            color,
            table,
        } => {
            let parsed_heatmap = match heatmap.as_deref() {
                Some("dow-hod") => Some(HeatmapKind::DowByHod),
                Some("dom-hod") => Some(HeatmapKind::DomByHod),
                Some(other) => {
                    eprintln!(
                        "Error: unknown --heatmap '{}'. Expected dow-hod|dom-hod.",
                        other
                    );
                    std::process::exit(1);
                }
                None => None,
            };
            let parsed_group = match group.as_deref() {
                Some("hod") => Some(Group::HourOfDay),
                Some("dow") => Some(Group::DayOfWeek),
                Some("dom") => Some(Group::DayOfMonth),
                Some(other) => {
                    eprintln!("Error: unknown --group '{}'. Expected hod|dow|dom.", other);
                    std::process::exit(1);
                }
                None => None,
            };
            if let Err(e) = run_code_frequency_with_options(
                parsed_group,
                parsed_heatmap,
                *weeks,
                *color,
                *table,
            ) {
                eprintln!("Error: {}", e);
                std::process::exit(1);
            }
        }
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
    file.write_all(json_output.as_bytes())
        .expect("Failed to write JSON to file.");
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
