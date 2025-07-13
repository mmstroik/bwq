use crate::ast::*;
use crate::error::{LintError, LintWarning};
use crate::validation::{ValidationContext, ValidationResult, ValidationRule};

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
                            return ValidationResult::with_error(LintError::FieldValidationError {
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
            } => match (start.parse::<i32>(), end.parse::<i32>()) {
                (Ok(start_num), Ok(end_num)) => {
                    if !(0..=5).contains(&start_num) || !(0..=5).contains(&end_num) {
                        return ValidationResult::with_error(LintError::FieldValidationError {
                            span: span.clone(),
                            message: "Rating values must be between 0 and 5".to_string(),
                        });
                    }
                    ValidationResult::new()
                }
                _ => ValidationResult::with_error(LintError::FieldValidationError {
                    span: span.clone(),
                    message: "Rating range values must be numbers".to_string(),
                }),
            },
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
                                        LintError::FieldValidationError {
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
                                        LintError::FieldValidationError {
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
            } => match (start.parse::<f64>(), end.parse::<f64>()) {
                (Ok(start_num), Ok(end_num)) => {
                    match field {
                        FieldType::Latitude => {
                            if !(-90.0..=90.0).contains(&start_num)
                                || !(-90.0..=90.0).contains(&end_num)
                            {
                                return ValidationResult::with_error(
                                    LintError::FieldValidationError {
                                        span: span.clone(),
                                        message: "Latitude values must be between -90 and 90"
                                            .to_string(),
                                    },
                                );
                            }
                        }
                        FieldType::Longitude => {
                            if !(-180.0..=180.0).contains(&start_num)
                                || !(-180.0..=180.0).contains(&end_num)
                            {
                                return ValidationResult::with_error(
                                    LintError::FieldValidationError {
                                        span: span.clone(),
                                        message: "Longitude values must be between -180 and 180"
                                            .to_string(),
                                    },
                                );
                            }
                        }
                        _ => {}
                    }
                    ValidationResult::new()
                }
                _ => {
                    let field_name = match field {
                        FieldType::Latitude => "Latitude",
                        FieldType::Longitude => "Longitude",
                        _ => "Coordinate",
                    };
                    ValidationResult::with_error(LintError::FieldValidationError {
                        span: span.clone(),
                        message: format!("{field_name} range values must be numbers"),
                    })
                }
            },
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
                        message: "Language codes should be 2-character ISO 639-1 codes (e.g., 'en', 'es')".to_string(),
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
                if !matches!(gender.as_str(), "F" | "M") {
                    return ValidationResult::with_error(LintError::FieldValidationError {
                        span: span.clone(),
                        message: "authorGender must be 'F' or 'M'".to_string(),
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
                        return ValidationResult::with_error(LintError::FieldValidationError {
                            span: span.clone(),
                            message: format!("{field_name} must be 'true' or 'false'"),
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
                let valid_types = ["COMMENT", "REPLY", "RETWEET", "QUOTE"];
                if !valid_types.contains(&engagement_type.as_str()) {
                    return ValidationResult::with_error(LintError::FieldValidationError {
                        span: span.clone(),
                        message: "engagementType must be 'COMMENT', 'REPLY', 'RETWEET', or 'QUOTE'"
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
                field: FieldType::EngagementType,
                ..
            }
        )
    }
}

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
                    return ValidationResult::with_error(LintError::FieldValidationError {
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
                    return ValidationResult::with_error(LintError::FieldValidationError {
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
                    return ValidationResult::with_error(LintError::InvalidFieldRange {
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

pub struct FollowerCountFieldRule;

impl ValidationRule for FollowerCountFieldRule {
    fn name(&self) -> &'static str {
        "follower-count-field"
    }

    fn validate(&self, expr: &Expression, _ctx: &ValidationContext) -> ValidationResult {
        match expr {
            Expression::Field {
                field: FieldType::AuthorFollowers,
                value,
                span,
            } => {
                // Check if the value is a range (valid) or a term (invalid)
                if let Expression::Range { start, end, .. } = value.as_ref() {
                    match (start.parse::<i64>(), end.parse::<i64>()) {
                        (Ok(start_num), Ok(end_num)) => {
                            let mut result = ValidationResult::new();

                            if start_num < 0 || end_num < 0 {
                                result.errors.push(LintError::InvalidFieldRange {
                                    span: span.clone(),
                                    message: "Follower counts cannot be negative".to_string(),
                                });
                            }

                            if end_num.to_string().len() > 10 {
                                result.errors.push(LintError::InvalidFieldRange {
                                    span: span.clone(),
                                    message: "Follower counts cannot exceed 10 digits".to_string(),
                                });
                            }

                            result
                        }
                        _ => ValidationResult::with_error(LintError::FieldValidationError {
                            span: span.clone(),
                            message: "authorFollowers range values must be numbers".to_string(),
                        }),
                    }
                } else {
                    ValidationResult::with_error(LintError::FieldValidationError {
                        span: span.clone(),
                        message: "authorFollowers must be used with a range (e.g., authorFollowers:[100 TO 1000])".to_string(),
                    })
                }
            }
            _ => ValidationResult::new(),
        }
    }

    fn can_validate(&self, expr: &Expression) -> bool {
        matches!(
            expr,
            Expression::Field {
                field: FieldType::AuthorFollowers,
                ..
            }
        )
    }
}

pub struct GuidFieldRule;

impl ValidationRule for GuidFieldRule {
    fn name(&self) -> &'static str {
        "guid-field"
    }

    fn validate(&self, expr: &Expression, _ctx: &ValidationContext) -> ValidationResult {
        if let Expression::Field {
            field: FieldType::Guid,
            value,
            span,
        } = expr
        {
            if let Expression::Term {
                term: Term::Word { value: guid_value },
                ..
            } = value.as_ref()
            {
                // GUID should be digits only or digits with underscores (for Facebook post IDs)
                if !guid_value.chars().all(|c| c.is_ascii_digit() || c == '_') {
                    return ValidationResult::with_error(LintError::FieldValidationError {
                        span: span.clone(),
                        message: "guid must contain only digits or digits with underscores (e.g., '123456789' or '123_456_789')".to_string(),
                    });
                }

                // Should not be all underscores or start/end with underscore
                if guid_value.is_empty()
                    || guid_value.chars().all(|c| c == '_')
                    || guid_value.starts_with('_')
                    || guid_value.ends_with('_')
                {
                    return ValidationResult::with_error(LintError::FieldValidationError {
                        span: span.clone(),
                        message:
                            "guid must contain digits and cannot start or end with underscores"
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
                field: FieldType::Guid,
                ..
            }
        )
    }
}
