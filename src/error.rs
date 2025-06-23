use thiserror::Error;

/// Position information for error reporting
#[derive(Debug, Clone, PartialEq)]
pub struct Position {
    pub line: usize,
    pub column: usize,
    pub offset: usize,
}

impl Position {
    pub fn new(line: usize, column: usize, offset: usize) -> Self {
        Self { line, column, offset }
    }
}

/// Span information for error reporting
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

/// Linting errors
#[derive(Error, Debug, Clone, PartialEq)]
pub enum LintError {
    #[error("Lexer error at {position:?}: {message}")]
    LexerError { 
        position: Position,
        message: String 
    },

    #[error("Parser error at {span:?}: {message}")]
    ParserError { 
        span: Span,
        message: String 
    },

    #[error("Validation error at {span:?}: {message}")]
    ValidationError { 
        span: Span,
        message: String 
    },

    #[error("Boolean operator '{operator}' must be capitalized at {span:?}")]
    InvalidBooleanCase { 
        span: Span,
        operator: String 
    },

    #[error("Unbalanced parentheses at {span:?}")]
    UnbalancedParentheses { 
        span: Span 
    },

    #[error("Invalid wildcard placement at {span:?}: wildcards cannot be at the beginning of a word")]
    InvalidWildcardPlacement { 
        span: Span 
    },

    #[error("Invalid proximity operator syntax at {span:?}: {message}")]
    InvalidProximityOperator { 
        span: Span,
        message: String 
    },

    #[error("Invalid field operator syntax at {span:?}: {message}")]
    InvalidFieldOperator { 
        span: Span,
        message: String 
    },

    #[error("Invalid range syntax at {span:?}: expected '[value TO value]'")]
    InvalidRangeSyntax { 
        span: Span 
    },

    #[error("Unexpected token '{token}' at {span:?}")]
    UnexpectedToken { 
        span: Span,
        token: String 
    },

    #[error("Expected '{expected}' but found '{found}' at {span:?}")]
    ExpectedToken { 
        span: Span,
        expected: String,
        found: String 
    },
}

/// Warning types for non-critical issues
#[derive(Debug, Clone, PartialEq)]
pub enum LintWarning {
    PotentialTypo { span: Span, suggestion: String },
    DeprecatedOperator { span: Span, replacement: String },
    PerformanceWarning { span: Span, message: String },
}

/// Result type for linting operations
pub type LintResult<T> = Result<T, LintError>;

/// Container for all linting results
#[derive(Debug, Clone, PartialEq)]
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