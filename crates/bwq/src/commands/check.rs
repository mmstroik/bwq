use std::fs;
use std::path::{Path, PathBuf};

use ignore::WalkBuilder;
use rayon::prelude::*;

use crate::{
    ExitStatus,
    output::{FileResults, OutputFormat, Printer},
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
        Ok(check_single_query_string(
            &query_str,
            show_warnings,
            &output_format,
            exit_zero,
        ))
    } else {
        let target_files = if files.is_empty() {
            vec![PathBuf::from(".")]
        } else {
            files
        };

        let results = check_files(&target_files, &extensions)?;

        let printer = Printer::new(OutputFormat::from(output_format.as_str()), show_warnings);
        printer.print_file_results(&results);

        Ok(if results.has_errors() && !exit_zero {
            ExitStatus::LintFailure
        } else {
            ExitStatus::Success
        })
    }
}

fn check_files(paths: &[PathBuf], extensions: &[String]) -> Result<FileResults, anyhow::Error> {
    // Validate that all paths exist
    for file_path in paths {
        if !file_path.exists() {
            anyhow::bail!("Path does not exist: {}", file_path.display());
        }
    }

    let files = discover_files(paths, extensions);

    if files.is_empty() {
        eprintln!(
            "Warning: No files found that have the extension(s): {}",
            extensions.join(", ")
        );
        return Ok(FileResults::new());
    }

    let results: Vec<_> = files
        .par_iter()
        .map(|file_path| match fs::read_to_string(file_path) {
            Ok(content) => {
                let query = content.trim();
                let analysis = analyze_query(query);
                Ok((file_path.clone(), analysis, query.to_string()))
            }
            Err(e) => {
                eprintln!("Error reading file {}: {}", file_path.display(), e);
                Err(file_path.clone())
            }
        })
        .collect();

    let read_errors = results.iter().filter(|r| r.is_err()).count();
    let successful = results.into_iter().filter_map(|r| r.ok()).collect();

    Ok(FileResults {
        successful,
        read_errors,
    })
}

fn check_single_query_string(
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
