#[derive(Debug)]
pub enum Commands {
    Stats,
    Json,
    User { username: String },
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

    // This function is made public for testing purposes, allowing us to pass custom arguments.
    pub fn parse_from_args(args: Vec<String>) -> Result<Cli, String> {
        if args.len() < 2 {
            return Err("Usage: git-insights [stats|json|user <username>]".to_string());
        }

        let command_str = &args[1];

        let command = match command_str.as_str() {
            "stats" => Commands::Stats,
            "json" => Commands::Json,
            "user" => {
                if args.len() < 3 {
                    return Err("Usage: git-insights user <username>".to_string());
                }
                let username = args[2].clone();
                Commands::User { username }
            }
            _ => {
                return Err(format!(
                    "Unknown command: {}\nUsage: git-insights [stats|json|user <username>]",
                    command_str
                ));
            }
        };

        Ok(Cli { command })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cli_stats() {
        let cli = Cli::parse_from_args(vec!["git-insights".to_string(), "stats".to_string()])
            .expect("Failed to parse args");
        assert!(matches!(cli.command, Commands::Stats));
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
    fn test_cli_no_args() {
        let err = Cli::parse_from_args(vec!["git-insights".to_string()])
            .expect_err("Expected an error for no args");
        assert_eq!(err, "Usage: git-insights [stats|json|user <username>]");
    }

    #[test]
    fn test_cli_unknown_command() {
        let err = Cli::parse_from_args(vec!["git-insights".to_string(), "invalid".to_string()])
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
