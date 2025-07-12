use crate::ast::*;
use crate::error::{LintError, LintWarning};
use crate::validation::{ValidationContext, ValidationResult, ValidationRule};

pub struct WildcardPlacementRule;

impl ValidationRule for WildcardPlacementRule {
    fn name(&self) -> &'static str {
        "wildcard-placement"
    }

    fn validate(&self, expr: &Expression, _ctx: &ValidationContext) -> ValidationResult {
        match expr {
            Expression::Term {
                term: Term::Wildcard { value },
                span,
            } => {
                let mut result = ValidationResult::new();

                if value.starts_with('*') || value.starts_with('?') {
                    result
                        .errors
                        .push(LintError::InvalidWildcardPlacement { span: span.clone(), message: "Wildcard operators (* and ?) cannot be used at the start of a search term. They're used within or at the end of a word to find any possible match.".to_string() });
                }

                let parts: Vec<&str> = value.split('*').collect();
                if let Some(first_part) = parts.first() {
                    if !first_part.is_empty() && first_part.len() == 1 && value.ends_with('*') {
                        result.errors.push(LintError::InvalidWildcardPlacement {
                                        span: span.clone(),
                                        message: "This wildcard matches too many unique terms. Use at least two letters with the wildcard. For example, d*g matches terms like dog, dig, and Doug.".to_string(),
                                    });
                    }
                }

                result
            }
            _ => ValidationResult::new(),
        }
    }

    fn can_validate(&self, expr: &Expression) -> bool {
        match expr {
            Expression::Term { term, .. } => {
                matches!(term, Term::Wildcard { .. })
            }

            _ => false,
        }
    }
}

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

pub struct RangePerformanceRule;

impl ValidationRule for RangePerformanceRule {
    fn name(&self) -> &'static str {
        "range-performance"
    }

    fn validate(&self, expr: &Expression, _ctx: &ValidationContext) -> ValidationResult {
        if let Expression::Range {
            field: Some(FieldType::AuthorFollowers),
            start,
            end,
            span,
        } = expr
        {
            if let (Ok(start_num), Ok(end_num)) = (start.parse::<i64>(), end.parse::<i64>()) {
                let mut result = ValidationResult::new();

                if start_num < 0 || end_num < 0 {
                    result.errors.push(LintError::RangeValidationError {
                        span: span.clone(),
                        message: "Follower counts cannot be negative".to_string(),
                    });
                }

                if end_num.to_string().len() > 10 {
                    result.errors.push(LintError::RangeValidationError {
                        span: span.clone(),
                        message: "Follower counts cannot exceed 10 digits".to_string(),
                    });
                }

                result
            } else {
                ValidationResult::new()
            }
        } else {
            ValidationResult::new()
        }
    }

    fn can_validate(&self, expr: &Expression) -> bool {
        matches!(
            expr,
            Expression::Range {
                field: Some(FieldType::AuthorFollowers),
                ..
            }
        )
    }
}
