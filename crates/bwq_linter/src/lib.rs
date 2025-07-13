pub mod ast;
pub mod error;
pub mod lexer;
pub mod parser;
pub mod validation;
pub mod validator;

use error::{LintError, LintReport, LintResult};
use lexer::Lexer;
use parser::Parser;
use validator::Validator;

pub struct BrandwatchLinter {
    validator: Validator,
}

impl BrandwatchLinter {
    pub fn new() -> Self {
        Self {
            validator: Validator::new(),
        }
    }

    pub fn lint(&mut self, query: &str) -> LintResult<LintReport> {
        let mut lexer = Lexer::new(query);
        let lex_result = lexer.tokenize();

        // Try to parse even if we have lexer errors
        match Parser::new(lex_result.tokens) {
            Ok(mut parser) => {
                match parser.parse() {
                    Ok(parse_result) => {
                        // Successful parsing - combine all errors and warnings
                        let mut report = self.validator.validate(&parse_result.query);

                        // Add lexer errors to the front
                        let mut all_errors = lex_result.errors;
                        all_errors.extend(report.errors);
                        report.errors = all_errors;

                        // Combine warnings
                        report.warnings.extend(parse_result.warnings);

                        Ok(report)
                    }
                    Err(parse_error) => {
                        // Parser failed - combine lexer errors + parser error
                        let mut all_errors = lex_result.errors;
                        all_errors.push(parse_error);

                        Ok(LintReport {
                            errors: all_errors,
                            warnings: vec![],
                        })
                    }
                }
            }
            Err(parser_creation_error) => {
                // Parser creation failed - combine lexer errors + parser creation error
                let mut all_errors = lex_result.errors;
                all_errors.push(parser_creation_error);

                Ok(LintReport {
                    errors: all_errors,
                    warnings: vec![],
                })
            }
        }
    }

    pub fn analyze(&mut self, query: &str) -> AnalysisResult {
        match self.lint(query) {
            Ok(report) => AnalysisResult {
                is_valid: !report.has_errors(),
                errors: report.errors,
                warnings: report.warnings,
                query: Some(query.to_string()),
            },
            Err(error) => AnalysisResult {
                is_valid: false,
                errors: vec![error],
                warnings: vec![],
                query: Some(query.to_string()),
            },
        }
    }

    pub fn analyze_and_skip_empty(&mut self, query: &str) -> AnalysisResult {
        if query.trim().is_empty() {
            return AnalysisResult {
                is_valid: true,
                errors: Vec::new(),
                warnings: Vec::new(),
                query: Some(query.to_string()),
            };
        }

        self.analyze(query)
    }
}

impl Default for BrandwatchLinter {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone)]
pub struct AnalysisResult {
    pub is_valid: bool,
    pub errors: Vec<LintError>,
    pub warnings: Vec<error::LintWarning>,
    pub query: Option<String>,
}

impl AnalysisResult {
    pub fn has_issues(&self) -> bool {
        !self.errors.is_empty() || !self.warnings.is_empty()
    }

    pub fn summary(&self) -> String {
        if self.is_valid && self.warnings.is_empty() {
            "Query is valid with no issues".to_string()
        } else {
            let error_count = self.errors.len();
            let warning_count = self.warnings.len();

            match (error_count, warning_count) {
                (0, 0) => "Query is valid with no issues".to_string(),
                (0, w) => format!(
                    "Query is valid with {} warning{}",
                    w,
                    if w == 1 { "" } else { "s" }
                ),
                (e, 0) => format!("Query has {} error{}", e, if e == 1 { "" } else { "s" }),
                (e, w) => format!(
                    "Query has {} error{} and {} warning{}",
                    e,
                    if e == 1 { "" } else { "s" },
                    w,
                    if w == 1 { "" } else { "s" }
                ),
            }
        }
    }

    pub fn format_issues(&self) -> String {
        let mut output = String::new();

        if !self.errors.is_empty() {
            output.push_str("Errors:\n");
            for (i, error) in self.errors.iter().enumerate() {
                output.push_str(&format!("  {}. {}\n", i + 1, error));
            }
        }

        if !self.warnings.is_empty() {
            if !output.is_empty() {
                output.push('\n');
            }
            output.push_str("Warnings:\n");
            for (i, warning) in self.warnings.iter().enumerate() {
                output.push_str(&format!("  {}. {:?}\n", i + 1, warning));
            }
        }

        output
    }
}

pub fn lint_query(query: &str) -> LintResult<LintReport> {
    let mut linter = BrandwatchLinter::new();
    linter.lint(query)
}

pub fn analyze_query(query: &str) -> AnalysisResult {
    let mut linter = BrandwatchLinter::new();
    linter.analyze(query)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_linting() {
        let mut linter = BrandwatchLinter::new();
        let report = linter.lint("apple AND juice").unwrap();
        assert!(!report.has_errors());
    }

    #[test]
    fn test_invalid_query() {
        let mut linter = BrandwatchLinter::new();
        let report = linter.lint("rating:6").unwrap();
        assert!(report.has_errors());
    }

    #[test]
    fn test_complex_query() {
        let query = r#"(apple OR orange) AND "fruit juice" NOT bitter"#;
        let mut linter = BrandwatchLinter::new();
        let report = linter.lint(query).unwrap();
        assert!(!report.has_errors());
    }

    #[test]
    fn test_field_query() {
        let query = r#"title:"apple juice" AND site:twitter.com"#;
        let mut linter = BrandwatchLinter::new();
        let report = linter.lint(query).unwrap();
        assert!(!report.has_errors());
    }

    #[test]
    fn test_proximity_query() {
        let mut linter = BrandwatchLinter::new();

        let query1 = r#"apple NEAR/3 juice"#;
        let report1 = linter.lint(query1).unwrap();
        assert!(!report1.has_errors());

        let query2 = r#""apple juice"~5"#;
        let report2 = linter.lint(query2).unwrap();
        assert!(!report2.has_errors());
    }

    #[test]
    fn test_analysis_result_summary() {
        let analysis = analyze_query("apple AND juice");
        assert_eq!(analysis.summary(), "Query is valid with no issues");

        let analysis = analyze_query("*invalid");
        assert!(analysis.summary().contains("error"));
    }
}
