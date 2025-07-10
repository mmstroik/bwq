pub mod args;
pub mod commands;
pub mod output;

use args::Cli;
use commands::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExitStatus {
    Success = 0,
    LintFailure = 1,
    Error = 2,
}

impl From<ExitStatus> for i32 {
    fn from(status: ExitStatus) -> Self {
        status as i32
    }
}

pub fn run(args: Cli) -> Result<ExitStatus, anyhow::Error> {
    match args.command {
        Some(args::Commands::Check {
            files,
            query,
            no_warnings,
            output_format,
            extensions,
            exit_zero,
        }) => run_check(
            files,
            query,
            no_warnings,
            output_format,
            extensions,
            exit_zero,
        ),
        Some(args::Commands::Examples) => run_examples(),
        Some(args::Commands::Server) => run_server(),
        None => {
            eprintln!("Error: A subcommand is required");
            eprintln!("\nUsage: bwq <COMMAND>");
            eprintln!("\nCommands:");
            eprintln!("  check        Lint files, directories, or queries");
            eprintln!("  examples     Show example queries");
            eprintln!("  server       Start language server");
            eprintln!("\nFor more information, try 'bwq --help'");
            Ok(ExitStatus::Error)
        }
    }
}
