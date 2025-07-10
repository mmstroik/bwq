use std::path::PathBuf;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "bwq")]
#[command(about = "A linter for Brandwatch query files (.bwq)")]
#[command(version = "0.2.0")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Subcommand)]
pub enum Commands {
    /// lint files, directories, or query strings
    #[command(name = "check")]
    Check {
        /// Files or directories to check (ignored if --query is used) [default: .]
        files: Vec<PathBuf>,

        /// Lint a query string directly (instead of files)
        #[arg(long, short = 'q')]
        query: Option<String>,

        /// Suppress warning messages
        #[arg(long)]
        no_warnings: bool,

        /// Output format (text or json)
        #[arg(long, default_value = "text")]
        output_format: String,

        /// Exit with status code 0, even upon detecting lint violations
        #[arg(long)]
        exit_zero: bool,

        /// File extensions to check (can be used multiple times)
        #[arg(long = "extension", short = 'e', default_values = ["bwq"])]
        extensions: Vec<String>,
    },

    /// Show example queries
    Examples,

    /// Start language server
    Server,
}
