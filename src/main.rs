use bwq::analyze_query;
use clap::{Parser, Subcommand};
use ignore::WalkBuilder;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

#[derive(Parser)]
#[command(name = "bwq")]
#[command(about = "A linter for Brandwatch query files (.bwq)")]
#[command(version = "0.2.0")]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// lint files, directories, or query strings
    #[command(name = "check")]
    Check {
        /// list of files or directories to check [default: .]
        files: Vec<PathBuf>,

        /// lint a query string directly
        #[arg(long)]
        query: Option<String>,

        #[arg(long)]
        no_warnings: bool,

        #[arg(long, default_value = "text")]
        output_format: String,

        /// exit with status code 0, even upon detecting lint violations
        #[arg(long)]
        exit_zero: bool,

        #[arg(long, default_value = "*.bwq")]
        pattern: String,
    },

    /// run in interactive mode
    Interactive {
        #[arg(long)]
        no_warnings: bool,
    },

    /// show example queries
    Examples,

    /// start LSP server
    Lsp,
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Some(Commands::Check {
            files,
            query,
            no_warnings,
            output_format,
            pattern,
            exit_zero,
        }) => {
            if let Some(query_str) = query {
                lint_single_query(&query_str, !no_warnings, &output_format, exit_zero);
            } else if files.is_empty() {
                if let Err(_) =
                    lint_directory(&PathBuf::from("."), !no_warnings, &output_format, &pattern)
                {
                    if !exit_zero {
                        std::process::exit(1);
                    }
                }
            } else {
                process_files(&files, !no_warnings, &output_format, &pattern, exit_zero);
            }
        }
        Some(Commands::Interactive { no_warnings }) => {
            interactive_mode(!no_warnings);
        }
        Some(Commands::Examples) => {
            show_examples();
        }
        Some(Commands::Lsp) => {
            if let Err(e) = bwq::lsp::LspServer::run() {
                eprintln!("LSP server error: {}", e);
                std::process::exit(1);
            }
        }
        None => {
            eprintln!("Error: A subcommand is required");
            eprintln!("\nUsage: bwq <COMMAND>");
            eprintln!("\nCommands:");
            eprintln!("  check        Lint files, directories, or queries");
            eprintln!("  interactive  Run in interactive mode");
            eprintln!("  examples     Show example queries");
            eprintln!("  lsp          Start LSP server");
            eprintln!("\nFor more information, try 'bwq --help'");
            std::process::exit(1);
        }
    }
}

fn process_files(
    files: &[PathBuf],
    show_warnings: bool,
    output_format: &str,
    pattern: &str,
    exit_zero: bool,
) {
    let mut any_errors = false;

    for file_path in files {
        if file_path.is_file() {
            if let Err(_) = lint_file(file_path, show_warnings, output_format) {
                any_errors = true;
            }
        } else if file_path.is_dir() {
            if let Err(_) = lint_directory(file_path, show_warnings, output_format, pattern) {
                any_errors = true;
            }
        } else {
            eprintln!("Path does not exist: {}", file_path.display());
            any_errors = true;
        }
    }

    if any_errors && !exit_zero {
        std::process::exit(1);
    }
}

fn lint_single_query(query: &str, show_warnings: bool, output_format: &str, exit_zero: bool) {
    let analysis = analyze_query(query);

    match output_format {
        "json" => {
            output_json(&analysis);
        }
        _ => {
            output_text(&analysis, show_warnings);
        }
    }

    if !analysis.is_valid && !exit_zero {
        std::process::exit(1);
    }
}

fn lint_file(path: &PathBuf, show_warnings: bool, output_format: &str) -> Result<(), ()> {
    let content = match fs::read_to_string(path) {
        Ok(content) => content,
        Err(e) => {
            eprintln!("Error reading file {}: {}", path.display(), e);
            return Err(());
        }
    };
    let query = content.trim();
    if query.is_empty() {
        eprintln!("File {} is empty", path.display());
        return Err(());
    }

    let analysis = analyze_query(query);

    match output_format {
        "json" => {
            let json_analysis = serde_json::json!({
                "file": path.display().to_string(),
                "valid": analysis.is_valid,
                "errors": analysis.errors.iter().map(|e| e.to_string()).collect::<Vec<_>>(),
                "warnings": analysis.warnings.iter().map(|w| format!("{:?}", w)).collect::<Vec<_>>(),
                "query": query
            });
            println!("{}", serde_json::to_string_pretty(&json_analysis).unwrap());
        }
        _ => {
            println!("File: {}", path.display());
            output_text(&analysis, show_warnings);
            println!();
        }
    }

    if !analysis.is_valid {
        return Err(());
    }
    Ok(())
}

fn lint_directory(
    path: &Path,
    show_warnings: bool,
    output_format: &str,
    pattern: &str,
) -> Result<(), ()> {
    let mut builder = WalkBuilder::new(path);
    builder.hidden(false);

    let mut total_files = 0;
    let mut valid_files = 0;
    let mut any_errors = false;
    let mut results = Vec::new();

    for entry in builder.build() {
        match entry {
            Ok(dir_entry) => {
                let file_path = dir_entry.path();

                if file_path.is_file()
                    && file_path.extension().and_then(|s| s.to_str()) == Some("bwq")
                {
                    total_files += 1;

                    let content = match fs::read_to_string(&file_path) {
                        Ok(content) => content,
                        Err(e) => {
                            eprintln!("Error reading file {}: {}", file_path.display(), e);
                            any_errors = true;
                            continue;
                        }
                    };

                    let query = content.trim();
                    if query.is_empty() {
                        eprintln!("Skipping empty file: {}", file_path.display());
                        continue;
                    }

                    let analysis = analyze_query(query);

                    if analysis.is_valid {
                        valid_files += 1;
                    } else {
                        any_errors = true;
                    }

                    results.push((file_path.to_path_buf(), analysis, query.to_string()));
                }
            }
            Err(e) => {
                eprintln!("Error processing path: {}", e);
                any_errors = true;
            }
        }
    }

    if total_files == 0 {
        eprintln!(
            "No files found matching pattern '{}' in directory '{}'",
            pattern,
            path.display()
        );
        return Err(());
    }
    match output_format {
        "json" => {
            let json_results: Vec<serde_json::Value> = results.into_iter().map(|(file_path, analysis, query)| {
                serde_json::json!({
                    "file": file_path.display().to_string(),
                    "valid": analysis.is_valid,
                    "errors": analysis.errors.iter().map(|e| e.to_string()).collect::<Vec<_>>(),
                    "warnings": analysis.warnings.iter().map(|w| format!("{:?}", w)).collect::<Vec<_>>(),
                    "query": query
                })
            }).collect();

            let summary = serde_json::json!({
                "summary": {
                    "total_files": total_files,
                    "valid_files": valid_files,
                    "invalid_files": total_files - valid_files
                },
                "results": json_results
            });

            println!("{}", serde_json::to_string_pretty(&summary).unwrap());
        }
        _ => {
            for (file_path, analysis, _) in results {
                if !analysis.is_valid || (show_warnings && !analysis.warnings.is_empty()) {
                    println!("File: {}", file_path.display());
                    output_text(&analysis, show_warnings);
                    println!();
                }
            }

            println!("Summary: {}/{} files valid", valid_files, total_files);
        }
    }

    if any_errors {
        return Err(());
    }
    Ok(())
}

fn interactive_mode(show_warnings: bool) {
    println!("Brandwatch Query Linter - Interactive Mode");
    println!("Enter queries to lint (Ctrl+C to exit):");
    println!();

    let stdin = io::stdin();
    let mut stdout = io::stdout();

    loop {
        print!("bwq> ");
        stdout.flush().unwrap();

        let mut line = String::new();
        match stdin.read_line(&mut line) {
            Ok(0) => break, // EOF
            Ok(_) => {
                let query = line.trim();
                if query.is_empty() {
                    continue;
                }

                if query == "exit" || query == "quit" {
                    break;
                }

                if query == "help" {
                    show_interactive_help();
                    continue;
                }

                if query == "examples" {
                    show_examples();
                    continue;
                }

                let analysis = analyze_query(query);
                output_text(&analysis, show_warnings);
                println!();
            }
            Err(e) => {
                eprintln!("Error reading input: {}", e);
                break;
            }
        }
    }
}

fn output_text(analysis: &bwq::AnalysisResult, show_warnings: bool) {
    println!("{}", analysis.summary());

    if !analysis.errors.is_empty() {
        println!("\nErrors:");
        for (i, error) in analysis.errors.iter().enumerate() {
            println!("  {}. {}", i + 1, error);
        }
    }

    if show_warnings && !analysis.warnings.is_empty() {
        println!("\nWarnings:");
        for (i, warning) in analysis.warnings.iter().enumerate() {
            println!("  {}. {:?}", i + 1, warning);
        }
    }
}

fn output_json(analysis: &bwq::AnalysisResult) {
    let json_output = serde_json::json!({
        "valid": analysis.is_valid,
        "summary": analysis.summary(),
        "errors": analysis.errors.iter().map(|e| e.to_string()).collect::<Vec<_>>(),
        "warnings": analysis.warnings.iter().map(|w| format!("{:?}", w)).collect::<Vec<_>>(),
        "query": analysis.query
    });

    println!("{}", serde_json::to_string_pretty(&json_output).unwrap());
}

fn show_interactive_help() {
    println!("Interactive Mode Commands:");
    println!("  help      - Show this help");
    println!("  examples  - Show query examples");
    println!("  exit/quit - Exit interactive mode");
    println!("  <query>   - Lint a query");
    println!();
}

fn show_examples() {
    println!("Brandwatch Query Examples:");
    println!();

    println!("Basic Boolean Operators:");
    println!("  apple AND juice");
    println!("  apple OR orange");
    println!("  apple NOT bitter");
    println!("  (apple OR orange) AND juice");
    println!();

    println!("Quoted Phrases:");
    println!("  \"apple juice\"");
    println!("  \"organic fruit\" AND healthy");
    println!();

    println!("Proximity Operators:");
    println!("  \"apple juice\"~5");
    println!("  apple NEAR/3 juice");
    println!("  apple NEAR/2f juice");
    println!();

    println!("Wildcards and Replacement:");
    println!("  appl*");
    println!("  customi?e");
    println!();

    println!("Field Operators:");
    println!("  title:\"apple juice\"");
    println!("  site:twitter.com");
    println!("  author:brandwatch");
    println!("  language:en");
    println!("  rating:[3 TO 5]");
    println!();

    println!("Location Operators:");
    println!("  country:usa");
    println!("  region:usa.ca");
    println!("  city:\"usa.ca.san francisco\"");
    println!();

    println!("Advanced Operators:");
    println!("  authorFollowers:[1000 TO 50000]");
    println!("  engagementType:RETWEET");
    println!("  authorGender:F");
    println!("  {{BrandWatch}}  (case-sensitive)");
    println!();

    println!("Comments:");
    println!("  apple <<<This is a comment>>> AND juice");
    println!();

    println!("Special Characters:");
    println!("  #MondayMotivation");
    println!("  @brandwatch");
    println!();
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_lint_single_query() {
        let analysis = analyze_query("apple AND juice");
        assert!(analysis.is_valid);
    }

    #[test]
    fn test_file_processing() {
        let mut temp_file = NamedTempFile::new().unwrap();
        writeln!(temp_file, "(apple AND juice)").unwrap();
        writeln!(temp_file, "OR").unwrap();
        writeln!(temp_file, "(orange NOT bitter)").unwrap();
        let content = fs::read_to_string(temp_file.path()).unwrap();
        assert!(content.contains("apple AND juice"));
        let analysis = analyze_query(content.trim());
        assert!(analysis.is_valid);
    }
}
