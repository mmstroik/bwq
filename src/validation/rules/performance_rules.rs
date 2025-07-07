use crate::ast::*;
use crate::error::{LintError, LintWarning};
use crate::validation::{ValidationContext, ValidationResult, ValidationRule};

pub struct WildcardPerformanceRule;

impl ValidationRule for WildcardPerformanceRule {
    fn name(&self) -> &'static str {
        "wildcard-performance"
    }

    fn validate(&self, expr: &Expression, _ctx: &ValidationContext) -> ValidationResult {
        match expr {
            Expression::Term { term, span } => match term {
                Term::Wildcard { value } => {
                    let mut result = ValidationResult::new();

                    if value.starts_with('*') {
                        result
                            .errors
                            .push(LintError::InvalidWildcardPlacement { span: span.clone() });
                    }

                    let parts: Vec<&str> = value.split('*').collect();
                    if let Some(first_part) = parts.first() {
                        if !first_part.is_empty() {
                            if first_part.len() == 1 {
                                result.errors.push(LintError::ValidationError {
                                        span: span.clone(),
                                        message: "This wildcard matches too many unique terms. Please make it more specific.".to_string(),
                                    });
                            } else if first_part.len() == 2 {
                                result.warnings.push(LintWarning::PerformanceWarning {
                                    span: span.clone(),
                                    message: "Short wildcard terms may impact performance"
                                        .to_string(),
                                });
                            }
                        }
                    }

                    result
                }
                Term::Replacement { value } => {
                    let question_count = value.chars().filter(|&c| c == '?').count();
                    if question_count > 3 {
                        ValidationResult::with_warning(LintWarning::PerformanceWarning {
                            span: span.clone(),
                            message: "Multiple replacement characters may impact performance"
                                .to_string(),
                        })
                    } else {
                        ValidationResult::new()
                    }
                }
                _ => ValidationResult::new(),
            },
            Expression::BooleanOp {
                operator: BooleanOperator::Or,
                left,
                right,
                span,
            } => {
                // warn about multiple wildcards in OR operations
                if let (
                    Expression::Term {
                        term: Term::Wildcard { .. },
                        ..
                    },
                    Some(right_expr),
                ) = (left.as_ref(), right.as_ref())
                {
                    if let Expression::Term {
                        term: Term::Wildcard { .. },
                        ..
                    } = right_expr.as_ref()
                    {
                        return ValidationResult::with_warning(LintWarning::PerformanceWarning {
                            span: span.clone(),
                            message: "Multiple wildcards in OR operations may significantly impact performance".to_string(),
                        });
                    }
                }
                ValidationResult::new()
            }
            _ => ValidationResult::new(),
        }
    }

    fn can_validate(&self, expr: &Expression) -> bool {
        match expr {
            Expression::Term { term, .. } => {
                matches!(term, Term::Wildcard { .. } | Term::Replacement { .. })
            }
            Expression::BooleanOp {
                operator: BooleanOperator::Or,
                ..
            } => true,
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

                        if value.contains(':') {
                            let parts: Vec<&str> = value.split(':').collect();
                            if parts.len() == 2 {
                                let field_part = parts[0];
                                if !field_part.is_empty() && FieldType::parse(field_part).is_none()
                                {
                                    result.errors.push(LintError::ValidationError {
                                        span: span.clone(),
                                        message: format!("Unknown field type: {}", field_part),
                                    });
                                }
                            }
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
            Expression::Comment { text, span } => {
                if text.trim().is_empty() {
                    ValidationResult::with_warning(LintWarning::PerformanceWarning {
                        span: span.clone(),
                        message: "Empty comment".to_string(),
                    })
                } else {
                    ValidationResult::new()
                }
            }
            _ => ValidationResult::new(),
        }
    }

    fn can_validate(&self, expr: &Expression) -> bool {
        matches!(expr, Expression::Term { .. } | Expression::Comment { .. })
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
                    result.errors.push(LintError::ValidationError {
                        span: span.clone(),
                        message: "Follower counts cannot be negative".to_string(),
                    });
                }

                if end_num > 1_000_000_000 {
                    result.warnings.push(LintWarning::PerformanceWarning {
                        span: span.clone(),
                        message: "Very large follower counts may not match any results".to_string(),
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
