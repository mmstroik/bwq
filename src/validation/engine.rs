use super::rules::*;
use super::{ValidationContext, ValidationRule};
use crate::ast::*;
use crate::error::{LintError, LintReport, LintWarning};

pub struct ValidationEngine {
    rules: Vec<Box<dyn ValidationRule>>,
}

impl ValidationEngine {
    pub fn new() -> Self {
        Self {
            rules: vec![
                // Field validation rules
                Box::new(RatingFieldRule),
                Box::new(CoordinateFieldRule),
                Box::new(LanguageFieldRule),
                Box::new(AuthorGenderFieldRule),
                Box::new(BooleanFieldRule),
                Box::new(EngagementTypeFieldRule),
                Box::new(VerifiedTypeFieldRule),
                Box::new(MinuteOfDayFieldRule),
                Box::new(RangeFieldRule),
                // Operator validation rules
                Box::new(MixedAndOrRule),
                Box::new(MixedNearRule),
                Box::new(PureNegativeRule),
                Box::new(BinaryOperatorRule),
                // Performance validation rules
                Box::new(WildcardPerformanceRule),
                Box::new(ShortTermRule),
                Box::new(RangePerformanceRule),
            ],
        }
    }

    pub fn validate(&self, query: &Query) -> LintReport {
        let mut all_errors = Vec::new();
        let mut all_warnings = Vec::new();

        let ctx = ValidationContext::default();
        self.walk_expression(&query.expression, &ctx, &mut all_errors, &mut all_warnings);

        LintReport {
            errors: all_errors,
            warnings: all_warnings,
        }
    }

    fn walk_expression(
        &self,
        expr: &Expression,
        ctx: &ValidationContext,
        errors: &mut Vec<LintError>,
        warnings: &mut Vec<LintWarning>,
    ) {
        // Apply all relevant rules to this expression
        for rule in &self.rules {
            if rule.can_validate(expr) {
                let result = rule.validate(expr, ctx);
                errors.extend(result.errors);
                warnings.extend(result.warnings);
            }
        }

        // Recursively validate child expressions with updated context
        match expr {
            Expression::BooleanOp {
                operator,
                left,
                right,
                ..
            } => {
                let mut child_ctx = ctx.clone();
                child_ctx.parent_operator = Some(operator.clone());

                self.walk_expression(left, &child_ctx, errors, warnings);
                if let Some(right_expr) = right {
                    self.walk_expression(right_expr, &child_ctx, errors, warnings);
                }
            }
            Expression::Group { expression, .. } => {
                let mut group_ctx = ctx.clone();
                group_ctx.inside_group = true;
                self.walk_expression(expression, &group_ctx, errors, warnings);
            }
            Expression::Proximity { terms, .. } => {
                for term in terms {
                    self.walk_expression(term, ctx, errors, warnings);
                }
            }
            Expression::Field { field, value, .. } => {
                let mut field_ctx = ctx.clone();
                field_ctx.field_context = Some(field.clone());
                self.walk_expression(value, &field_ctx, errors, warnings);
            }
            Expression::Range { .. } | Expression::Term { .. } | Expression::Comment { .. } => {
                // Terminal nodes - no recursion needed
            }
        }
    }
}

impl Default for ValidationEngine {
    fn default() -> Self {
        Self::new()
    }
}
