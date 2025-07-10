use anyhow::Result;
use lsp_types::{Diagnostic, DiagnosticSeverity, NumberOrString};

use crate::utils::span_to_range;
use bwq_linter::{
    BrandwatchLinter,
    error::{LintError, LintWarning},
};

pub struct DiagnosticsHandler;

impl DiagnosticsHandler {
    pub fn new() -> Self {
        Self
    }

    pub fn analyze_content(
        &self,
        content: &str,
        linter: &mut BrandwatchLinter,
    ) -> Result<Vec<Diagnostic>> {
        let mut diagnostics = Vec::new();

        let analysis = linter.analyze_and_skip_empty(content);

        for error in &analysis.errors {
            diagnostics.push(self.error_to_diagnostic(error));
        }

        for warning in &analysis.warnings {
            diagnostics.push(self.warning_to_diagnostic(warning));
        }

        Ok(diagnostics)
    }

    fn error_to_diagnostic(&self, error: &LintError) -> Diagnostic {
        let (range, message) = match error {
            LintError::LexerError { span, message } => (span_to_range(span), message.clone()),
            LintError::ParserError { span, message } => (span_to_range(span), message.clone()),
            LintError::ValidationError { span, message } => (span_to_range(span), message.clone()),
            LintError::InvalidBooleanCase { span, operator } => (
                span_to_range(span),
                format!("Boolean operator '{operator}' must be capitalized"),
            ),
            LintError::UnbalancedParentheses { span } => {
                (span_to_range(span), "Unbalanced parentheses".to_string())
            }
            LintError::InvalidWildcardPlacement { span } => (
                span_to_range(span),
                "Invalid wildcard placement: wildcards cannot be at the beginning of a word"
                    .to_string(),
            ),
            LintError::InvalidProximityOperator { span, message } => (
                span_to_range(span),
                format!("Invalid proximity operator syntax: {message}"),
            ),
            LintError::InvalidFieldOperator { span, message } => (
                span_to_range(span),
                format!("Invalid field operator syntax: {message}"),
            ),
            LintError::InvalidRangeSyntax { span } => (
                span_to_range(span),
                "Invalid range syntax: expected '[value TO value]'".to_string(),
            ),
            LintError::UnexpectedToken { span, token } => {
                (span_to_range(span), format!("Unexpected token '{token}'"))
            }
            LintError::ExpectedToken {
                span,
                expected,
                found,
            } => (
                span_to_range(span),
                format!("Expected '{expected}' but found '{found}'"),
            ),
            LintError::FieldValidationError { span, message } => {
                (span_to_range(span), message.clone())
            }
            LintError::ProximityOperatorError { span, message } => {
                (span_to_range(span), message.clone())
            }
            LintError::RangeValidationError { span, message } => {
                (span_to_range(span), message.clone())
            }
            LintError::OperatorMixingError { span, message } => {
                (span_to_range(span), message.clone())
            }
            LintError::PureNegativeQueryError { span, message } => {
                (span_to_range(span), message.clone())
            }
        };

        Diagnostic {
            range,
            severity: Some(DiagnosticSeverity::ERROR),
            code: Some(NumberOrString::String(error.code().to_string())),
            code_description: None,
            source: Some("bwq".to_string()),
            message,
            related_information: None,
            tags: None,
            data: None,
        }
    }

    fn warning_to_diagnostic(&self, warning: &LintWarning) -> Diagnostic {
        Diagnostic {
            range: span_to_range(warning.span()),
            severity: Some(DiagnosticSeverity::WARNING),
            code: Some(NumberOrString::String(warning.code().to_string())),
            code_description: None,
            source: Some("bwq".to_string()),
            message: format!("{warning}"),
            related_information: None,
            tags: None,
            data: None,
        }
    }
}

impl Default for DiagnosticsHandler {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bwq_linter::BrandwatchLinter;

    #[test]
    fn test_diagnostics_conversion() {
        let mut linter = BrandwatchLinter::new();
        let handler = DiagnosticsHandler::new();

        let content = "rating:6 AND *invalid";
        let diagnostics = handler.analyze_content(content, &mut linter).unwrap();

        assert!(
            !diagnostics.is_empty(),
            "Should have found some diagnostics"
        );

        let errors: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.severity == Some(lsp_types::DiagnosticSeverity::ERROR))
            .collect();
        assert!(!errors.is_empty(), "Should have found at least one error");

        println!("Found {} diagnostics", diagnostics.len());
        for diag in &diagnostics {
            println!("  {:?}: {}", diag.severity, diag.message);
        }
    }

    #[test]
    fn test_valid_query_diagnostics() {
        let mut linter = BrandwatchLinter::new();
        let handler = DiagnosticsHandler::new();

        let content = "apple AND juice";
        let diagnostics = handler.analyze_content(content, &mut linter).unwrap();

        let errors: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.severity == Some(lsp_types::DiagnosticSeverity::ERROR))
            .collect();
        assert!(errors.is_empty(), "Valid query should not have errors");
    }
}
