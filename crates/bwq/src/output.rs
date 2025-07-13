use std::path::PathBuf;

use bwq_linter::{
    AnalysisResult,
    error::{LintError, LintWarning},
};

#[derive(Debug)]
struct ContextWindow {
    start_line: usize,
    end_line: usize,
}

#[derive(Debug)]
struct UnderlineStyle {
    underline_char: char,
    pipe_indent: String,
    color_start: String,
    color_end: String,
}

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
        if let Some(query) = &analysis.query {
            if !analysis.errors.is_empty() {
                for error in &analysis.errors {
                    self.print_error_with_context(query, error, None);
                    println!();
                }
            }

            if self.show_warnings && !analysis.warnings.is_empty() {
                for warning in &analysis.warnings {
                    self.print_warning_with_context(query, warning, None);
                    println!();
                }
            }
        } else {
            // Fallback to old format if no query
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
    }

    fn print_error_with_context(
        &self,
        query: &str,
        error: &LintError,
        file_path: Option<&PathBuf>,
    ) {
        let span = error.span();

        // Print header in ty-style format with color
        println!("\x1b[1;31merror[{}]\x1b[0m: {}", error.code(), error);

        if let Some(path) = file_path {
            println!(
                "  --> \x1b[1m{}\x1b[0m:{}:{}",
                path.display(),
                span.start.line,
                span.start.column
            );
        } else {
            println!("  --> {}:{}", span.start.line, span.start.column);
        }

        let style = UnderlineStyle {
            underline_char: '^',
            pipe_indent: String::new(), // Will be calculated in the function
            color_start: "\x1b[1;31m".to_string(),
            color_end: "\x1b[0m".to_string(),
        };
        self.print_snippet_with_underline(query, span, style);
    }

    fn print_warning_with_context(
        &self,
        query: &str,
        warning: &LintWarning,
        file_path: Option<&PathBuf>,
    ) {
        let span = warning.span();

        // Print header in ty-style format with color
        println!("\x1b[1;33mwarning[{}]\x1b[0m: {}", warning.code(), warning);

        if let Some(path) = file_path {
            println!(
                "  --> \x1b[1m{}\x1b[0m:{}:{}",
                path.display(),
                span.start.line,
                span.start.column
            );
        } else {
            println!("  --> {}:{}", span.start.line, span.start.column);
        }

        let style = UnderlineStyle {
            underline_char: '^',
            pipe_indent: String::new(), // Will be calculated in the function
            color_start: "\x1b[1;33m".to_string(),
            color_end: "\x1b[0m".to_string(),
        };
        self.print_snippet_with_underline(query, span, style);
    }

    fn print_snippet_with_underline(
        &self,
        query: &str,
        span: &bwq_linter::error::Span,
        mut style: UnderlineStyle,
    ) {
        let lines: Vec<&str> = query.lines().collect();

        // Convert 1-based line numbers to 0-based indices
        let start_line_idx = span.start.line.saturating_sub(1);
        let end_line_idx = span.end.line.saturating_sub(1);

        if start_line_idx >= lines.len() {
            return;
        }

        // Determine context window
        let max_chars_per_context = 200; // Max chars to show per context line
        let max_total_context_chars = 800; // Max total chars in entire display

        let context_result = self.calculate_context_window(
            &lines,
            start_line_idx,
            end_line_idx,
            max_chars_per_context,
            max_total_context_chars,
        );

        // Calculate the width needed for line numbers
        let max_line_num = context_result.end_line + 1; // Convert to 1-based
        let line_num_width = max_line_num.to_string().len();
        style.pipe_indent = " ".repeat(line_num_width); // Align with line number column

        // Print context lines with line numbers
        println!("{} |", style.pipe_indent);

        let mut current_chars = 0;
        for line_idx in context_result.start_line..=context_result.end_line {
            if line_idx >= lines.len() {
                break;
            }

            let line = lines[line_idx];
            let line_num = line_idx + 1; // Convert back to 1-based

            // Check if this line contains the error
            let is_error_line = line_idx >= start_line_idx && line_idx <= end_line_idx;

            // Truncate line if too long
            let display_line = if line.len() > max_chars_per_context {
                self.truncate_line_for_span(
                    line,
                    if is_error_line {
                        Some((span.start.column, span.end.column))
                    } else {
                        None
                    },
                    max_chars_per_context,
                )
            } else {
                (line.to_string(), 0)
            };

            println!(
                "{:width$} | {}",
                line_num,
                display_line.0,
                width = line_num_width
            );

            // Print underline if this is an error line
            if is_error_line {
                self.print_underline_for_line(
                    line_idx,
                    span,
                    &display_line.0,
                    display_line.1,
                    &style,
                );
            }

            current_chars += display_line.0.len();
            if current_chars > max_total_context_chars {
                println!("{} | ... (output truncated)", style.pipe_indent);
                break;
            }
        }

        println!("{} |", style.pipe_indent);
    }

    fn calculate_context_window(
        &self,
        lines: &[&str],
        start_line_idx: usize,
        end_line_idx: usize,
        max_chars_per_line: usize,
        max_total_chars: usize,
    ) -> ContextWindow {
        let mut start_line = start_line_idx;
        let mut end_line = end_line_idx;

        // Try to show 2 lines before and after
        let desired_context = 2;

        // Expand context window while respecting character limits
        let mut total_chars = 0;

        // First, count chars in error lines
        for idx in start_line_idx..=end_line_idx {
            if idx < lines.len() {
                total_chars += lines[idx].len().min(max_chars_per_line);
            }
        }

        // Add context lines before
        for i in 1..=desired_context {
            if start_line_idx >= i {
                let context_line_idx = start_line_idx - i;
                let line_chars = lines[context_line_idx].len().min(max_chars_per_line);
                if total_chars + line_chars <= max_total_chars {
                    start_line = context_line_idx;
                    total_chars += line_chars;
                } else {
                    break;
                }
            }
        }

        // Add context lines after
        for i in 1..=desired_context {
            let context_line_idx = end_line_idx + i;
            if context_line_idx < lines.len() {
                let line_chars = lines[context_line_idx].len().min(max_chars_per_line);
                if total_chars + line_chars <= max_total_chars {
                    end_line = context_line_idx;
                    total_chars += line_chars;
                } else {
                    break;
                }
            }
        }

        ContextWindow {
            start_line,
            end_line,
        }
    }

    fn truncate_line_for_span(
        &self,
        line: &str,
        span_cols: Option<(usize, usize)>,
        max_chars: usize,
    ) -> (String, usize) {
        if line.len() <= max_chars {
            return (line.to_string(), 0);
        }

        let ellipsis = "…";
        let available_chars = max_chars - (ellipsis.len() * 2); // Reserve space for ellipsis on both sides

        if let Some((start_col, end_col)) = span_cols {
            // Center around the error span
            let span_start = start_col.saturating_sub(1); // Convert to 0-based
            let span_end = end_col.saturating_sub(1).min(line.len());
            let span_len = span_end.saturating_sub(span_start);

            // If the span itself is too long, just show the span
            if span_len >= available_chars {
                let truncated = line
                    .chars()
                    .skip(span_start)
                    .take(available_chars)
                    .collect::<String>();
                return (format!("{ellipsis}{truncated}…"), span_start);
            }

            // Try to center around the span
            let context_per_side = (available_chars - span_len) / 2;
            let window_start = span_start.saturating_sub(context_per_side);
            let window_end = (span_end + context_per_side).min(line.len());

            let mut result = String::new();
            let mut actual_start = window_start;

            if window_start > 0 {
                result.push_str(ellipsis);
            } else {
                actual_start = 0;
            }

            result.push_str(
                &line
                    .chars()
                    .skip(window_start)
                    .take(window_end - window_start)
                    .collect::<String>(),
            );

            if window_end < line.len() {
                result.push_str(ellipsis);
            }

            (result, actual_start)
        } else {
            // No specific span, just truncate from the beginning
            let truncated = line.chars().take(available_chars).collect::<String>();
            (format!("{truncated}…"), 0)
        }
    }

    fn print_underline_for_line(
        &self,
        line_idx: usize,
        span: &bwq_linter::error::Span,
        display_line: &str,
        start_offset: usize,
        style: &UnderlineStyle,
    ) {
        let start_line_idx = span.start.line.saturating_sub(1);
        let end_line_idx = span.end.line.saturating_sub(1);

        if line_idx < start_line_idx || line_idx > end_line_idx {
            return;
        }

        // Calculate column positions (convert from 1-based to 0-based)
        let line_start_col = if line_idx == start_line_idx {
            span.start.column.saturating_sub(1)
        } else {
            0
        };

        let line_end_col = if line_idx == end_line_idx {
            span.end.column.saturating_sub(1)
        } else {
            display_line.chars().count()
        };

        // Adjust for truncation offset
        let display_start_col = line_start_col.saturating_sub(start_offset);
        let display_end_col = line_end_col.saturating_sub(start_offset);

        if display_start_col >= display_line.chars().count() {
            return;
        }

        let actual_end_col = display_end_col.min(display_line.chars().count());

        if actual_end_col <= display_start_col {
            return;
        }

        // Build underline string
        let mut underline = String::new();

        // Add spaces before the underline
        for _ in 0..display_start_col {
            underline.push(' ');
        }

        // Add underline characters
        for _ in display_start_col..actual_end_col {
            underline.push(style.underline_char);
        }

        if !underline.trim().is_empty() {
            println!(
                "{} | {}{underline}{}",
                style.pipe_indent, style.color_start, style.color_end
            );
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
        for (file_path, analysis, query) in &results.successful {
            if !analysis.is_valid || (self.show_warnings && !analysis.warnings.is_empty()) {
                // Print errors with file context
                for error in &analysis.errors {
                    self.print_error_with_context(query, error, Some(file_path));
                    println!();
                }

                // Print warnings with file context
                if self.show_warnings {
                    for warning in &analysis.warnings {
                        self.print_warning_with_context(query, warning, Some(file_path));
                        println!();
                    }
                }
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
