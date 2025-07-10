use std::fs;
use std::path::{Path, PathBuf};

use ignore::WalkBuilder;
use rayon::prelude::*;

use crate::{
    ExitStatus,
    output::{OutputFormat, Printer},
};
use bwq_linter::analyze_query;

pub fn run_check(
    files: Vec<PathBuf>,
    query: Option<String>,
    no_warnings: bool,
    output_format: String,
    extensions: Vec<String>,
    exit_zero: bool,
) -> Result<ExitStatus, anyhow::Error> {
    let show_warnings = !no_warnings;

    if let Some(query_str) = query {
        Ok(lint_single_query_string(
            &query_str,
            show_warnings,
            &output_format,
            exit_zero,
        ))
    } else if files.is_empty() {
        // Default to current directory when no files specified
        let default_files = vec![PathBuf::from(".")];
        process_files(
            &default_files,
            show_warnings,
            &output_format,
            &extensions,
            exit_zero,
        )
    } else {
        process_files(
            &files,
            show_warnings,
            &output_format,
            &extensions,
            exit_zero,
        )
    }
}

fn process_files(
    files: &[PathBuf],
    show_warnings: bool,
    output_format: &str,
    extensions: &[String],
    exit_zero: bool,
) -> Result<ExitStatus, anyhow::Error> {
    // Validate that all paths exist
    for file_path in files {
        if !file_path.exists() {
            eprintln!("Path does not exist: {}", file_path.display());
            return Ok(if exit_zero {
                ExitStatus::Success
            } else {
                ExitStatus::Error
            });
        }
    }

    // Use unified linting approach
    match lint_paths(files, show_warnings, output_format, extensions) {
        Ok(()) => Ok(ExitStatus::Success),
        Err(()) => Ok(if exit_zero {
            ExitStatus::Success
        } else {
            ExitStatus::LintFailure
        }),
    }
}

fn lint_single_query_string(
    query: &str,
    show_warnings: bool,
    output_format: &str,
    exit_zero: bool,
) -> ExitStatus {
    let analysis = analyze_query(query);
    let printer = Printer::new(OutputFormat::from(output_format), show_warnings);
    printer.print_analysis(&analysis);

    if analysis.is_valid || exit_zero {
        ExitStatus::Success
    } else {
        ExitStatus::LintFailure
    }
}

fn matches_extensions(file_path: &Path, extensions: &[String]) -> bool {
    if let Some(file_ext) = file_path.extension().and_then(|ext| ext.to_str()) {
        extensions.iter().any(|ext| ext == file_ext)
    } else {
        false
    }
}

fn discover_files(paths: &[PathBuf], extensions: &[String]) -> Vec<PathBuf> {
    let mut discovered_files = Vec::new();

    for path in paths {
        if path.is_file() {
            // For explicit file arguments, include them regardless of extension
            discovered_files.push(path.clone());
        } else if path.is_dir() {
            // For directories, discover files with matching extensions
            let mut builder = WalkBuilder::new(path);
            builder.hidden(false);

            for dir_entry in builder.build().flatten() {
                let file_path = dir_entry.path();
                if file_path.is_file() && matches_extensions(file_path, extensions) {
                    discovered_files.push(file_path.to_path_buf());
                }
            }
        }
    }

    discovered_files
}

fn lint_paths(
    paths: &[PathBuf],
    show_warnings: bool,
    output_format: &str,
    extensions: &[String],
) -> Result<(), ()> {
    let files = discover_files(paths, extensions);

    if files.is_empty() {
        eprintln!(
            "No files found that have the extension(s): {}",
            extensions.join(", ")
        );
        return Err(());
    }

    // process files in parallel
    let results: Vec<_> = files
        .par_iter()
        .filter_map(|file_path| {
            let content = match fs::read_to_string(file_path) {
                Ok(content) => content,
                Err(e) => {
                    eprintln!("Error reading file {}: {}", file_path.display(), e);
                    return None;
                }
            };

            let query = content.trim();
            let analysis = analyze_query(query);
            Some((file_path.clone(), analysis, query.to_string()))
        })
        .collect();

    let total_files = results.len();
    let valid_files = results
        .iter()
        .filter(|(_, analysis, _)| analysis.is_valid)
        .count();
    let any_errors = results.iter().any(|(_, analysis, _)| !analysis.is_valid);

    let format = OutputFormat::from(output_format);
    match format {
        OutputFormat::Json => {
            let mut errors = Vec::new();
            let mut warnings = Vec::new();

            for (file_path, analysis, _) in &results {
                for error in &analysis.errors {
                    let mut error_json = error.to_json();
                    if let Some(obj) = error_json.as_object_mut() {
                        obj.insert(
                            "filename".to_string(),
                            serde_json::Value::String(file_path.display().to_string()),
                        );
                    }
                    errors.push(error_json);
                }

                for warning in &analysis.warnings {
                    let mut warning_json = warning.to_json();
                    if let Some(obj) = warning_json.as_object_mut() {
                        obj.insert(
                            "filename".to_string(),
                            serde_json::Value::String(file_path.display().to_string()),
                        );
                    }
                    warnings.push(warning_json);
                }
            }

            let output = serde_json::json!({
                "summary": {
                    "total_files": total_files,
                    "valid_files": valid_files,
                    "invalid_files": total_files - valid_files
                },
                "errors": errors,
                "warnings": warnings
            });

            println!("{}", serde_json::to_string_pretty(&output).unwrap());
        }
        OutputFormat::Text => {
            let printer = Printer::new(format, show_warnings);
            for (file_path, analysis, _) in &results {
                if !analysis.is_valid || (show_warnings && !analysis.warnings.is_empty()) {
                    println!("File: {}", file_path.display());
                    printer.print_analysis(analysis);
                    println!();
                }
            }

            println!("Summary: {valid_files}/{total_files} files valid");
        }
    }

    if any_errors { Err(()) } else { Ok(()) }
}
