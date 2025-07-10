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
        let warnings: Vec<_> = analysis.warnings.iter().map(|w| w.to_json()).collect();

        let json_output = serde_json::json!({
            "query": analysis.query,
            "errors": errors,
            "warnings": warnings
        });

        println!("{}", serde_json::to_string_pretty(&json_output).unwrap());
    }
}
