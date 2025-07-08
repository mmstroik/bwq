use thiserror::Error;

#[derive(Debug, Clone, PartialEq)]
pub struct Position {
    pub line: usize,
    pub column: usize,
    pub offset: usize,
}

impl Position {
    pub fn new(line: usize, column: usize, offset: usize) -> Self {
        Self {
            line,
            column,
            offset,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Span {
    pub start: Position,
    pub end: Position,
}

impl Span {
    pub fn new(start: Position, end: Position) -> Self {
        Self { start, end }
    }

    pub fn single(pos: Position) -> Self {
        Self {
            start: pos.clone(),
            end: pos,
        }
    }
}

#[derive(Error, Debug, Clone, PartialEq)]
pub enum LintError {
    #[error("Lexer error at {position:?}: {message}")]
    LexerError { position: Position, message: String },

    #[error("Parser error at {span:?}: {message}")]
    ParserError { span: Span, message: String },

    #[error("Validation error at {span:?}: {message}")]
    ValidationError { span: Span, message: String },

    #[error("Boolean operator '{operator}' must be capitalized at {span:?}")]
    InvalidBooleanCase { span: Span, operator: String },

    #[error("Unbalanced parentheses at {span:?}")]
    UnbalancedParentheses { span: Span },

    #[error(
        "Invalid wildcard placement at {span:?}: wildcards cannot be at the beginning of a word"
    )]
    InvalidWildcardPlacement { span: Span },

    #[error("Invalid proximity operator syntax at {span:?}: {message}")]
    InvalidProximityOperator { span: Span, message: String },

    #[error("Invalid field operator syntax at {span:?}: {message}")]
    InvalidFieldOperator { span: Span, message: String },

    #[error("Invalid range syntax at {span:?}: expected '[value TO value]'")]
    InvalidRangeSyntax { span: Span },

    #[error("Unexpected token '{token}' at {span:?}")]
    UnexpectedToken { span: Span, token: String },

    #[error("Expected '{expected}' but found '{found}' at {span:?}")]
    ExpectedToken {
        span: Span,
        expected: String,
        found: String,
    },

    #[error("Field validation error at {span:?}: {message}")]
    FieldValidationError { span: Span, message: String },

    #[error("Proximity operator error at {span:?}: {message}")]
    ProximityOperatorError { span: Span, message: String },

    #[error("Range validation error at {span:?}: {message}")]
    RangeValidationError { span: Span, message: String },

    #[error("Operator mixing error at {span:?}: {message}")]
    OperatorMixingError { span: Span, message: String },

    #[error("Pure negative query error at {span:?}: {message}")]
    PureNegativeQueryError { span: Span, message: String },
}

impl LintError {
    pub fn code(&self) -> &'static str {
        match self {
            LintError::LexerError { .. } => "E001",
            LintError::ParserError { .. } => "E002",
            LintError::ValidationError { .. } => "E003",
            LintError::InvalidBooleanCase { .. } => "E004",
            LintError::UnbalancedParentheses { .. } => "E005",
            LintError::InvalidWildcardPlacement { .. } => "E006",
            LintError::InvalidProximityOperator { .. } => "E007",
            LintError::InvalidFieldOperator { .. } => "E008",
            LintError::InvalidRangeSyntax { .. } => "E009",
            LintError::UnexpectedToken { .. } => "E010",
            LintError::ExpectedToken { .. } => "E011",
            LintError::FieldValidationError { .. } => "E012",
            LintError::ProximityOperatorError { .. } => "E013",
            LintError::RangeValidationError { .. } => "E014",
            LintError::OperatorMixingError { .. } => "E015",
            LintError::PureNegativeQueryError { .. } => "E016",
        }
    }

    pub fn span_json(&self) -> serde_json::Value {
        match self {
            LintError::LexerError { position, .. } => serde_json::json!({
                "start": {"line": position.line, "column": position.column, "offset": position.offset},
                "end": {"line": position.line, "column": position.column + 1, "offset": position.offset + 1}
            }),
            LintError::ParserError { span, .. }
            | LintError::ValidationError { span, .. }
            | LintError::InvalidBooleanCase { span, .. }
            | LintError::UnbalancedParentheses { span }
            | LintError::InvalidWildcardPlacement { span }
            | LintError::InvalidProximityOperator { span, .. }
            | LintError::InvalidFieldOperator { span, .. }
            | LintError::InvalidRangeSyntax { span }
            | LintError::UnexpectedToken { span, .. }
            | LintError::ExpectedToken { span, .. }
            | LintError::FieldValidationError { span, .. }
            | LintError::ProximityOperatorError { span, .. }
            | LintError::RangeValidationError { span, .. }
            | LintError::OperatorMixingError { span, .. }
            | LintError::PureNegativeQueryError { span, .. } => serde_json::json!({
                "start": {"line": span.start.line, "column": span.start.column, "offset": span.start.offset},
                "end": {"line": span.end.line, "column": span.end.column, "offset": span.end.offset}
            }),
        }
    }

    pub fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "code": self.code(),
            "message": format!("{}", self),
            "span": self.span_json()
        })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum LintWarning {
    PotentialTypo { span: Span, suggestion: String },
    DeprecatedOperator { span: Span, replacement: String },
    PerformanceWarning { span: Span, message: String },
}

impl std::fmt::Display for LintWarning {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LintWarning::PotentialTypo { suggestion, .. } => {
                write!(f, "Potential typo. Did you mean '{suggestion}'?")
            }
            LintWarning::DeprecatedOperator { replacement, .. } => {
                write!(f, "Deprecated operator. Consider using '{replacement}'")
            }
            LintWarning::PerformanceWarning { message, .. } => {
                write!(f, "Performance warning: {message}")
            }
        }
    }
}

impl LintWarning {
    pub fn code(&self) -> &'static str {
        match self {
            LintWarning::PotentialTypo { .. } => "W001",
            LintWarning::DeprecatedOperator { .. } => "W002",
            LintWarning::PerformanceWarning { .. } => "W003",
        }
    }

    pub fn span(&self) -> &Span {
        match self {
            LintWarning::PotentialTypo { span, .. }
            | LintWarning::DeprecatedOperator { span, .. }
            | LintWarning::PerformanceWarning { span, .. } => span,
        }
    }

    pub fn to_json(&self) -> serde_json::Value {
        let span = self.span();
        serde_json::json!({
            "code": self.code(),
            "message": format!("{}", self),
            "span": {
                "start": {"line": span.start.line, "column": span.start.column, "offset": span.start.offset},
                "end": {"line": span.end.line, "column": span.end.column, "offset": span.end.offset}
            }
        })
    }
}

pub type LintResult<T> = Result<T, LintError>;

#[derive(Debug, Clone, PartialEq, Default)]
pub struct LintReport {
    pub errors: Vec<LintError>,
    pub warnings: Vec<LintWarning>,
}

impl LintReport {
    pub fn new() -> Self {
        Self {
            errors: Vec::new(),
            warnings: Vec::new(),
        }
    }

    pub fn add_error(&mut self, error: LintError) {
        self.errors.push(error);
    }

    pub fn add_warning(&mut self, warning: LintWarning) {
        self.warnings.push(warning);
    }

    pub fn has_errors(&self) -> bool {
        !self.errors.is_empty()
    }

    pub fn has_warnings(&self) -> bool {
        !self.warnings.is_empty()
    }

    pub fn is_clean(&self) -> bool {
        self.errors.is_empty() && self.warnings.is_empty()
    }
}
