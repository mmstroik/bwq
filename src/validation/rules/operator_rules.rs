use crate::ast::*;
use crate::error::LintError;
use crate::validation::{ValidationContext, ValidationResult, ValidationRule};

// Mixed AND/OR validation rule
pub struct MixedAndOrRule;

impl ValidationRule for MixedAndOrRule {
    fn name(&self) -> &'static str {
        "mixed-and-or"
    }

    fn validate(&self, expr: &Expression, _ctx: &ValidationContext) -> ValidationResult {
        match expr {
            Expression::BooleanOp {
                operator,
                left,
                right,
                span,
            } => {
                if matches!(operator, BooleanOperator::And) {
                    if let Some(right_expr) = right {
                        if self.contains_or_at_top_level(right_expr)
                            || self.contains_or_at_top_level(left)
                        {
                            return ValidationResult::with_error(LintError::ValidationError {
                                span: span.clone(),
                                message: "The AND and OR operators cannot be mixed in the same sub-query. Please use parentheses to disambiguate - e.g. vanilla AND (icecream OR cake).".to_string(),
                            });
                        }
                    }
                } else if matches!(operator, BooleanOperator::Or) {
                    if let Some(right_expr) = right {
                        if self.contains_and_at_top_level(right_expr)
                            || self.contains_and_at_top_level(left)
                        {
                            return ValidationResult::with_error(LintError::ValidationError {
                                span: span.clone(),
                                message: "The AND and OR operators cannot be mixed in the same sub-query. Please use parentheses to disambiguate - e.g. vanilla AND (icecream OR cake).".to_string(),
                            });
                        }
                    }
                }
                ValidationResult::new()
            }
            _ => ValidationResult::new(),
        }
    }

    fn can_validate(&self, expr: &Expression) -> bool {
        matches!(
            expr,
            Expression::BooleanOp {
                operator: BooleanOperator::And | BooleanOperator::Or,
                ..
            }
        )
    }
}

impl MixedAndOrRule {
    fn contains_and_at_top_level(&self, expr: &Expression) -> bool {
        matches!(
            expr,
            Expression::BooleanOp {
                operator: BooleanOperator::And,
                ..
            }
        )
    }

    fn contains_or_at_top_level(&self, expr: &Expression) -> bool {
        matches!(
            expr,
            Expression::BooleanOp {
                operator: BooleanOperator::Or,
                ..
            }
        )
    }
}

// Mixed NEAR/boolean validation rule
pub struct MixedNearRule;

impl ValidationRule for MixedNearRule {
    fn name(&self) -> &'static str {
        "mixed-near"
    }

    fn validate(&self, expr: &Expression, ctx: &ValidationContext) -> ValidationResult {
        match expr {
            Expression::BooleanOp {
                operator,
                left,
                right,
                span,
            } => {
                if matches!(operator, BooleanOperator::And) {
                    if let Some(right_expr) = right {
                        if self.contains_near_at_top_level(right_expr)
                            || self.contains_near_at_top_level(left)
                        {
                            return ValidationResult::with_error(LintError::ValidationError {
                                span: span.clone(),
                                message: "The AND operator cannot be used within the NEAR operator. Either remove this operator or disambiguate with parenthesis, e.g. (vanilla NEAR/5 ice-cream) AND cake.".to_string(),
                            });
                        }
                    }
                }
                if !ctx.inside_group && matches!(operator, BooleanOperator::Or) {
                    if let Some(right_expr) = right {
                        if self.contains_near_at_top_level(right_expr)
                            || self.contains_near_at_top_level(left)
                        {
                            return ValidationResult::with_error(LintError::ValidationError {
                                span: span.clone(),
                                message: "Please use parentheses for disambiguation when using the OR or NEAR operators with another NEAR operator - e.g. (vanilla OR chocolate) NEAR/5 (ice-cream NEAR/5 cake).".to_string(),
                            });
                        }
                    }
                }
                ValidationResult::new()
            }
            Expression::Proximity { terms, span, .. } => {
                for term in terms {
                    if self.contains_or_at_top_level(term) || self.contains_and_at_top_level(term) {
                        return ValidationResult::with_error(LintError::ValidationError {
                            span: span.clone(),
                            message: "Please use parentheses for disambiguation when using the OR or NEAR operators with another NEAR operator - e.g. (vanilla OR chocolate) NEAR/5 (ice-cream NEAR/5 cake).".to_string(),
                        });
                    }
                }
                ValidationResult::new()
            }
            _ => ValidationResult::new(),
        }
    }

    fn can_validate(&self, expr: &Expression) -> bool {
        matches!(
            expr,
            Expression::BooleanOp {
                operator: BooleanOperator::And | BooleanOperator::Or,
                ..
            } | Expression::Proximity { .. }
        )
    }
}

impl MixedNearRule {
    fn contains_and_at_top_level(&self, expr: &Expression) -> bool {
        matches!(
            expr,
            Expression::BooleanOp {
                operator: BooleanOperator::And,
                ..
            }
        )
    }

    fn contains_or_at_top_level(&self, expr: &Expression) -> bool {
        matches!(
            expr,
            Expression::BooleanOp {
                operator: BooleanOperator::Or,
                ..
            }
        )
    }

    fn contains_near_at_top_level(&self, expr: &Expression) -> bool {
        match expr {
            Expression::Proximity { .. } => true,
            Expression::Group { .. } => false,
            _ => false,
        }
    }
}

// Pure negative query validation rule
pub struct PureNegativeRule;

impl ValidationRule for PureNegativeRule {
    fn name(&self) -> &'static str {
        "pure-negative"
    }

    fn validate(&self, _expr: &Expression, _ctx: &ValidationContext) -> ValidationResult {
        // This will be handled at the query level in the engine, not per expression
        ValidationResult::new()
    }

    fn can_validate(&self, _expr: &Expression) -> bool {
        // Only validate at the root query level
        false
    }
}

impl PureNegativeRule {
    #[allow(clippy::only_used_in_recursion)]
    pub fn is_pure_negative_query(&self, expr: &Expression) -> bool {
        match expr {
            // For binary NOT, check if we're starting with a NOT operation at the top level
            Expression::BooleanOp {
                operator: BooleanOperator::Not,
                left,
                right: _,
                ..
            } => {
                // Check if this is a leading NOT (dummy left operand)
                if let Expression::Term {
                    term: Term::Word { value },
                    ..
                } = left.as_ref()
                {
                    if value.is_empty() {
                        // This is a leading NOT - ANY leading NOT is pure negative
                        // according to Brandwatch API behavior
                        return true;
                    }
                }
                false
            }
            Expression::BooleanOp {
                operator: BooleanOperator::And,
                left,
                right,
                ..
            } => {
                self.is_pure_negative_query(left)
                    && right
                        .as_ref()
                        .is_none_or(|r| self.is_pure_negative_query(r))
            }
            Expression::BooleanOp {
                operator: BooleanOperator::Or,
                left,
                right,
                ..
            } => {
                self.is_pure_negative_query(left)
                    && right
                        .as_ref()
                        .is_none_or(|r| self.is_pure_negative_query(r))
            }
            Expression::Group { expression, .. } => self.is_pure_negative_query(expression),
            _ => false,
        }
    }
}

// Binary operator validation rule
pub struct BinaryOperatorRule;

impl ValidationRule for BinaryOperatorRule {
    fn name(&self) -> &'static str {
        "binary-operator"
    }

    fn validate(&self, expr: &Expression, _ctx: &ValidationContext) -> ValidationResult {
        if let Expression::BooleanOp {
            operator,
            left,
            right,
            span,
        } = expr
        {
            // Check for NOT operator with empty left operand (this is valid binary NOT)
            if matches!(operator, BooleanOperator::Not) {
                if let Expression::Term {
                    term: Term::Word { value },
                    ..
                } = left.as_ref()
                {
                    if value.is_empty() {
                        if right.is_none() {
                            return ValidationResult::with_error(LintError::ValidationError {
                                span: span.clone(),
                                message: "NOT operator requires an operand".to_string(),
                            });
                        }
                        return ValidationResult::new();
                    }
                }
            }

            // For non-NOT operators, ensure we have both operands
            if right.is_none() && !matches!(operator, BooleanOperator::Not) {
                return ValidationResult::with_error(LintError::ValidationError {
                    span: span.clone(),
                    message: format!("{} operator requires two operands", operator.as_str()),
                });
            }
        }
        ValidationResult::new()
    }

    fn can_validate(&self, expr: &Expression) -> bool {
        matches!(expr, Expression::BooleanOp { .. })
    }
}
