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
    // user <username> [--ownership] [--by-email|-e] [--top N|--top=N] [--sort loc|pct|--sort=loc]
    User {
        username: String,
        ownership: bool,
        by_email: bool,
        top: Option<usize>,
        sort: Option<String>,
    },
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

    pub fn parse_from_args(args: Vec<String>) -> Result<Cli, String> {
        if args.len() < 2 {
            return Ok(Cli {
                command: Commands::Help {
                    topic: HelpTopic::Top,
                },
            });
        }

        let command_str = &args[1];

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

        let command = match command_str.as_str() {
            "stats" => {
                if has_flag(&args[2..], "-h") || has_flag(&args[2..], "--help") {
                    Commands::Help {
                        topic: HelpTopic::Stats,
                    }
                } else {
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
                        return Err("Usage: git-insights user <username> [--ownership] [--by-email|-e] [--top N] [--sort loc|pct]".to_string());
                    }
                    let username = args[2].clone();
                    let mut ownership = false;
                    let mut by_email = false;
                    let mut top: Option<usize> = None;
                    let mut sort: Option<String> = None;

                    let rest = &args[3..];
                    let mut i = 0;
                    while i < rest.len() {
                        let a = &rest[i];
                        if a == "--ownership" {
                            ownership = true;
                        } else if a == "--by-email" || a == "-e" {
                            by_email = true;
                        } else if a == "--top" {
                            if i + 1 < rest.len() {
                                if let Ok(v) = rest[i + 1].parse::<usize>() {
                                    top = Some(v);
                                }
                                i += 1;
                            }
                        } else if let Some(eq) = a.strip_prefix("--top=") {
                            if let Ok(v) = eq.parse::<usize>() {
                                top = Some(v);
                            }
                        } else if a == "--sort" {
                            if i + 1 < rest.len() {
                                sort = Some(rest[i + 1].to_lowercase());
                                i += 1;
                            }
                        } else if let Some(eq) = a.strip_prefix("--sort=") {
                            sort = Some(eq.to_lowercase());
                        }
                        i += 1;
                    }

                    Commands::User {
                        username,
                        ownership,
                        by_email,
                        top,
                        sort,
                    }
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

fn has_flag(args: &[String], needle: &str) -> bool {
    args.iter().any(|a| a == needle)
}

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
- Only text files considered (git grep -I --name-only . HEAD AND ls-files)
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

Show insights for a specific user.

Default behavior:
- Merged pull request count (via commit message heuristics)
- Tags where the user authored commits

Ownership mode (per-file \"ownership\" list):
- Computes surviving LOC per file attributed to this user at HEAD via blame
- Shows file path, user LOC, file LOC, and ownership percentage

USAGE:
  git-insights user <username> [--ownership] [--by-email|-e] [--top N] [--sort loc|pct]

OPTIONS:
  --ownership       Show per-file ownership table for this user
  -e, --by-email    Match by email (author-mail) instead of author name
  --top N           Limit to top N rows (default: 10)
  --sort loc|pct    Sort by user LOC (loc, default) or percentage (pct)
  -h, --help        Show this help

EXAMPLES:
  git-insights user alice
  git-insights user alice --ownership
  git-insights user \"alice@example.com\" --ownership --by-email --top 5 --sort pct"
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
            Commands::User { username, ownership, by_email, top, sort } => {
                assert_eq!(username, "testuser");
                assert!(!ownership);
                assert!(!by_email);
                assert!(top.is_none());
                assert!(sort.is_none());
            }
            _ => panic!("Expected User command"),
        }
    }

    #[test]
    fn test_cli_user_ownership_flags() {
        let cli = Cli::parse_from_args(vec![
            "git-insights".to_string(),
            "user".to_string(),
            "palash".to_string(),
            "--ownership".to_string(),
            "--by-email".to_string(),
            "--top".to_string(),
            "5".to_string(),
            "--sort".to_string(),
            "pct".to_string(),
        ])
        .expect("Failed to parse args");
        match cli.command {
            Commands::User { username, ownership, by_email, top, sort } => {
                assert_eq!(username, "palash");
                assert!(ownership);
                assert!(by_email);
                assert_eq!(top, Some(5));
                assert_eq!(sort.as_deref(), Some("pct"));
            }
            _ => panic!("Expected User command with ownership flags"),
        }

        // equals-style flags should also parse
        let cli2 = Cli::parse_from_args(vec![
            "git-insights".to_string(),
            "user".to_string(),
            "palash".to_string(),
            "--ownership".to_string(),
            "-e".to_string(),
            "--top=3".to_string(),
            "--sort=loc".to_string(),
        ])
        .expect("Failed to parse args");
        match cli2.command {
            Commands::User { username, ownership, by_email, top, sort } => {
                assert_eq!(username, "palash");
                assert!(ownership);
                assert!(by_email);
                assert_eq!(top, Some(3));
                assert_eq!(sort.as_deref(), Some("loc"));
            }
            _ => panic!("Expected User command with equals-style flags"),
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
        assert_eq!(err, "Usage: git-insights user <username> [--ownership] [--by-email|-e] [--top N] [--sort loc|pct]");
    }
}
