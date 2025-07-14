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
        let tokens = lexer.tokenize()?;

        let mut parser = Parser::new(tokens)?;
        let parse_result = parser.parse()?;

        let mut report = self.validator.validate(&parse_result.query);
        report.warnings.extend(parse_result.warnings);

        Ok(report)
    }

    pub fn analyze(&mut self, query: &str) -> AnalysisResult {
        match self.lint(query) {
            Ok(report) => AnalysisResult {
                is_valid: !report.has_errors(),
                errors: report.errors,
                warnings: report.warnings,
                query: query.to_string(),
            },
            Err(error) => AnalysisResult {
                is_valid: false,
                errors: vec![error],
                warnings: vec![],
                query: query.to_string(),
            },
        }
    }

    pub fn analyze_and_skip_empty(&mut self, query: &str) -> AnalysisResult {
        if query.trim().is_empty() {
            return AnalysisResult {
                is_valid: true,
                errors: Vec::new(),
                warnings: Vec::new(),
                query: query.to_string(),
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
    pub query: String,
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
}
