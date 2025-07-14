use std::path::PathBuf;

use bwq_linter::{
    AnalysisResult,
    error::{LintError, LintWarning},
};

#[derive(Debug)]
struct LineContextWindow {
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

/// Display width calculation utilities
mod width_utils {
    use unicode_width::UnicodeWidthChar;

    /// Calculate the display width of a character, handling tabs and Unicode properly
    pub fn char_width(c: char) -> usize {
        if c == '\t' {
            4
        } else {
            UnicodeWidthChar::width(c).unwrap_or(0)
        }
    }

    /// Calculate the display width of a string
    pub fn str_width(s: &str) -> usize {
        s.chars().map(char_width).sum()
    }
}

use width_utils::{char_width, str_width};

#[derive(Debug)]
struct ContextWindow {
    start_char: usize,
    end_char: usize,
}

/// Find the optimal context window around an error span for truncation
fn find_context_window(
    chars: &[char],
    span_start: usize,
    span_end: usize,
    span_width: usize,
    available_width: usize,
) -> ContextWindow {
    let remaining_width = available_width - span_width;
    let context_width_per_side = remaining_width / 2;

    // Find how many characters we can include before the span
    let mut before_width = 0;
    let mut window_start = span_start;

    for i in (0..span_start).rev() {
        let ch_width = char_width(chars[i]);
        if before_width + ch_width > context_width_per_side {
            break;
        }
        before_width += ch_width;
        window_start = i;
    }

    // Find how many characters we can include after the span
    let mut after_width = 0;
    let mut window_end = span_end;

    for (offset, &ch) in chars.iter().enumerate().skip(span_end) {
        let ch_width = char_width(ch);
        if after_width + ch_width > context_width_per_side {
            break;
        }
        after_width += ch_width;
        window_end = offset + 1;
    }

    ContextWindow {
        start_char: window_start,
        end_char: window_end,
    }
}

/// Build the truncated display string with ellipses
fn build_truncated_display(
    chars: &[char],
    window: ContextWindow,
    ellipsis: &str,
) -> (String, usize) {
    let mut result = String::new();

    // Add left ellipsis if needed and determine character offset
    let char_offset = if window.start_char > 0 {
        result.push_str(ellipsis);
        window.start_char
    } else {
        0
    };

    for &ch in &chars[window.start_char..window.end_char] {
        result.push(ch);
    }

    // Add right ellipsis if needed
    if window.end_char < chars.len() {
        result.push_str(ellipsis);
    }

    (result, char_offset)
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
        if !analysis.errors.is_empty() {
            for error in &analysis.errors {
                self.print_error_with_context(&analysis.query, error, None);
                println!();
            }
        }

        if self.show_warnings && !analysis.warnings.is_empty() {
            for warning in &analysis.warnings {
                self.print_warning_with_context(&analysis.query, warning, None);
                println!();
            }
        }
        let warning_count = if self.show_warnings { analysis.warnings.len() } else { 0 };
        let error_count = analysis.errors.len();
        if error_count == 0 && warning_count == 0 {
            println!("All checks passed!");
        } else {
            println!("Found {} diagnostics", error_count + warning_count);
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
                for error in &analysis.errors {
                    self.print_error_with_context(query, error, Some(file_path));
                    println!();
                }

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
        } else if total_files == valid_files {
            println!("All checks passed!");
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
            pipe_indent: String::new(),
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
            pipe_indent: String::new(),
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

        let max_chars_per_context = 200;
        let max_total_context_chars = 800;

        let context_result = self.calculate_context_window(
            &lines,
            start_line_idx,
            end_line_idx,
            max_chars_per_context,
            max_total_context_chars,
        );

        let max_line_num = context_result.end_line + 1; // Convert to 1-based
        let line_num_width = max_line_num.to_string().len();
        style.pipe_indent = " ".repeat(line_num_width); // Align with line number column

        println!("{} |", style.pipe_indent);

        let mut current_chars = 0;
        for line_idx in context_result.start_line..=context_result.end_line {
            if line_idx >= lines.len() {
                break;
            }

            let line = lines[line_idx];
            let line_num = line_idx + 1; // Convert back to 1-based

            let is_error_line = line_idx >= start_line_idx && line_idx <= end_line_idx;

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
    ) -> LineContextWindow {
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

        LineContextWindow {
            start_line,
            end_line,
        }
    }

    fn truncate_line_for_span(
        &self,
        line: &str,
        span_cols: Option<(usize, usize)>,
        max_width: usize,
    ) -> (String, usize) {
        let line_width = str_width(line);
        if line_width <= max_width {
            return (line.to_string(), 0);
        }

        let ellipsis = "…";
        let ellipsis_width = str_width(ellipsis);

        // Prevent integer underflow - need at least space for two ellipses plus one character
        if max_width < ellipsis_width * 2 + 1 {
            // If max_width is too small, just return the ellipsis
            return (ellipsis.to_string(), 0);
        }

        let available_width = max_width.saturating_sub(ellipsis_width * 2);
        let chars: Vec<char> = line.chars().collect();

        if let Some((start_col, end_col)) = span_cols {
            // Convert to 0-based character indices
            let span_start_char = start_col.saturating_sub(1);
            let span_end_char = end_col.saturating_sub(1);

            let span_start_char = span_start_char.min(chars.len());
            let span_end_char = span_end_char.min(chars.len());

            let span_width: usize = chars[span_start_char..span_end_char]
                .iter()
                .map(|&c| char_width(c))
                .sum();

            // If the span itself is too wide, just show what we can of the span
            if span_width >= available_width {
                return self.truncate_span_only(
                    &chars,
                    span_start_char,
                    span_end_char,
                    available_width,
                    ellipsis,
                );
            }

            let window = find_context_window(
                &chars,
                span_start_char,
                span_end_char,
                span_width,
                available_width,
            );

            build_truncated_display(&chars, window, ellipsis)
        } else {
            // No specific span, just truncate from the beginning
            self.truncate_from_start(&chars, available_width, ellipsis)
        }
    }

    fn truncate_span_only(
        &self,
        chars: &[char],
        span_start: usize,
        span_end: usize,
        available_width: usize,
        ellipsis: &str,
    ) -> (String, usize) {
        let mut result_width = 0;
        let mut truncated = String::new();

        for &ch in &chars[span_start..span_end] {
            let ch_width = char_width(ch);
            if result_width + ch_width > available_width {
                break;
            }
            result_width += ch_width;
            truncated.push(ch);
        }

        (format!("{ellipsis}{truncated}{ellipsis}"), span_start)
    }

    fn truncate_from_start(
        &self,
        chars: &[char],
        available_width: usize,
        ellipsis: &str,
    ) -> (String, usize) {
        let mut result_width = 0;
        let mut truncated = String::new();

        for &ch in chars {
            let ch_width = char_width(ch);
            if result_width + ch_width > available_width {
                break;
            }
            result_width += ch_width;
            truncated.push(ch);
        }

        (format!("{truncated}{ellipsis}"), 0)
    }

    fn print_underline_for_line(
        &self,
        line_idx: usize,
        span: &bwq_linter::error::Span,
        display_line: &str,
        char_offset: usize,
        style: &UnderlineStyle,
    ) {
        let start_line_idx = span.start.line.saturating_sub(1);
        let end_line_idx = span.end.line.saturating_sub(1);

        if line_idx < start_line_idx || line_idx > end_line_idx {
            return;
        }

        // Convert display line to character vector for easier manipulation
        let display_chars: Vec<char> = display_line.chars().collect();

        // Calculate column positions (convert from 1-based to 0-based character indices)
        let line_start_char = if line_idx == start_line_idx {
            span.start.column.saturating_sub(1)
        } else {
            0
        };

        let line_end_char = if line_idx == end_line_idx {
            span.end.column.saturating_sub(1)
        } else {
            // For multiline spans, underline to the end of the visible line
            display_chars.len()
        };

        // Adjust for truncation - char_offset is how many characters were removed from the left
        let display_start_char = line_start_char.saturating_sub(char_offset);
        let display_end_char = line_end_char.saturating_sub(char_offset);

        // Check if the span is visible in the truncated line
        if display_start_char >= display_chars.len() {
            return;
        }

        let actual_end_char = display_end_char.min(display_chars.len());

        if actual_end_char <= display_start_char {
            return;
        }

        let mut underline = String::new();

        // Add spaces for characters before the underline
        for &ch in &display_chars[0..display_start_char] {
            let ch_width = char_width(ch);
            for _ in 0..ch_width {
                underline.push(' ');
            }
        }

        // Add underline characters for the span
        for &ch in &display_chars[display_start_char..actual_end_char] {
            let ch_width = char_width(ch);
            for _ in 0..ch_width {
                underline.push(style.underline_char);
            }
        }

        if !underline.trim().is_empty() {
            println!(
                "{} | {}{underline}{}",
                style.pipe_indent, style.color_start, style.color_end
            );
        }
    }
}
