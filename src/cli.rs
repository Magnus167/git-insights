use clap::{Parser, Subcommand};

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Generate statistics for the repository
    Stats,
    /// Export statistics to a JSON file
    Json,
    /// Get detailed insights for a specific user
    User {
        /// The username to get insights for
        username: String,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cli_stats() {
        let cli = Cli::parse_from(&["git-insights", "stats"]);
        assert!(matches!(cli.command, Commands::Stats));
    }

    #[test]
    fn test_cli_json() {
        let cli = Cli::parse_from(&["git-insights", "json"]);
        assert!(matches!(cli.command, Commands::Json));
    }

    #[test]
    fn test_cli_user() {
        let cli = Cli::parse_from(&["git-insights", "user", "testuser"]);
        match cli.command {
            Commands::User { username } => assert_eq!(username, "testuser"),
            _ => panic!("Expected User command"),
        }
    }
}
