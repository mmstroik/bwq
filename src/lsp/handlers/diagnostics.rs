use anyhow::Result;
use lsp_types::{Diagnostic, DiagnosticSeverity, NumberOrString, Position, Range};

use super::super::utils::{position_to_lsp, span_to_range};
use crate::{
    error::{LintError, LintWarning},
    BrandwatchLinter,
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

        let analysis = linter.analyze(content);

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
            LintError::LexerError { position, message } => {
                let lsp_pos = position_to_lsp(position);
                let range = Range {
                    start: lsp_pos,
                    end: Position {
                        line: lsp_pos.line,
                        character: lsp_pos.character + 1,
                    },
                };
                (range, message.clone())
            }
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
        };

        Diagnostic {
            range,
            severity: Some(DiagnosticSeverity::ERROR),
            code: Some(NumberOrString::String(self.error_code(error))),
            code_description: None,
            source: Some("bwq".to_string()),
            message,
            related_information: None,
            tags: None,
            data: None,
        }
    }

    fn warning_to_diagnostic(&self, warning: &LintWarning) -> Diagnostic {
        let (range, message) = match warning {
            LintWarning::PotentialTypo { span, suggestion } => (
                span_to_range(span),
                format!("Potential typo. Did you mean '{suggestion}'?"),
            ),
            LintWarning::DeprecatedOperator { span, replacement } => (
                span_to_range(span),
                format!("Deprecated operator. Consider using '{replacement}'"),
            ),
            LintWarning::PerformanceWarning { span, message } => (
                span_to_range(span),
                format!("Performance warning: {message}"),
            ),
        };

        Diagnostic {
            range,
            severity: Some(DiagnosticSeverity::WARNING),
            code: Some(NumberOrString::String(self.warning_code(warning))),
            code_description: None,
            source: Some("bwq".to_string()),
            message,
            related_information: None,
            tags: None,
            data: None,
        }
    }

    fn error_code(&self, error: &LintError) -> String {
        match error {
            LintError::LexerError { .. } => "E001".to_string(),
            LintError::ParserError { .. } => "E002".to_string(),
            LintError::ValidationError { .. } => "E003".to_string(),
            LintError::InvalidBooleanCase { .. } => "E004".to_string(),
            LintError::UnbalancedParentheses { .. } => "E005".to_string(),
            LintError::InvalidWildcardPlacement { .. } => "E006".to_string(),
            LintError::InvalidProximityOperator { .. } => "E007".to_string(),
            LintError::InvalidFieldOperator { .. } => "E008".to_string(),
            LintError::InvalidRangeSyntax { .. } => "E009".to_string(),
            LintError::UnexpectedToken { .. } => "E010".to_string(),
            LintError::ExpectedToken { .. } => "E011".to_string(),
        }
    }

    fn warning_code(&self, warning: &LintWarning) -> String {
        match warning {
            LintWarning::PotentialTypo { .. } => "W001".to_string(),
            LintWarning::DeprecatedOperator { .. } => "W002".to_string(),
            LintWarning::PerformanceWarning { .. } => "W003".to_string(),
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
    use crate::BrandwatchLinter;

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
