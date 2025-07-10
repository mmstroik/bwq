use crate::ast::*;
use crate::error::{LintError, LintWarning};
use crate::validation::{ValidationContext, ValidationResult, ValidationRule};

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
                            return ValidationResult::with_error(LintError::OperatorMixingError {
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
                            return ValidationResult::with_error(LintError::OperatorMixingError {
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
                        if self.is_unparenthesized_near_and_mix(left, right_expr) {
                            return ValidationResult::with_error(LintError::ProximityOperatorError {
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
                            return ValidationResult::with_error(LintError::ProximityOperatorError {
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
                        return ValidationResult::with_error(LintError::ProximityOperatorError {
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

    fn is_unparenthesized_near_and_mix(&self, left: &Expression, right: &Expression) -> bool {
        // check if left side contains NEAR/x operators (not tilde) at top level
        self.contains_binary_near_at_top_level(left)
            || self.contains_binary_near_at_top_level(right)
    }

    fn contains_binary_near_at_top_level(&self, expr: &Expression) -> bool {
        match expr {
            Expression::Proximity { operator, .. } => {
                // allow tilde proximity (single operand) but reject NEAR/x (binary)
                matches!(
                    operator,
                    ProximityOperator::Near { .. } | ProximityOperator::NearForward { .. }
                )
            }
            Expression::Group { .. } => false,
            _ => false,
        }
    }
}

/// Pure negative query validation rule.
///
/// NOTE: This rule is designed for query-level validation, not expression-level validation.
/// It is registered in the ValidationEngine but its can_validate() always returns false
/// because pure negative validation requires analyzing the entire query structure.
/// The actual validation logic is in is_pure_negative_query() and is called from validator.rs.
pub struct PureNegativeRule;

impl ValidationRule for PureNegativeRule {
    fn name(&self) -> &'static str {
        "pure-negative"
    }

    fn validate(&self, _expr: &Expression, _ctx: &ValidationContext) -> ValidationResult {
        // handled at the query level in validator.rs, not per expression
        ValidationResult::new()
    }

    fn can_validate(&self, _expr: &Expression) -> bool {
        // only validate at the root query level in validator.rs
        false
    }
}

impl PureNegativeRule {
    #[allow(clippy::only_used_in_recursion)]
    pub fn is_pure_negative_query(&self, expr: &Expression) -> bool {
        match expr {
            // check if we're starting with a NOT operation at the top level
            Expression::BooleanOp {
                operator: BooleanOperator::Not,
                left,
                right,
                ..
            } => {
                // leading NOT with no right operand is pure negative
                if right.is_none() {
                    return true;
                }
                // binary NOT: if left side is pure negative, then the whole operation
                // is pure negative because the right side is being excluded, not included
                if let Some(_right_expr) = right {
                    return self.is_pure_negative_query(left);
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

pub struct BinaryOperatorRule;

impl ValidationRule for BinaryOperatorRule {
    fn name(&self) -> &'static str {
        "binary-operator"
    }

    fn validate(&self, expr: &Expression, _ctx: &ValidationContext) -> ValidationResult {
        if let Expression::BooleanOp {
            operator,
            left: _,
            right,
            span,
        } = expr
        {
            if matches!(operator, BooleanOperator::Not) {
                // for leading NOT, left contains the operand and right is None
                // for binary NOT, left and right both contain operands
                if right.is_none() {
                    // leading NOT case - left should contain the operand
                    return ValidationResult::new();
                }
            }

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

pub struct TildeUsageRule;

impl ValidationRule for TildeUsageRule {
    fn name(&self) -> &'static str {
        "tilde-usage"
    }

    fn validate(&self, expr: &Expression, _ctx: &ValidationContext) -> ValidationResult {
        if let Expression::Proximity {
            operator: ProximityOperator::Proximity { .. },
            terms,
            span,
        } = expr
        {
            if let Some(first_term) = terms.first() {
                match first_term {
                    Expression::Term {
                        term: Term::Word { .. },
                        ..
                    } => {
                        return ValidationResult::with_warning(LintWarning::PotentialTypo {
                            span: span.clone(),
                            suggestion: "Single term tilde may produce unexpected fuzzy matching results. Consider using quoted phrases for proximity: \"term1 term2\"~5".to_string(),
                        });
                    }
                    Expression::Term {
                        term: Term::Phrase { value },
                        ..
                    } => {
                        let words: Vec<&str> = value.split_whitespace().collect();
                        if words.len() == 1 {
                            return ValidationResult::with_warning(LintWarning::PotentialTypo {
                                span: span.clone(),
                                suggestion: "Tilde operator on single quoted words has no effect. Use unquoted word or multi-word phrase.".to_string(),
                            });
                        }
                    }
                    _ => {}
                }
            }
        }
        ValidationResult::new()
    }

    fn can_validate(&self, expr: &Expression) -> bool {
        matches!(
            expr,
            Expression::Proximity {
                operator: ProximityOperator::Proximity { .. },
                ..
            }
        )
    }
}
