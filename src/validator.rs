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
        self.validate_expression_with_context(expr, false);
    }
    
    fn validate_expression_with_context(&mut self, expr: &Expression, inside_group: bool) {
        self.validate_operator_interactions(expr, inside_group);
        
        match expr {
            Expression::BooleanOp { operator, left, right, span } => {
                self.validate_boolean_op(operator, left, right.as_deref(), span, inside_group);
            }
            Expression::Group { expression, span } => {
                self.validate_expression_with_context(expression, true);
            }
            Expression::Proximity { operator, terms, span } => {
                self.validate_proximity_op(operator, terms, span, inside_group);
            }
            Expression::Field { field, value, span } => {
                self.validate_field_op(field, value, span, inside_group);
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
        inside_group: bool,
    ) {
        if matches!(operator, BooleanOperator::Not) {
            if let Expression::Term { term: Term::Word { value }, .. } = left {
                if value.is_empty() {
                    if let Some(right_expr) = right {
                        self.validate_expression_with_context(right_expr, inside_group);
                    } else {
                        self.errors.push(LintError::ValidationError {
                            span: span.clone(),
                            message: "NOT operator requires an operand".to_string(),
                        });
                    }
                    return;
                }
            }
        }

        self.validate_expression_with_context(left, inside_group);

        if let Some(right_expr) = right {
            self.validate_expression_with_context(right_expr, inside_group);
        } else {
            self.errors.push(LintError::ValidationError {
                span: span.clone(),
                message: format!("{} operator requires two operands", operator.as_str()),
            });
        }

        self.check_boolean_performance_warnings(operator, left, right, span);
    }

    fn validate_proximity_op(
        &mut self,
        operator: &ProximityOperator,
        terms: &[Expression],
        span: &Span,
        inside_group: bool,
    ) {
        match operator {
            ProximityOperator::Proximity { .. } => {
                if terms.is_empty() || terms.len() > 2 {
                    self.errors.push(LintError::InvalidProximityOperator {
                        span: span.clone(),
                        message: "Proximity operations require 1 or 2 terms".to_string(),
                    });
                    return;
                }
            }
            ProximityOperator::Near { .. } | ProximityOperator::NearForward { .. } => {
                if terms.len() != 2 {
                    self.errors.push(LintError::InvalidProximityOperator {
                        span: span.clone(),
                        message: "NEAR operations require exactly two terms".to_string(),
                    });
                    return;
                }
            }
        }

        for term in terms {
            self.validate_expression_with_context(term, inside_group);
        }

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

    fn validate_field_op(&mut self, field: &FieldType, value: &Expression, span: &Span, inside_group: bool) {
        self.validate_expression_with_context(value, inside_group);

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
        if let (Ok(start_num), Ok(end_num)) = (start.parse::<f64>(), end.parse::<f64>()) {
            if start_num > end_num {
                self.errors.push(LintError::ValidationError {
                    span: span.clone(),
                    message: "Range start value cannot be greater than end value".to_string(),
                });
            }
        }

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
        if value.starts_with('*') {
            self.errors.push(LintError::InvalidWildcardPlacement {
                span: span.clone(),
            });
        }
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
        let question_count = value.chars().filter(|&c| c == '?').count();
        if question_count > 3 {
            self.warnings.push(LintWarning::PerformanceWarning {
                span: span.clone(),
                message: "Multiple replacement characters may impact performance".to_string(),
            });
        }
    }

    fn validate_case_sensitive(&mut self, value: &str, span: &Span) {
        if value.chars().all(|c| c.is_lowercase()) || value.chars().all(|c| c.is_uppercase()) {
            self.warnings.push(LintWarning::PerformanceWarning {
                span: span.clone(),
                message: "Case-sensitive matching is unnecessary for single-case terms".to_string(),
            });
        }
    }

    fn validate_rating_range(&mut self, start: &str, end: &str, span: &Span) {
        if let (Ok(start_num), Ok(end_num)) = (start.parse::<i32>(), end.parse::<i32>()) {
            if start_num < 0 || start_num > 5 || end_num < 0 || end_num > 5 {
                self.errors.push(LintError::ValidationError {
                    span: span.clone(),
                    message: "Rating values must be between 0 and 5".to_string(),
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
                if !matches!(gender.as_str(), "F" | "M" | "f" | "m" | "X" | "x" | "U" | "u") {
                    self.warnings.push(LintWarning::PotentialTypo {
                        span: span.clone(),
                        suggestion: "Common gender values are 'F', 'M', 'X', or 'U'".to_string(),
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
                let common_types = ["COMMENT", "REPLY", "RETWEET", "QUOTE", "LIKE", "SHARE", "MENTION"];
                if !common_types.contains(&engagement_type.as_str()) {
                    self.warnings.push(LintWarning::PotentialTypo {
                        span: span.clone(),
                        suggestion: "Common engagement types are 'COMMENT', 'REPLY', 'RETWEET', 'QUOTE', 'LIKE'".to_string(),
                    });
                }
            }
        }
    }

    fn validate_minute_of_day_field(&mut self, value: &Expression, span: &Span) {
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
                if rating_num < 0 || rating_num > 5 {
                    self.errors.push(LintError::ValidationError {
                        span: span.clone(),
                        message: "Rating must be between 0 and 5".to_string(),
                    });
                }
            }
        }
    }

    fn validate_location_field(&mut self, field: &FieldType, value: &Expression, span: &Span) {
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
        if value.contains(':') {
            let parts: Vec<&str> = value.split(':').collect();
            if parts.len() == 2 {
                let field_part = parts[0];
                if !field_part.is_empty() && FieldType::from_str(field_part).is_none() {
                    self.errors.push(LintError::ValidationError {
                        span: span.clone(),
                        message: format!("Unknown field type: {}", field_part),
                    });
                    return;
                }
            }
        }
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
            Expression::BooleanOp { operator: BooleanOperator::Not, left, right, .. } => {
                // Check if this is a leading NOT (dummy left operand)
                if let Expression::Term { term: Term::Word { value }, .. } = left.as_ref() {
                    if value.is_empty() {
                        // This is a leading NOT - ANY leading NOT is pure negative
                        // according to Brandwatch API behavior
                        return true;
                    }
                }
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


    fn validate_operator_interactions(&mut self, expr: &Expression, inside_group: bool) {
        match expr {
            Expression::BooleanOp { operator, left, right, span } => {
                if matches!(operator, BooleanOperator::And) {
                    if let Some(right_expr) = right {
                        if self.contains_or_at_top_level(right_expr) || self.contains_or_at_top_level(left) {
                            self.errors.push(LintError::ValidationError {
                                span: span.clone(),
                                message: "The AND and OR operators cannot be mixed in the same sub-query. Please use parentheses to disambiguate - e.g. vanilla AND (icecream OR cake).".to_string(),
                            });
                        }
                    }
                } else if matches!(operator, BooleanOperator::Or) {
                    if let Some(right_expr) = right {
                        if self.contains_and_at_top_level(right_expr) || self.contains_and_at_top_level(left) {
                            self.errors.push(LintError::ValidationError {
                                span: span.clone(),
                                message: "The AND and OR operators cannot be mixed in the same sub-query. Please use parentheses to disambiguate - e.g. vanilla AND (icecream OR cake).".to_string(),
                            });
                        }
                    }
                }
                if matches!(operator, BooleanOperator::And) {
                    if let Some(right_expr) = right {
                        if self.contains_near_at_top_level(right_expr) || self.contains_near_at_top_level(left) {
                            self.errors.push(LintError::ValidationError {
                                span: span.clone(),
                                message: "The AND operator cannot be used within the NEAR operator. Either remove this operator or disambiguate with parenthesis, e.g. (vanilla NEAR/5 ice-cream) AND cake.".to_string(),
                            });
                        }
                    }
                }
                if !inside_group && matches!(operator, BooleanOperator::Or) {
                    if let Some(right_expr) = right {
                        if self.contains_near_at_top_level(right_expr) || self.contains_near_at_top_level(left) {
                            self.errors.push(LintError::ValidationError {
                                span: span.clone(),
                                message: "Please use parentheses for disambiguation when using the OR or NEAR operators with another NEAR operator - e.g. (vanilla OR chocolate) NEAR/5 (ice-cream NEAR/5 cake).".to_string(),
                            });
                        }
                    }
                }
            }
            Expression::Proximity { terms, span, .. } => {
                for term in terms {
                    if self.contains_or_at_top_level(term) || self.contains_and_at_top_level(term) {
                        self.errors.push(LintError::ValidationError {
                            span: span.clone(),
                            message: "Please use parentheses for disambiguation when using the OR or NEAR operators with another NEAR operator - e.g. (vanilla OR chocolate) NEAR/5 (ice-cream NEAR/5 cake).".to_string(),
                        });
                        break;
                    }
                }
            }
            _ => {}
        }
    }
    
    fn contains_and_at_top_level(&self, expr: &Expression) -> bool {
        matches!(expr, Expression::BooleanOp { operator: BooleanOperator::And, .. })
    }
    
    fn contains_or_at_top_level(&self, expr: &Expression) -> bool {
        matches!(expr, Expression::BooleanOp { operator: BooleanOperator::Or, .. })
    }
    
    fn contains_near_at_top_level(&self, expr: &Expression) -> bool {
        match expr {
            Expression::Proximity { .. } => true,
            Expression::Group { .. } => false,
            _ => false,
        }
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
        let mut parser = Parser::new(tokens);
        let result = parser.parse().unwrap();
        
        let mut validator = Validator::new();
        let report = validator.validate(&result.query);
        
        assert!(!report.has_errors());
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