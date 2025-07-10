use clap::Parser;

use bwq::args::Cli;

fn main() {
    let args = Cli::parse();

    match bwq::run(args) {
        Ok(exit_status) => std::process::exit(exit_status.into()),
        Err(e) => {
            eprintln!("Error: {e}");
            std::process::exit(1);
        }
    }
}
