use std::path::PathBuf;

use bwq_linter::AnalysisResult;

pub struct Printer {
    pub format: OutputFormat,
    pub show_warnings: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OutputFormat {
    Text,
    Json,
}

#[derive(Debug)]
pub struct FileResults {
    pub successful: Vec<(PathBuf, AnalysisResult, String)>,
    pub read_errors: usize,
}

impl Default for FileResults {
    fn default() -> Self {
        Self::new()
    }
}

impl FileResults {
    pub fn new() -> Self {
        Self {
            successful: Vec::new(),
            read_errors: 0,
        }
    }

    pub fn total_files_processed(&self) -> usize {
        self.successful.len()
    }

    pub fn valid_files(&self) -> usize {
        self.successful
            .iter()
            .filter(|(_, analysis, _)| analysis.is_valid)
            .count()
    }

    pub fn has_errors(&self) -> bool {
        self.read_errors > 0
            || self
                .successful
                .iter()
                .any(|(_, analysis, _)| !analysis.is_valid)
    }
}

impl From<&str> for OutputFormat {
    fn from(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "json" => OutputFormat::Json,
            _ => OutputFormat::Text,
        }
    }
}

impl Printer {
    pub fn new(format: OutputFormat, show_warnings: bool) -> Self {
        Self {
            format,
            show_warnings,
        }
    }

    pub fn print_analysis(&self, analysis: &AnalysisResult) {
        match self.format {
            OutputFormat::Json => self.print_json(analysis),
            OutputFormat::Text => self.print_text(analysis),
        }
    }

    pub fn print_file_results(&self, results: &FileResults) {
        match self.format {
            OutputFormat::Json => self.print_file_results_json(results),
            OutputFormat::Text => self.print_file_results_text(results),
        }
    }

    fn print_text(&self, analysis: &AnalysisResult) {
        println!("{}", analysis.summary());

        if !analysis.errors.is_empty() {
            println!("\nErrors:");
            for (i, error) in analysis.errors.iter().enumerate() {
                println!("  {}. {}: {}", i + 1, error.code(), error);
            }
        }

        if self.show_warnings && !analysis.warnings.is_empty() {
            println!("\nWarnings:");
            for (i, warning) in analysis.warnings.iter().enumerate() {
                println!("  {}. {}: {}", i + 1, warning.code(), warning);
            }
        }
    }

    fn print_json(&self, analysis: &AnalysisResult) {
        let errors: Vec<_> = analysis.errors.iter().map(|e| e.to_json()).collect();
        let warnings: Vec<_> = if self.show_warnings {
            analysis.warnings.iter().map(|w| w.to_json()).collect()
        } else {
            Vec::new()
        };

        let json_output = serde_json::json!({
            "query": analysis.query,
            "errors": errors,
            "warnings": warnings
        });

        println!("{}", serde_json::to_string_pretty(&json_output).unwrap());
    }

    fn print_file_results_text(&self, results: &FileResults) {
        for (file_path, analysis, _) in &results.successful {
            if !analysis.is_valid || (self.show_warnings && !analysis.warnings.is_empty()) {
                println!("File: {}", file_path.display());
                self.print_analysis(analysis);
                println!();
            }
        }

        let valid_files = results.valid_files();
        let total_files = results.total_files_processed();

        if results.read_errors > 0 {
            println!(
                "Summary: {valid_files}/{total_files} files valid ({} files could not be read)",
                results.read_errors
            );
        } else {
            println!("Summary: {valid_files}/{total_files} files valid");
        }
    }

    fn print_file_results_json(&self, results: &FileResults) {
        let mut errors = Vec::new();
        let mut warnings = Vec::new();

        // Add lint errors and warnings from successful files
        for (file_path, analysis, _) in &results.successful {
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

            if self.show_warnings {
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
        }

        let valid_files = results.valid_files();
        let total_files = results.total_files_processed();

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
}
