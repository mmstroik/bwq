use crate::ast::*;
use crate::error::{LintError, LintReport};
use crate::validation::{rules::PureNegativeRule, ValidationEngine};

/// plugin-based query-level validator
pub struct Validator {
    engine: ValidationEngine,
    pure_negative_rule: PureNegativeRule,
}

impl Validator {
    pub fn new() -> Self {
        Self {
            engine: ValidationEngine::new(),
            pure_negative_rule: PureNegativeRule,
        }
    }

    pub fn validate(&mut self, query: &Query) -> LintReport {
        let mut report = self.engine.validate(query);

        if self
            .pure_negative_rule
            .is_pure_negative_query(&query.expression)
        {
            report.errors.push(LintError::PureNegativeQueryError {
                span: query.span.clone(),
                message: "Queries must contain at least one non-excluded term".to_string(),
            });
        }

        report
    }
}

impl Default for Validator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::Lexer;
    use crate::parser::Parser;

    #[test]
    fn test_wildcard_validation() {
        let mut lexer = Lexer::new("valid*");
        let tokens = lexer.tokenize().unwrap();
        let mut parser = Parser::new(tokens).unwrap();
        let result = parser.parse().unwrap();

        let mut validator = Validator::new();
        let report = validator.validate(&result.query);

        assert!(!report.has_errors());
    }

    #[test]
    fn test_rating_validation() {
        let mut lexer = Lexer::new("rating:6");
        let tokens = lexer.tokenize().unwrap();
        let mut parser = Parser::new(tokens).unwrap();
        let result = parser.parse().unwrap();

        let mut validator = Validator::new();
        let report = validator.validate(&result.query);

        assert!(report.has_errors());
    }

    #[test]
    fn test_valid_query() {
        let mut lexer = Lexer::new("apple AND juice");
        let tokens = lexer.tokenize().unwrap();
        let mut parser = Parser::new(tokens).unwrap();
        let result = parser.parse().unwrap();

        let mut validator = Validator::new();
        let report = validator.validate(&result.query);

        assert!(report.is_clean());
    }

    #[test]
    fn test_mixed_and_or_validation() {
        let mut lexer = Lexer::new("apple AND banana OR juice");
        let tokens = lexer.tokenize().unwrap();
        let mut parser = Parser::new(tokens).unwrap();
        let result = parser.parse().unwrap();

        let mut validator = Validator::new();
        let report = validator.validate(&result.query);

        assert!(report.has_errors());
        assert!(report.errors.iter().any(|e| e.code() == "E015"));
    }

    #[test]
    fn test_pure_negative_query() {
        let mut lexer = Lexer::new("NOT bitter");
        let tokens = lexer.tokenize().unwrap();
        let mut parser = Parser::new(tokens).unwrap();
        let result = parser.parse().unwrap();

        let mut validator = Validator::new();
        let report = validator.validate(&result.query);

        assert!(report.has_errors());
        assert!(report.errors.iter().any(|e| e.code() == "E016"));
    }

    #[test]
    fn test_coordinate_validation() {
        let mut lexer = Lexer::new("latitude:100");
        let tokens = lexer.tokenize().unwrap();
        let mut parser = Parser::new(tokens).unwrap();
        let result = parser.parse().unwrap();

        let mut validator = Validator::new();
        let report = validator.validate(&result.query);

        assert!(report.has_errors());
        assert!(report.errors.iter().any(|e| e.code() == "E012"));
    }

    #[test]
    fn test_boolean_field_validation() {
        let mut lexer = Lexer::new("authorVerified:yes");
        let tokens = lexer.tokenize().unwrap();
        let mut parser = Parser::new(tokens).unwrap();
        let result = parser.parse().unwrap();

        let mut validator = Validator::new();
        let report = validator.validate(&result.query);

        assert!(report.has_errors());
        assert!(report.errors.iter().any(|e| e.code() == "E012"));
    }

    #[test]
    fn test_wildcard_placement_validation() {
        let mut lexer = Lexer::new("*invalid");
        let tokens = lexer.tokenize().unwrap();
        let mut parser = Parser::new(tokens).unwrap();
        let result = parser.parse().unwrap();

        let mut validator = Validator::new();
        let report = validator.validate(&result.query);

        assert!(report.has_errors());
        assert!(report.errors.iter().any(|e| e.code() == "E006"));
    }

    #[test]
    fn test_performance_warnings() {
        let mut lexer = Lexer::new("ab*");
        let tokens = lexer.tokenize().unwrap();
        let mut parser = Parser::new(tokens).unwrap();
        let result = parser.parse().unwrap();

        let mut validator = Validator::new();
        let report = validator.validate(&result.query);

        assert!(!report.warnings.is_empty());
        assert!(report.warnings.iter().any(|w| w.code() == "W003"));
    }
}
