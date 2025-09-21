#[derive(Debug, Clone)]
pub enum HelpTopic {
    Top,
    Stats,
    Json,
    User,
}

#[derive(Debug)]
pub enum Commands {
    // Default grouping by author name; pass --by-email/-e to group by name+email
    Stats { by_name: bool },
    Json,
    User { username: String },
    Help { topic: HelpTopic },
    Version,
}

#[derive(Debug)]
pub struct Cli {
    pub command: Commands,
}

impl Cli {
    pub fn parse() -> Result<Cli, String> {
        let args: Vec<String> = std::env::args().collect();
        Cli::parse_from_args(args)
    }

    // Public for testing; accepts a custom args vector including argv[0]
    pub fn parse_from_args(args: Vec<String>) -> Result<Cli, String> {
        // No subcommand provided: show top-level help
        if args.len() < 2 {
            return Ok(Cli {
                command: Commands::Help {
                    topic: HelpTopic::Top,
                },
            });
        }

        let command_str = &args[1];

        // Global help/version
        if command_str == "-h" || command_str == "--help" {
            return Ok(Cli {
                command: Commands::Help {
                    topic: HelpTopic::Top,
                },
            });
        }
        if command_str == "-v" || command_str == "--version" {
            return Ok(Cli {
                command: Commands::Version,
            });
        }

        // Subcommand parsing
        let command = match command_str.as_str() {
            "stats" => {
                // Per-command help
                if has_flag(&args[2..], "-h") || has_flag(&args[2..], "--help") {
                    Commands::Help {
                        topic: HelpTopic::Stats,
                    }
                } else {
                    // Default to by_name=true; --by-email/-e makes by_name=false
                    let by_email =
                        has_flag(&args[2..], "--by-email") || has_flag(&args[2..], "-e");
                    let by_name = !by_email;
                    Commands::Stats { by_name }
                }
            }
            "json" => {
                if has_flag(&args[2..], "-h") || has_flag(&args[2..], "--help") {
                    Commands::Help {
                        topic: HelpTopic::Json,
                    }
                } else {
                    Commands::Json
                }
            }
            "user" => {
                if has_flag(&args[2..], "-h") || has_flag(&args[2..], "--help") {
                    Commands::Help {
                        topic: HelpTopic::User,
                    }
                } else {
                    if args.len() < 3 {
                        return Err("Usage: git-insights user <username>".to_string());
                    }
                    let username = args[2].clone();
                    Commands::User { username }
                }
            }
            _ => {
                return Err(format!(
                    "Unknown command: {}\n{}",
                    command_str,
                    render_help(HelpTopic::Top)
                ));
            }
        };

        Ok(Cli { command })
    }
}

// Small helper for flag presence
fn has_flag(args: &[String], needle: &str) -> bool {
    args.iter().any(|a| a == needle)
}

// Render help text by topic; returned as a String for flexible printing
pub fn render_help(topic: HelpTopic) -> String {
    match topic {
        HelpTopic::Top => {
            let ver = version_string();
            format!(
                "\
git-insights v{ver}

A CLI tool to generate Git repo stats and insights (no dependencies).

USAGE:
  git-insights <COMMAND> [OPTIONS]

COMMANDS:
  stats           Show repository stats (surviving LOC, commits, files)
  json            Export stats to git-insights.json
  user <name>     Show insights for a specific user
  help            Show this help
  version         Show version information

GLOBAL OPTIONS:
  -h, --help      Show help
  -v, --version   Show version

EXAMPLES:
  git-insights stats
  git-insights stats --by-email
  git-insights json
  git-insights user alice

See 'git-insights <COMMAND> --help' for command-specific options."
            )
        }
        HelpTopic::Stats => {
            "\
git-insights stats

Compute repository stats using a gitfame-like method:
- Surviving LOC via git blame --line-porcelain HEAD
- Commits via git shortlog -s -e HEAD
- Only text files considered (git grep -I --name-only . HEAD ∩ ls-files)
- Clean git commands (no pager), no dependencies

USAGE:
  git-insights stats [OPTIONS]

OPTIONS:
  -e, --by-email  Group by \"Name <email>\" (default groups by name only)
  -h, --help      Show this help

EXAMPLES:
  git-insights stats
  git-insights stats --by-email"
                .to_string()
        }
        HelpTopic::Json => {
            "\
git-insights json

Export stats to a JSON file (git-insights.json) mapping:
  author -> { loc, commits, files[] }

USAGE:
  git-insights json

EXAMPLES:
  git-insights json"
                .to_string()
        }
        HelpTopic::User => {
            "\
git-insights user

Show basic insights for a specific user:
- Merged pull request count (via commit message heuristics)
- Tags where the user authored commits

USAGE:
  git-insights user <username>

EXAMPLES:
  git-insights user alice"
                .to_string()
        }
    }
}

// Expose version pulled from Cargo metadata
pub fn version_string() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cli_stats_default_by_name() {
        let cli = Cli::parse_from_args(vec![
            "git-insights".to_string(),
            "stats".to_string(),
        ])
        .expect("Failed to parse args");
        match cli.command {
            Commands::Stats { by_name } => assert!(by_name),
            _ => panic!("Expected Stats command"),
        }
    }

    #[test]
    fn test_cli_stats_by_email_flag() {
        let cli = Cli::parse_from_args(vec![
            "git-insights".to_string(),
            "stats".to_string(),
            "--by-email".to_string(),
        ])
        .expect("Failed to parse args");
        match cli.command {
            Commands::Stats { by_name } => assert!(!by_name),
            _ => panic!("Expected Stats command"),
        }
    }

    #[test]
    fn test_cli_stats_short_e_flag() {
        let cli = Cli::parse_from_args(vec![
            "git-insights".to_string(),
            "stats".to_string(),
            "-e".to_string(),
        ])
        .expect("Failed to parse args");
        match cli.command {
            Commands::Stats { by_name } => assert!(!by_name),
            _ => panic!("Expected Stats command"),
        }
    }

    #[test]
    fn test_cli_json() {
        let cli = Cli::parse_from_args(vec!["git-insights".to_string(), "json".to_string()])
            .expect("Failed to parse args");
        assert!(matches!(cli.command, Commands::Json));
    }

    #[test]
    fn test_cli_user() {
        let cli = Cli::parse_from_args(vec![
            "git-insights".to_string(),
            "user".to_string(),
            "testuser".to_string(),
        ])
        .expect("Failed to parse args");
        match cli.command {
            Commands::User { username } => assert_eq!(username, "testuser"),
            _ => panic!("Expected User command"),
        }
    }

    #[test]
    fn test_cli_no_args_yields_help() {
        let cli = Cli::parse_from_args(vec!["git-insights".to_string()]).expect("parse");
        match cli.command {
            Commands::Help { topic } => match topic {
                HelpTopic::Top => {}
                _ => panic!("Expected top-level help"),
            },
            _ => panic!("Expected Help command"),
        }
    }

    #[test]
    fn test_cli_top_help_flag() {
        let cli = Cli::parse_from_args(vec![
            "git-insights".to_string(),
            "--help".to_string(),
        ])
        .expect("parse");
        match cli.command {
            Commands::Help { topic } => match topic {
                HelpTopic::Top => {}
                _ => panic!("Expected top-level help"),
            },
            _ => panic!("Expected Help command"),
        }
    }

    #[test]
    fn test_cli_stats_help_flag() {
        let cli = Cli::parse_from_args(vec![
            "git-insights".to_string(),
            "stats".to_string(),
            "--help".to_string(),
        ])
        .expect("parse");
        match cli.command {
            Commands::Help { topic } => match topic {
                HelpTopic::Stats => {}
                _ => panic!("Expected stats help"),
            },
            _ => panic!("Expected Help command"),
        }
    }

    #[test]
    fn test_cli_version_flag() {
        let cli = Cli::parse_from_args(vec![
            "git-insights".to_string(),
            "--version".to_string(),
        ])
        .expect("parse");
        assert!(matches!(cli.command, Commands::Version));
    }

    #[test]
    fn test_cli_unknown_command() {
        let err =
            Cli::parse_from_args(vec!["git-insights".to_string(), "invalid".to_string()])
                .expect_err("Expected an error for unknown command");
        assert!(err.contains("Unknown command: invalid"));
    }

    #[test]
    fn test_cli_user_no_username() {
        let err = Cli::parse_from_args(vec!["git-insights".to_string(), "user".to_string()])
            .expect_err("Expected an error for user command without username");
        assert_eq!(err, "Usage: git-insights user <username>");
    }
}
