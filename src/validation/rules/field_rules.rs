use crate::ast::*;
use crate::error::{LintError, LintWarning};
use crate::validation::{ValidationContext, ValidationResult, ValidationRule};

// Rating field validation rule
pub struct RatingFieldRule;

impl ValidationRule for RatingFieldRule {
    fn name(&self) -> &'static str {
        "rating-field"
    }

    fn validate(&self, expr: &Expression, _ctx: &ValidationContext) -> ValidationResult {
        match expr {
            Expression::Field {
                field: FieldType::Rating,
                value,
                span,
            } => {
                if let Expression::Term {
                    term: Term::Word { value: rating },
                    ..
                } = value.as_ref()
                {
                    if let Ok(rating_num) = rating.parse::<i32>() {
                        if !(0..=5).contains(&rating_num) {
                            return ValidationResult::with_error(LintError::ValidationError {
                                span: span.clone(),
                                message: "Rating must be between 0 and 5".to_string(),
                            });
                        }
                    }
                }
                ValidationResult::new()
            }
            Expression::Range {
                field: Some(FieldType::Rating),
                start,
                end,
                span,
            } => {
                if let (Ok(start_num), Ok(end_num)) = (start.parse::<i32>(), end.parse::<i32>()) {
                    if !(0..=5).contains(&start_num) || !(0..=5).contains(&end_num) {
                        return ValidationResult::with_error(LintError::ValidationError {
                            span: span.clone(),
                            message: "Rating values must be between 0 and 5".to_string(),
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
            Expression::Field {
                field: FieldType::Rating,
                ..
            } | Expression::Range {
                field: Some(FieldType::Rating),
                ..
            }
        )
    }
}

// Coordinate field validation rule
pub struct CoordinateFieldRule;

impl ValidationRule for CoordinateFieldRule {
    fn name(&self) -> &'static str {
        "coordinate-field"
    }

    fn validate(&self, expr: &Expression, _ctx: &ValidationContext) -> ValidationResult {
        match expr {
            Expression::Field { field, value, span } => {
                if let Expression::Term {
                    term: Term::Word { value: coord },
                    ..
                } = value.as_ref()
                {
                    if let Ok(coord_num) = coord.parse::<f64>() {
                        match field {
                            FieldType::Latitude => {
                                if !(-90.0..=90.0).contains(&coord_num) {
                                    return ValidationResult::with_error(
                                        LintError::ValidationError {
                                            span: span.clone(),
                                            message: "Latitude must be between -90 and 90"
                                                .to_string(),
                                        },
                                    );
                                }
                            }
                            FieldType::Longitude => {
                                if !(-180.0..=180.0).contains(&coord_num) {
                                    return ValidationResult::with_error(
                                        LintError::ValidationError {
                                            span: span.clone(),
                                            message: "Longitude must be between -180 and 180"
                                                .to_string(),
                                        },
                                    );
                                }
                            }
                            _ => {}
                        }
                    }
                }
                ValidationResult::new()
            }
            Expression::Range {
                field: Some(field),
                start,
                end,
                span,
            } => {
                if let (Ok(start_num), Ok(end_num)) = (start.parse::<f64>(), end.parse::<f64>()) {
                    match field {
                        FieldType::Latitude => {
                            if !(-90.0..=90.0).contains(&start_num)
                                || !(-90.0..=90.0).contains(&end_num)
                            {
                                return ValidationResult::with_error(LintError::ValidationError {
                                    span: span.clone(),
                                    message: "Latitude values must be between -90 and 90"
                                        .to_string(),
                                });
                            }
                        }
                        FieldType::Longitude => {
                            if !(-180.0..=180.0).contains(&start_num)
                                || !(-180.0..=180.0).contains(&end_num)
                            {
                                return ValidationResult::with_error(LintError::ValidationError {
                                    span: span.clone(),
                                    message: "Longitude values must be between -180 and 180"
                                        .to_string(),
                                });
                            }
                        }
                        _ => {}
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
            Expression::Field {
                field: FieldType::Latitude | FieldType::Longitude,
                ..
            } | Expression::Range {
                field: Some(FieldType::Latitude | FieldType::Longitude),
                ..
            }
        )
    }
}

// Language field validation rule
pub struct LanguageFieldRule;

impl ValidationRule for LanguageFieldRule {
    fn name(&self) -> &'static str {
        "language-field"
    }

    fn validate(&self, expr: &Expression, _ctx: &ValidationContext) -> ValidationResult {
        if let Expression::Field {
            field: FieldType::Language,
            value,
            span,
        } = expr
        {
            if let Expression::Term {
                term: Term::Word { value: lang_code },
                ..
            } = value.as_ref()
            {
                if lang_code.len() != 2 || !lang_code.chars().all(|c| c.is_ascii_lowercase()) {
                    return ValidationResult::with_warning(LintWarning::PotentialTypo {
                        span: span.clone(),
                        suggestion: "Language codes should be 2-character ISO 639-1 codes (e.g., 'en', 'es')".to_string(),
                    });
                }
            }
        }
        ValidationResult::new()
    }

    fn can_validate(&self, expr: &Expression) -> bool {
        matches!(
            expr,
            Expression::Field {
                field: FieldType::Language,
                ..
            }
        )
    }
}

// Author gender field validation rule
pub struct AuthorGenderFieldRule;

impl ValidationRule for AuthorGenderFieldRule {
    fn name(&self) -> &'static str {
        "author-gender-field"
    }

    fn validate(&self, expr: &Expression, _ctx: &ValidationContext) -> ValidationResult {
        if let Expression::Field {
            field: FieldType::AuthorGender,
            value,
            span,
        } = expr
        {
            if let Expression::Term {
                term: Term::Word { value: gender },
                ..
            } = value.as_ref()
            {
                if !matches!(
                    gender.as_str(),
                    "F" | "M" | "f" | "m" | "X" | "x" | "U" | "u"
                ) {
                    return ValidationResult::with_warning(LintWarning::PotentialTypo {
                        span: span.clone(),
                        suggestion: "Common gender values are 'F', 'M', 'X', or 'U'".to_string(),
                    });
                }
            }
        }
        ValidationResult::new()
    }

    fn can_validate(&self, expr: &Expression) -> bool {
        matches!(
            expr,
            Expression::Field {
                field: FieldType::AuthorGender,
                ..
            }
        )
    }
}

// Boolean field validation rule
pub struct BooleanFieldRule;

impl ValidationRule for BooleanFieldRule {
    fn name(&self) -> &'static str {
        "boolean-field"
    }

    fn validate(&self, expr: &Expression, _ctx: &ValidationContext) -> ValidationResult {
        if let Expression::Field { field, value, span } = expr {
            let is_boolean_field = matches!(
                field,
                FieldType::AuthorVerified
                    | FieldType::RedditSpoiler
                    | FieldType::SubredditNSFW
                    | FieldType::SensitiveContent
            );

            if is_boolean_field {
                if let Expression::Term {
                    term: Term::Word { value: bool_val },
                    ..
                } = value.as_ref()
                {
                    if !matches!(bool_val.as_str(), "true" | "false") {
                        let field_name = field.as_str();
                        return ValidationResult::with_error(LintError::ValidationError {
                            span: span.clone(),
                            message: format!("{} must be 'true' or 'false'", field_name),
                        });
                    }
                }
            }
        }
        ValidationResult::new()
    }

    fn can_validate(&self, expr: &Expression) -> bool {
        if let Expression::Field { field, .. } = expr {
            matches!(
                field,
                FieldType::AuthorVerified
                    | FieldType::RedditSpoiler
                    | FieldType::SubredditNSFW
                    | FieldType::SensitiveContent
            )
        } else {
            false
        }
    }
}

// Engagement type field validation rule
pub struct EngagementTypeFieldRule;

impl ValidationRule for EngagementTypeFieldRule {
    fn name(&self) -> &'static str {
        "engagement-type-field"
    }

    fn validate(&self, expr: &Expression, _ctx: &ValidationContext) -> ValidationResult {
        if let Expression::Field {
            field: FieldType::EngagementType,
            value,
            span,
        } = expr
        {
            if let Expression::Term {
                term: Term::Word {
                    value: engagement_type,
                },
                ..
            } = value.as_ref()
            {
                let common_types = [
                    "COMMENT", "REPLY", "RETWEET", "QUOTE", "LIKE", "SHARE", "MENTION",
                ];
                if !common_types.contains(&engagement_type.as_str()) {
                    return ValidationResult::with_warning(LintWarning::PotentialTypo {
                        span: span.clone(),
                        suggestion: "Common engagement types are 'COMMENT', 'REPLY', 'RETWEET', 'QUOTE', 'LIKE'".to_string(),
                    });
                }
            }
        }
        ValidationResult::new()
    }

    fn can_validate(&self, expr: &Expression) -> bool {
        matches!(
            expr,
            Expression::Field {
                field: FieldType::EngagementType,
                ..
            }
        )
    }
}

// Verified type field validation rule
pub struct VerifiedTypeFieldRule;

impl ValidationRule for VerifiedTypeFieldRule {
    fn name(&self) -> &'static str {
        "verified-type-field"
    }

    fn validate(&self, expr: &Expression, _ctx: &ValidationContext) -> ValidationResult {
        if let Expression::Field {
            field: FieldType::AuthorVerifiedType,
            value,
            span,
        } = expr
        {
            if let Expression::Term {
                term: Term::Word {
                    value: verified_type,
                },
                ..
            } = value.as_ref()
            {
                if !matches!(verified_type.as_str(), "blue" | "business" | "government") {
                    return ValidationResult::with_error(LintError::ValidationError {
                        span: span.clone(),
                        message: "authorVerifiedType must be 'blue', 'business', or 'government'"
                            .to_string(),
                    });
                }
            }
        }
        ValidationResult::new()
    }

    fn can_validate(&self, expr: &Expression) -> bool {
        matches!(
            expr,
            Expression::Field {
                field: FieldType::AuthorVerifiedType,
                ..
            }
        )
    }
}

// Minute of day field validation rule
pub struct MinuteOfDayFieldRule;

impl ValidationRule for MinuteOfDayFieldRule {
    fn name(&self) -> &'static str {
        "minute-of-day-field"
    }

    fn validate(&self, expr: &Expression, _ctx: &ValidationContext) -> ValidationResult {
        if let Expression::Range {
            field: Some(FieldType::MinuteOfDay),
            start,
            end,
            span,
        } = expr
        {
            if let (Ok(start_num), Ok(end_num)) = (start.parse::<i32>(), end.parse::<i32>()) {
                if !(0..=1439).contains(&start_num) || !(0..=1439).contains(&end_num) {
                    return ValidationResult::with_error(LintError::ValidationError {
                        span: span.clone(),
                        message: "minuteOfDay values must be between 0 and 1439".to_string(),
                    });
                }
            }
        }
        ValidationResult::new()
    }

    fn can_validate(&self, expr: &Expression) -> bool {
        matches!(
            expr,
            Expression::Range {
                field: Some(FieldType::MinuteOfDay),
                ..
            }
        )
    }
}

// Range field validation rule (general range logic)
pub struct RangeFieldRule;

impl ValidationRule for RangeFieldRule {
    fn name(&self) -> &'static str {
        "range-field"
    }

    fn validate(&self, expr: &Expression, _ctx: &ValidationContext) -> ValidationResult {
        if let Expression::Range {
            start, end, span, ..
        } = expr
        {
            if let (Ok(start_num), Ok(end_num)) = (start.parse::<f64>(), end.parse::<f64>()) {
                if start_num > end_num {
                    return ValidationResult::with_error(LintError::ValidationError {
                        span: span.clone(),
                        message: "Range start value cannot be greater than end value".to_string(),
                    });
                }
            }
        }
        ValidationResult::new()
    }

    fn can_validate(&self, expr: &Expression) -> bool {
        matches!(expr, Expression::Range { .. })
    }
}
