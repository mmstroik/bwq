use bwq_lint::{analyze_query, BrandwatchLinter};
use clap::{Parser, Subcommand};
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

#[derive(Parser)]
#[command(name = "bwq-lint")]
#[command(about = "A linter for Brandwatch query files (.bwq)")]
#[command(version = "0.1.0")]
struct Cli {
    /// Input to analyze - can be a query string, file path, directory, or glob pattern
    input: Option<String>,

    #[arg(long)]
    no_warnings: bool,

    #[arg(short, long, default_value = "text")]
    format: String,

    #[arg(long, default_value = "*.bwq")]
    pattern: String,

    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Run in interactive mode
    Interactive {
        #[arg(long)]
        no_warnings: bool,
    },

    /// Validate a query (returns exit code 0/1)
    Validate { query: String },

    /// Show example queries
    Examples,

    /// Lint a specific query string (explicit)
    Lint {
        query: String,
        #[arg(long)]
        no_warnings: bool,
        #[arg(short, long, default_value = "text")]
        format: String,
    },

    /// Lint a specific file (explicit)
    File {
        path: PathBuf,
        #[arg(long)]
        no_warnings: bool,
        #[arg(short, long, default_value = "text")]
        format: String,
    },

    /// Lint a directory (explicit)
    Dir {
        #[arg(short, long, default_value = ".")]
        path: PathBuf,
        #[arg(long)]
        no_warnings: bool,
        #[arg(short, long, default_value = "text")]
        format: String,
        #[arg(long, default_value = "*.bwq")]
        pattern: String,
    },
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Some(Commands::Interactive { no_warnings }) => {
            interactive_mode(!no_warnings);
        }
        Some(Commands::Validate { query }) => {
            validate_query(&query);
        }
        Some(Commands::Examples) => {
            show_examples();
        }
        Some(Commands::Lint {
            query,
            no_warnings,
            format,
        }) => {
            lint_single_query(&query, !no_warnings, &format);
        }
        Some(Commands::File {
            path,
            no_warnings,
            format,
        }) => {
            lint_file(&path, !no_warnings, &format);
        }
        Some(Commands::Dir {
            path,
            no_warnings,
            format,
            pattern,
        }) => {
            lint_directory(&path, !no_warnings, &format, &pattern);
        }
        None => {
            if let Some(input) = cli.input {
                auto_detect_and_process(&input, !cli.no_warnings, &cli.format, &cli.pattern);
            } else {
                lint_directory(&PathBuf::from("."), !cli.no_warnings, &cli.format, &cli.pattern);
            }
        }
    }
}

fn auto_detect_and_process(input: &str, show_warnings: bool, format: &str, pattern: &str) {
    let path = Path::new(input);

    if path.exists() {
        if path.is_file() {
            lint_file(&path.to_path_buf(), show_warnings, format);
        } else if path.is_dir() {
            lint_directory(&path.to_path_buf(), show_warnings, format, pattern);
        }
    } else if contains_glob_pattern(input) {
        lint_directory(&PathBuf::from("."), show_warnings, format, input);
    } else {
        lint_single_query(input, show_warnings, format);
    }
}

fn contains_glob_pattern(input: &str) -> bool {
    input.contains('*') || input.contains('?') || input.contains('[') || input.contains('{')
}

fn lint_single_query(query: &str, show_warnings: bool, format: &str) {
    let analysis = analyze_query(query);

    match format {
        "json" => {
            output_json(&analysis);
        }
        "text" | _ => {
            output_text(&analysis, show_warnings);
        }
    }

    if !analysis.is_valid {
        std::process::exit(1);
    }
}

fn lint_file(path: &PathBuf, show_warnings: bool, format: &str) {
    let content = match fs::read_to_string(path) {
        Ok(content) => content,
        Err(e) => {
            eprintln!("Error reading file {}: {}", path.display(), e);
            std::process::exit(1);
        }
    };
    let query = content.trim();
    if query.is_empty() {
        eprintln!("File {} is empty", path.display());
        std::process::exit(1);
    }

    let analysis = analyze_query(query);

    match format {
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
        "text" | _ => {
            println!("File: {}", path.display());
            output_text(&analysis, show_warnings);
            println!();
        }
    }

    if !analysis.is_valid {
        std::process::exit(1);
    }
}

fn lint_directory(path: &PathBuf, show_warnings: bool, format: &str, pattern: &str) {
    let search_pattern = if path.display().to_string() == "." {
        format!("**/{}", pattern)
    } else {
        format!("{}/**/{}", path.display(), pattern)
    };

    let entries = match glob::glob(&search_pattern) {
        Ok(entries) => entries,
        Err(e) => {
            eprintln!("Error parsing glob pattern '{}': {}", search_pattern, e);
            std::process::exit(1);
        }
    };

    let mut total_files = 0;
    let mut valid_files = 0;
    let mut any_errors = false;
    let mut results = Vec::new();

    for entry in entries {
        match entry {
            Ok(file_path) => {
                if file_path.is_file() {
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

                    results.push((file_path, analysis, query.to_string()));
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
        std::process::exit(1);
    }
    match format {
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
        "text" | _ => {
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
        std::process::exit(1);
    }
}

fn interactive_mode(show_warnings: bool) {
    println!("Brandwatch Query Linter - Interactive Mode");
    println!("Enter queries to lint (Ctrl+C to exit):");
    println!();

    let stdin = io::stdin();
    let mut stdout = io::stdout();

    loop {
        print!("bwq-lint> ");
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

fn validate_query(query: &str) {
    let mut linter = BrandwatchLinter::new();
    let is_valid = linter.is_valid(query);

    if is_valid {
        println!("Query is valid");
        std::process::exit(0);
    } else {
        println!("Query is invalid");
        std::process::exit(1);
    }
}

fn output_text(analysis: &bwq_lint::AnalysisResult, show_warnings: bool) {
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

fn output_json(analysis: &bwq_lint::AnalysisResult) {
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
        // This is more of an integration test
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
