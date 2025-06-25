use crate::ast::*;
use crate::error::{LintError, LintWarning};

pub mod rules;
pub mod engine;

pub use engine::ValidationEngine;

#[derive(Debug, Clone)]
pub struct ValidationContext {
    pub inside_group: bool,
    pub parent_operator: Option<BooleanOperator>,
    pub field_context: Option<FieldType>,
}

impl Default for ValidationContext {
    fn default() -> Self {
        Self {
            inside_group: false,
            parent_operator: None,
            field_context: None,
        }
    }
}

pub trait ValidationRule {
    fn name(&self) -> &'static str;
    fn validate(&self, expr: &Expression, ctx: &ValidationContext) -> ValidationResult;
    fn can_validate(&self, expr: &Expression) -> bool;
}

#[derive(Debug, Default)]
pub struct ValidationResult {
    pub errors: Vec<LintError>,
    pub warnings: Vec<LintWarning>,
}

impl ValidationResult {
    pub fn new() -> Self {
        Self::default()
    }
    
    pub fn with_error(error: LintError) -> Self {
        Self {
            errors: vec![error],
            warnings: vec![],
        }
    }
    
    pub fn with_warning(warning: LintWarning) -> Self {
        Self {
            errors: vec![],
            warnings: vec![warning],
        }
    }
    
    pub fn extend(&mut self, other: ValidationResult) {
        self.errors.extend(other.errors);
        self.warnings.extend(other.warnings);
    }
}