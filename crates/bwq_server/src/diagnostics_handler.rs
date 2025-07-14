use anyhow::Result;
use lsp_types::{Diagnostic, DiagnosticSeverity, NumberOrString};

use crate::utils::span_to_range;
use bwq_linter::{
    BrandwatchLinter,
    ast::Query,
    error::{LintError, LintWarning},
};

pub struct DiagnosticsHandler;

impl DiagnosticsHandler {
    pub fn new() -> Self {
        Self
    }

    pub fn analyze_content_with_ast(
        &self,
        content: &str,
        linter: &mut BrandwatchLinter,
    ) -> Result<(Vec<Diagnostic>, Option<Query>)> {
        let mut diagnostics = Vec::new();

        let analysis = linter.analyze_for_server(content);

        for error in &analysis.errors {
            diagnostics.push(self.error_to_diagnostic(error));
        }

        for warning in &analysis.warnings {
            diagnostics.push(self.warning_to_diagnostic(warning));
        }

        Ok((diagnostics, analysis.ast))
    }

    fn error_to_diagnostic(&self, error: &LintError) -> Diagnostic {
        Diagnostic {
            range: span_to_range(error.span()),
            severity: Some(DiagnosticSeverity::ERROR),
            code: Some(NumberOrString::String(error.code().to_string())),
            code_description: None,
            source: Some("bwq".to_string()),
            message: error.to_string(),
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
        let (diagnostics, _) = handler
            .analyze_content_with_ast(content, &mut linter)
            .unwrap();

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
        let (diagnostics, _) = handler
            .analyze_content_with_ast(content, &mut linter)
            .unwrap();

        let errors: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.severity == Some(lsp_types::DiagnosticSeverity::ERROR))
            .collect();
        assert!(errors.is_empty(), "Valid query should not have errors");
    }
}
