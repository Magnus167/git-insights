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
