use bw_bool::{analyze_query, BrandwatchLinter};
use clap::{Parser, Subcommand};
use std::fs;
use std::io::{self, BufRead, Write};
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "bw-bool")]
#[command(about = "A linter for Brandwatch boolean search queries")]
#[command(version = "0.1.0")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Lint {
        query: String,
        #[arg(short, long)]
        warnings: bool,
        #[arg(short, long, default_value = "text")]
        format: String,
    },
    
    File {
        path: PathBuf,
        #[arg(short, long)]
        warnings: bool,
        #[arg(short, long, default_value = "text")]
        format: String,
    },
    
    Interactive {
        #[arg(short, long)]
        warnings: bool,
    },
    
    Validate {
        query: String,
    },
    
    Examples,
}

fn main() {
    let cli = Cli::parse();
    
    match cli.command {
        Commands::Lint { query, warnings, format } => {
            lint_single_query(&query, warnings, &format);
        }
        Commands::File { path, warnings, format } => {
            lint_file(&path, warnings, &format);
        }
        Commands::Interactive { warnings } => {
            interactive_mode(warnings);
        }
        Commands::Validate { query } => {
            validate_query(&query);
        }
        Commands::Examples => {
            show_examples();
        }
    }
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
    
    // Exit with error code if there are errors
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
    
    let mut has_errors = false;
    let mut total_queries = 0;
    let mut valid_queries = 0;
    
    for (line_num, line) in content.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        
        total_queries += 1;
        let analysis = analyze_query(line);
        
        if analysis.is_valid {
            valid_queries += 1;
        } else {
            has_errors = true;
        }
        
        match format {
            "json" => {
                let mut json_analysis = serde_json::json!({
                    "line": line_num + 1,
                    "query": line,
                    "valid": analysis.is_valid,
                    "errors": analysis.errors.iter().map(|e| e.to_string()).collect::<Vec<_>>(),
                    "warnings": analysis.warnings.iter().map(|w| format!("{:?}", w)).collect::<Vec<_>>(),
                });
                println!("{}", serde_json::to_string_pretty(&json_analysis).unwrap());
            }
            "text" | _ => {
                if !analysis.is_valid || (show_warnings && !analysis.warnings.is_empty()) {
                    println!("Line {}: {}", line_num + 1, line);
                    output_text(&analysis, show_warnings);
                    println!();
                }
            }
        }
    }
    
    if format == "text" {
        println!("Summary: {}/{} queries valid", valid_queries, total_queries);
    }
    
    if has_errors {
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
        print!("bw-bool> ");
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

fn output_text(analysis: &bw_bool::AnalysisResult, show_warnings: bool) {
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

fn output_json(analysis: &bw_bool::AnalysisResult) {
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
        writeln!(temp_file, "apple AND juice").unwrap();
        writeln!(temp_file, "# This is a comment").unwrap();
        writeln!(temp_file, "*invalid wildcard").unwrap();
        writeln!(temp_file, "").unwrap();
        writeln!(temp_file, "valid query").unwrap();
        
        // The actual file testing would require running the CLI
        // For now, just test that we can read the file
        let content = fs::read_to_string(temp_file.path()).unwrap();
        assert!(content.contains("apple AND juice"));
    }
}