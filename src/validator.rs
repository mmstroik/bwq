use crate::ast::*;
use crate::error::{LintError, LintReport, LintWarning, Span};

/// Validator for Brandwatch boolean queries
pub struct Validator {
    errors: Vec<LintError>,
    warnings: Vec<LintWarning>,
}

impl Validator {
    pub fn new() -> Self {
        Self {
            errors: Vec::new(),
            warnings: Vec::new(),
        }
    }

    pub fn validate(&mut self, query: &Query) -> LintReport {
        self.errors.clear();
        self.warnings.clear();
        
        // Check for pure negative queries (must have at least one positive term)
        if self.is_pure_negative_query(&query.expression) {
            self.errors.push(LintError::ValidationError {
                span: query.span.clone(),
                message: "Queries must contain at least one non-excluded term".to_string(),
            });
        }
        
        self.validate_expression(&query.expression);
        
        LintReport {
            errors: self.errors.clone(),
            warnings: self.warnings.clone(),
        }
    }

    fn validate_expression(&mut self, expr: &Expression) {
        match expr {
            Expression::BooleanOp { operator, left, right, span } => {
                self.validate_boolean_op(operator, left, right.as_deref(), span);
            }
            Expression::Group { expression, span } => {
                self.validate_expression(expression);
            }
            Expression::Proximity { operator, terms, span } => {
                self.validate_proximity_op(operator, terms, span);
            }
            Expression::Field { field, value, span } => {
                self.validate_field_op(field, value, span);
            }
            Expression::Range { field, start, end, span } => {
                self.validate_range(field.as_ref(), start, end, span);
            }
            Expression::Term { term, span } => {
                self.validate_term(term, span);
            }
            Expression::Comment { text, span } => {
                self.validate_comment(text, span);
            }
        }
    }

    fn validate_boolean_op(
        &mut self,
        operator: &BooleanOperator,
        left: &Expression,
        right: Option<&Expression>,
        span: &Span,
    ) {
        // Validate left operand
        self.validate_expression(left);

        // All operators (AND, OR, NOT) should have right operand in Brandwatch
        if let Some(right_expr) = right {
            self.validate_expression(right_expr);
        } else {
            self.errors.push(LintError::ValidationError {
                span: span.clone(),
                message: format!("{} operator requires two operands", operator.as_str()),
            });
        }

        // Check for performance warnings
        self.check_boolean_performance_warnings(operator, left, right, span);
    }

    fn validate_proximity_op(
        &mut self,
        operator: &ProximityOperator,
        terms: &[Expression],
        span: &Span,
    ) {
        // Validate term count based on operator type
        match operator {
            ProximityOperator::Proximity { .. } => {
                // Tilde operator can have 1 term (quoted phrase) or 2 terms
                if terms.is_empty() || terms.len() > 2 {
                    self.errors.push(LintError::InvalidProximityOperator {
                        span: span.clone(),
                        message: "Proximity operations require 1 or 2 terms".to_string(),
                    });
                    return;
                }
            }
            ProximityOperator::Near { .. } | ProximityOperator::NearForward { .. } => {
                // NEAR operators require exactly 2 terms
                if terms.len() != 2 {
                    self.errors.push(LintError::InvalidProximityOperator {
                        span: span.clone(),
                        message: "NEAR operations require exactly two terms".to_string(),
                    });
                    return;
                }
            }
        }

        // Validate each term
        for term in terms {
            self.validate_expression(term);
        }

        // Validate distance values
        match operator {
            ProximityOperator::Proximity { distance } => {
                if let Some(dist) = distance {
                    if *dist > 100 {
                        self.warnings.push(LintWarning::PerformanceWarning {
                            span: span.clone(),
                            message: "Large proximity distances may impact performance".to_string(),
                        });
                    }
                }
            }
            ProximityOperator::Near { distance } | ProximityOperator::NearForward { distance } => {
                if *distance > 100 {
                    self.warnings.push(LintWarning::PerformanceWarning {
                        span: span.clone(),
                        message: "Large NEAR distances may impact performance".to_string(),
                    });
                }
                if *distance == 0 {
                    self.errors.push(LintError::InvalidProximityOperator {
                        span: span.clone(),
                        message: "NEAR distance cannot be zero".to_string(),
                    });
                }
            }
        }
    }

    fn validate_field_op(&mut self, field: &FieldType, value: &Expression, span: &Span) {
        // Validate the field value
        self.validate_expression(value);

        // Field-specific validation
        match field {
            FieldType::AuthorFollowers => {
                self.validate_numeric_field(value, span, "authorFollowers");
            }
            FieldType::Rating => {
                self.validate_rating_field(value, span);
            }
            FieldType::Language => {
                self.validate_language_field(value, span);
            }
            FieldType::AuthorGender => {
                self.validate_gender_field(value, span);
            }
            FieldType::AuthorVerified => {
                self.validate_boolean_field(value, span, "authorVerified");
            }
            FieldType::AuthorVerifiedType => {
                self.validate_verified_type_field(value, span);
            }
            FieldType::EngagementType => {
                self.validate_engagement_type_field(value, span);
            }
            FieldType::MinuteOfDay => {
                self.validate_minute_of_day_field(value, span);
            }
            FieldType::Continent | FieldType::Country | FieldType::Region | FieldType::City => {
                self.validate_location_field(field, value, span);
            }
            FieldType::Latitude | FieldType::Longitude => {
                self.validate_coordinate_field(field, value, span);
            }
            _ => {
                // General field validation
                self.validate_general_field(field, value, span);
            }
        }
    }

    fn validate_range(
        &mut self,
        field: Option<&FieldType>,
        start: &str,
        end: &str,
        span: &Span,
    ) {
        // Try to parse as numbers if possible
        if let (Ok(start_num), Ok(end_num)) = (start.parse::<f64>(), end.parse::<f64>()) {
            if start_num > end_num {
                self.errors.push(LintError::ValidationError {
                    span: span.clone(),
                    message: "Range start value cannot be greater than end value".to_string(),
                });
            }
        }

        // Field-specific range validation
        if let Some(field_type) = field {
            match field_type {
                FieldType::Rating => {
                    self.validate_rating_range(start, end, span);
                }
                FieldType::AuthorFollowers => {
                    self.validate_followers_range(start, end, span);
                }
                FieldType::MinuteOfDay => {
                    self.validate_minute_range(start, end, span);
                }
                FieldType::Latitude => {
                    self.validate_latitude_range(start, end, span);
                }
                FieldType::Longitude => {
                    self.validate_longitude_range(start, end, span);
                }
                _ => {}
            }
        }
    }

    fn validate_term(&mut self, term: &Term, span: &Span) {
        match term {
            Term::Word { value } => {
                self.validate_word(value, span);
            }
            Term::Phrase { value } => {
                self.validate_phrase(value, span);
            }
            Term::Wildcard { value } => {
                self.validate_wildcard(value, span);
            }
            Term::Replacement { value } => {
                self.validate_replacement(value, span);
            }
            Term::CaseSensitive { value } => {
                self.validate_case_sensitive(value, span);
            }
            Term::Hashtag { value } => {
                self.validate_hashtag(value, span);
            }
            Term::Mention { value } => {
                self.validate_mention(value, span);
            }
            Term::Emoji { value } => {
                // Emojis are generally valid
            }
        }
    }

    fn validate_comment(&mut self, text: &str, span: &Span) {
        if text.trim().is_empty() {
            self.warnings.push(LintWarning::PerformanceWarning {
                span: span.clone(),
                message: "Empty comment".to_string(),
            });
        }
    }


    fn validate_wildcard(&mut self, value: &str, span: &Span) {
        // Wildcards cannot be at the beginning of a word
        if value.starts_with('*') {
            self.errors.push(LintError::InvalidWildcardPlacement {
                span: span.clone(),
            });
        }

        // Check for performance warnings with short wildcards
        let parts: Vec<&str> = value.split('*').collect();
        for part in parts {
            if !part.is_empty() && part.len() < 3 {
                self.warnings.push(LintWarning::PerformanceWarning {
                    span: span.clone(),
                    message: "Short wildcard terms may impact performance".to_string(),
                });
                break;
            }
        }
    }

    fn validate_replacement(&mut self, value: &str, span: &Span) {
        // Replacement character should be used for single character variations
        let question_count = value.chars().filter(|&c| c == '?').count();
        if question_count > 3 {
            self.warnings.push(LintWarning::PerformanceWarning {
                span: span.clone(),
                message: "Multiple replacement characters may impact performance".to_string(),
            });
        }
    }

    fn validate_case_sensitive(&mut self, value: &str, span: &Span) {
        // Check if the term actually has mixed case
        if value.chars().all(|c| c.is_lowercase()) || value.chars().all(|c| c.is_uppercase()) {
            self.warnings.push(LintWarning::PerformanceWarning {
                span: span.clone(),
                message: "Case-sensitive matching is unnecessary for single-case terms".to_string(),
            });
        }
    }

    fn validate_rating_range(&mut self, start: &str, end: &str, span: &Span) {
        if let (Ok(start_num), Ok(end_num)) = (start.parse::<i32>(), end.parse::<i32>()) {
            if start_num < 1 || start_num > 5 || end_num < 1 || end_num > 5 {
                self.errors.push(LintError::ValidationError {
                    span: span.clone(),
                    message: "Rating values must be between 1 and 5".to_string(),
                });
            }
        }
    }

    fn validate_latitude_range(&mut self, start: &str, end: &str, span: &Span) {
        if let (Ok(start_num), Ok(end_num)) = (start.parse::<f64>(), end.parse::<f64>()) {
            if start_num < -90.0 || start_num > 90.0 || end_num < -90.0 || end_num > 90.0 {
                self.errors.push(LintError::ValidationError {
                    span: span.clone(),
                    message: "Latitude values must be between -90 and 90".to_string(),
                });
            }
        }
    }

    fn validate_longitude_range(&mut self, start: &str, end: &str, span: &Span) {
        if let (Ok(start_num), Ok(end_num)) = (start.parse::<f64>(), end.parse::<f64>()) {
            if start_num < -180.0 || start_num > 180.0 || end_num < -180.0 || end_num > 180.0 {
                self.errors.push(LintError::ValidationError {
                    span: span.clone(),
                    message: "Longitude values must be between -180 and 180".to_string(),
                });
            }
        }
    }

    fn validate_language_field(&mut self, value: &Expression, span: &Span) {
        if let Expression::Term { term, .. } = value {
            if let Term::Word { value: lang_code } = term {
                // Basic validation for ISO 639-1 codes (2 characters)
                if lang_code.len() != 2 || !lang_code.chars().all(|c| c.is_ascii_lowercase()) {
                    self.warnings.push(LintWarning::PotentialTypo {
                        span: span.clone(),
                        suggestion: "Language codes should be 2-character ISO 639-1 codes (e.g., 'en', 'es')".to_string(),
                    });
                }
            }
        }
    }

    fn validate_gender_field(&mut self, value: &Expression, span: &Span) {
        if let Expression::Term { term, .. } = value {
            if let Term::Word { value: gender } = term {
                if !matches!(gender.as_str(), "F" | "M" | "f" | "m") {
                    self.errors.push(LintError::ValidationError {
                        span: span.clone(),
                        message: "Gender must be 'F' (Female) or 'M' (Male)".to_string(),
                    });
                }
            }
        }
    }

    fn validate_boolean_field(&mut self, value: &Expression, span: &Span, field_name: &str) {
        if let Expression::Term { term, .. } = value {
            if let Term::Word { value: bool_val } = term {
                if !matches!(bool_val.as_str(), "true" | "false") {
                    self.errors.push(LintError::ValidationError {
                        span: span.clone(),
                        message: format!("{} must be 'true' or 'false'", field_name),
                    });
                }
            }
        }
    }

    fn validate_verified_type_field(&mut self, value: &Expression, span: &Span) {
        if let Expression::Term { term, .. } = value {
            if let Term::Word { value: verified_type } = term {
                if !matches!(verified_type.as_str(), "blue" | "business" | "government") {
                    self.errors.push(LintError::ValidationError {
                        span: span.clone(),
                        message: "authorVerifiedType must be 'blue', 'business', or 'government'".to_string(),
                    });
                }
            }
        }
    }

    fn validate_engagement_type_field(&mut self, value: &Expression, span: &Span) {
        if let Expression::Term { term, .. } = value {
            if let Term::Word { value: engagement_type } = term {
                let valid_types = ["COMMENT", "REPLY", "RETWEET", "QUOTE"];
                if !valid_types.contains(&engagement_type.as_str()) {
                    self.errors.push(LintError::ValidationError {
                        span: span.clone(),
                        message: "engagementType must be 'COMMENT', 'REPLY', 'RETWEET', or 'QUOTE'".to_string(),
                    });
                }
            }
        }
    }

    fn validate_minute_of_day_field(&mut self, value: &Expression, span: &Span) {
        // Minutes of day should be 0-1439 (24 * 60 - 1)
        if let Expression::Range { start, end, .. } = value {
            if let (Ok(start_num), Ok(end_num)) = (start.parse::<i32>(), end.parse::<i32>()) {
                if start_num < 0 || start_num > 1439 || end_num < 0 || end_num > 1439 {
                    self.errors.push(LintError::ValidationError {
                        span: span.clone(),
                        message: "minuteOfDay values must be between 0 and 1439".to_string(),
                    });
                }
            }
        }
    }

    fn validate_numeric_field(&mut self, value: &Expression, span: &Span, field_name: &str) {
        match value {
            Expression::Term { term: Term::Word { value }, .. } if value.chars().all(|c| c.is_ascii_digit() || c == '.') => {
                // Valid numeric term
            }
            Expression::Range { start, end, .. } => {
                if start.parse::<f64>().is_err() || end.parse::<f64>().is_err() {
                    self.errors.push(LintError::ValidationError {
                        span: span.clone(),
                        message: format!("{} range values must be numeric", field_name),
                    });
                }
            }
            _ => {
                self.warnings.push(LintWarning::PotentialTypo {
                    span: span.clone(),
                    suggestion: format!("{} field typically expects numeric values", field_name),
                });
            }
        }
    }

    fn validate_rating_field(&mut self, value: &Expression, span: &Span) {
        if let Expression::Term { term: Term::Word { value: rating }, .. } = value {
            if let Ok(rating_num) = rating.parse::<i32>() {
                if rating_num < 1 || rating_num > 5 {
                    self.errors.push(LintError::ValidationError {
                        span: span.clone(),
                        message: "Rating must be between 1 and 5".to_string(),
                    });
                }
            }
        }
    }

    fn validate_location_field(&mut self, field: &FieldType, value: &Expression, span: &Span) {
        // Location codes have specific formats - this is a simplified validation
        if let Expression::Term { term: Term::Word { value: location_code }, .. } = value {
            match field {
                FieldType::Country => {
                    if location_code.len() != 3 {
                        self.warnings.push(LintWarning::PotentialTypo {
                            span: span.clone(),
                            suggestion: "Country codes should be 3-character ISO codes (e.g., 'usa', 'gbr')".to_string(),
                        });
                    }
                }
                FieldType::Region => {
                    if !location_code.contains('.') {
                        self.warnings.push(LintWarning::PotentialTypo {
                            span: span.clone(),
                            suggestion: "Region codes should include country prefix (e.g., 'usa.fl')".to_string(),
                        });
                    }
                }
                _ => {}
            }
        }
    }

    fn validate_coordinate_field(&mut self, field: &FieldType, value: &Expression, span: &Span) {
        if let Expression::Term { term: Term::Word { value: coord }, .. } = value {
            if let Ok(coord_num) = coord.parse::<f64>() {
                match field {
                    FieldType::Latitude => {
                        if coord_num < -90.0 || coord_num > 90.0 {
                            self.errors.push(LintError::ValidationError {
                                span: span.clone(),
                                message: "Latitude must be between -90 and 90".to_string(),
                            });
                        }
                    }
                    FieldType::Longitude => {
                        if coord_num < -180.0 || coord_num > 180.0 {
                            self.errors.push(LintError::ValidationError {
                                span: span.clone(),
                                message: "Longitude must be between -180 and 180".to_string(),
                            });
                        }
                    }
                    _ => {}
                }
            }
        }
    }

    fn validate_general_field(&mut self, field: &FieldType, value: &Expression, span: &Span) {
        // General field validation - check for common issues
        match value {
            Expression::Term { term: Term::Word { value }, .. } => {
                if value.trim().is_empty() {
                    self.errors.push(LintError::ValidationError {
                        span: span.clone(),
                        message: "Field value cannot be empty".to_string(),
                    });
                }
            }
            Expression::Term { term: Term::Phrase { value }, .. } => {
                if value.trim().is_empty() {
                    self.errors.push(LintError::ValidationError {
                        span: span.clone(),
                        message: "Field value cannot be empty".to_string(),
                    });
                }
            }
            _ => {}
        }
    }

    fn validate_word(&mut self, value: &str, span: &Span) {
        if value.trim().is_empty() {
            self.errors.push(LintError::ValidationError {
                span: span.clone(),
                message: "Word cannot be empty".to_string(),
            });
        }
        
        // Check for very short terms that might impact performance
        if value.len() == 1 {
            self.warnings.push(LintWarning::PerformanceWarning {
                span: span.clone(),
                message: "Single character terms may impact performance".to_string(),
            });
        }
    }

    fn validate_phrase(&mut self, value: &str, span: &Span) {
        if value.trim().is_empty() {
            self.errors.push(LintError::ValidationError {
                span: span.clone(),
                message: "Quoted phrase cannot be empty".to_string(),
            });
        }
    }

    fn validate_hashtag(&mut self, value: &str, span: &Span) {
        if value.trim().is_empty() {
            self.errors.push(LintError::ValidationError {
                span: span.clone(),
                message: "Hashtag cannot be empty".to_string(),
            });
        }
    }

    fn validate_mention(&mut self, value: &str, span: &Span) {
        if value.trim().is_empty() {
            self.errors.push(LintError::ValidationError {
                span: span.clone(),
                message: "Mention cannot be empty".to_string(),
            });
        }
    }

    fn validate_followers_range(&mut self, start: &str, end: &str, span: &Span) {
        if let (Ok(start_num), Ok(end_num)) = (start.parse::<i64>(), end.parse::<i64>()) {
            if start_num < 0 || end_num < 0 {
                self.errors.push(LintError::ValidationError {
                    span: span.clone(),
                    message: "Follower counts cannot be negative".to_string(),
                });
            }
            if end_num > 1_000_000_000 {
                self.warnings.push(LintWarning::PerformanceWarning {
                    span: span.clone(),
                    message: "Very large follower counts may not match any results".to_string(),
                });
            }
        }
    }

    fn validate_minute_range(&mut self, start: &str, end: &str, span: &Span) {
        if let (Ok(start_num), Ok(end_num)) = (start.parse::<i32>(), end.parse::<i32>()) {
            if start_num < 0 || start_num > 1439 || end_num < 0 || end_num > 1439 {
                self.errors.push(LintError::ValidationError {
                    span: span.clone(),
                    message: "minuteOfDay values must be between 0 and 1439".to_string(),
                });
            }
        }
    }

    fn check_boolean_performance_warnings(
        &mut self,
        operator: &BooleanOperator,
        left: &Expression,
        right: Option<&Expression>,
        span: &Span,
    ) {
        // Warn about very broad OR operations
        if matches!(operator, BooleanOperator::Or) {
            if let (Expression::Term { term: Term::Wildcard { .. }, .. }, Some(Expression::Term { term: Term::Wildcard { .. }, .. })) = (left, right) {
                self.warnings.push(LintWarning::PerformanceWarning {
                    span: span.clone(),
                    message: "Multiple wildcards in OR operations may significantly impact performance".to_string(),
                });
            }
        }
        
    }

    fn is_pure_negative_query(&self, expr: &Expression) -> bool {
        match expr {
            // For binary NOT, check if we're starting with a NOT operation at the top level
            Expression::BooleanOp { operator: BooleanOperator::Not, .. } => {
                // This is tricky with binary NOT - we need to check the entire structure
                // For now, let's be conservative and only flag standalone NOT terms
                false
            }
            Expression::BooleanOp { operator: BooleanOperator::And, left, right, .. } => {
                self.is_pure_negative_query(left) && right.as_ref().map_or(true, |r| self.is_pure_negative_query(r))
            }
            Expression::BooleanOp { operator: BooleanOperator::Or, left, right, .. } => {
                self.is_pure_negative_query(left) && right.as_ref().map_or(true, |r| self.is_pure_negative_query(r))
            }
            Expression::Group { expression, .. } => {
                self.is_pure_negative_query(expression)
            }
            _ => false,
        }
    }

    fn contains_near_operator(&self, expr: &Expression) -> bool {
        match expr {
            Expression::Proximity { .. } => true,
            Expression::BooleanOp { left, right, .. } => {
                self.contains_near_operator(left) || right.as_ref().map_or(false, |r| self.contains_near_operator(r))
            }
            Expression::Group { expression, .. } => {
                self.contains_near_operator(expression)
            }
            _ => false,
        }
    }

    fn is_direct_near_operator(&self, expr: &Expression) -> bool {
        matches!(expr, Expression::Proximity { .. })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::Lexer;
    use crate::parser::Parser;

    #[test]
    fn test_wildcard_validation() {
        // This test would need to be updated since "*invalid" now fails at parsing
        // Testing with a valid wildcard placement instead
        let mut lexer = Lexer::new("valid*");
        let tokens = lexer.tokenize().unwrap();
        let mut parser = Parser::new(tokens);
        let result = parser.parse().unwrap();
        
        let mut validator = Validator::new();
        let report = validator.validate(&result.query);
        
        assert!(!report.has_errors()); // valid* should be fine
    }

    #[test]
    fn test_rating_validation() {
        let mut lexer = Lexer::new("rating:6");
        let tokens = lexer.tokenize().unwrap();
        let mut parser = Parser::new(tokens);
        let result = parser.parse().unwrap();
        
        let mut validator = Validator::new();
        let report = validator.validate(&result.query);
        
        assert!(report.has_errors());
    }

    #[test]
    fn test_valid_query() {
        let mut lexer = Lexer::new("apple AND juice");
        let tokens = lexer.tokenize().unwrap();
        let mut parser = Parser::new(tokens);
        let result = parser.parse().unwrap();
        
        let mut validator = Validator::new();
        let report = validator.validate(&result.query);
        
        assert!(report.is_clean());
    }
}