use crate::ast::*;
use crate::error::{LintError, LintWarning};
use crate::validation::{ValidationContext, ValidationResult, ValidationRule};

pub struct ShortTermRule;

impl ValidationRule for ShortTermRule {
    fn name(&self) -> &'static str {
        "short-term"
    }

    fn validate(&self, expr: &Expression, _ctx: &ValidationContext) -> ValidationResult {
        match expr {
            Expression::Term { term, span } => {
                match term {
                    Term::Word { value } => {
                        let mut result = ValidationResult::new();

                        // Check for empty terms
                        if value.trim().is_empty() {
                            result.errors.push(LintError::ValidationError {
                                span: span.clone(),
                                message: "Word cannot be empty".to_string(),
                            });
                        }

                        result
                    }
                    Term::Phrase { value } => {
                        if value.trim().is_empty() {
                            ValidationResult::with_error(LintError::ValidationError {
                                span: span.clone(),
                                message: "Quoted phrase cannot be empty".to_string(),
                            })
                        } else {
                            ValidationResult::new()
                        }
                    }
                    Term::Hashtag { value } => {
                        if value.trim().is_empty() {
                            ValidationResult::with_error(LintError::ValidationError {
                                span: span.clone(),
                                message: "Hashtag cannot be empty".to_string(),
                            })
                        } else if value.starts_with('*') || value.starts_with('?') {
                            ValidationResult::with_warning(LintWarning::PerformanceWarning {
                                span: span.clone(),
                                message: "Wildcard usage after '#' is discouraged and may lead to unexpected results".to_string(),
                            })
                        } else {
                            ValidationResult::new()
                        }
                    }
                    Term::Mention { value } => {
                        if value.trim().is_empty() {
                            ValidationResult::with_error(LintError::ValidationError {
                                span: span.clone(),
                                message: "Mention cannot be empty".to_string(),
                            })
                        } else if value.starts_with('*') || value.starts_with('?') {
                            ValidationResult::with_warning(LintWarning::PerformanceWarning {
                                span: span.clone(),
                                message: "Wildcard usage after '@' is discouraged and may lead to unexpected results".to_string(),
                            })
                        } else {
                            ValidationResult::new()
                        }
                    }
                    Term::CaseSensitive { .. } => ValidationResult::new(),
                    _ => ValidationResult::new(),
                }
            }
            _ => ValidationResult::new(),
        }
    }

    fn can_validate(&self, expr: &Expression) -> bool {
        matches!(expr, Expression::Term { .. })
    }
}
