use crate::ast::*;
use crate::error::{LintError, LintResult, LintWarning, Span};
use crate::lexer::{Token, TokenType};

/// result type with parsed query and any parser warnings
pub struct ParseResult {
    pub query: Query,
    pub warnings: Vec<LintWarning>,
}

/// recursive descent parser for queries
pub struct Parser {
    tokens: Vec<Token>,
    current: usize,
    implicit_and_spans: Vec<Span>,
}

impl Parser {
    pub fn new(tokens: Vec<Token>) -> Result<Self, LintError> {
        // filter out all comment-related tokens including content between comment markers
        let mut filtered_tokens: Vec<Token> = Vec::new();
        let mut inside_comment = false;
        let mut comment_start_span: Option<Span> = None;

        for token in tokens {
            match &token.token_type {
                TokenType::CommentStart => {
                    inside_comment = true;
                    comment_start_span = Some(token.span.clone());
                }
                TokenType::CommentEnd => {
                    inside_comment = false;
                    comment_start_span = None;
                }
                TokenType::Eof if inside_comment => {
                    return Err(LintError::ParserError {
                        span: comment_start_span.unwrap(),
                        message: "Please add a >>> mark to close this commented text.".to_string(),
                    });
                }
                _ if inside_comment => {}
                _ => {
                    filtered_tokens.push(token);
                }
            }
        }

        Ok(Self {
            tokens: filtered_tokens,
            current: 0,
            implicit_and_spans: Vec::new(),
        })
    }

    /// parse the tokens into a queryAST
    pub fn parse(&mut self) -> LintResult<ParseResult> {
        let expression = self.parse_expression()?;
        let span = expression.span().clone();

        // ensure we've consumed all tokens except EOF
        if !self.is_at_end() && !matches!(self.peek().token_type, TokenType::Eof) {
            // Check for specific case of unmatched closing parenthesis
            if matches!(self.peek().token_type, TokenType::RightParen) {
                return Err(LintError::ParserError {
                    span: self.peek().span.clone(),
                    message: "Unmatched closing parenthesis".to_string(),
                });
            }

            return Err(LintError::UnexpectedToken {
                span: self.peek().span.clone(),
                token: self.peek().token_type.to_string(),
            });
        }

        let mut warnings = Vec::new();
        for span in &self.implicit_and_spans {
            warnings.push(LintWarning::PotentialTypo {
                span: span.clone(),
                message: "Two or more terms without an operator between them are implicitly ANDed. Consider using explicit 'AND' operator for clarity".to_string(),
            });
        }

        Ok(ParseResult {
            query: Query { expression, span },
            warnings,
        })
    }

    fn parse_expression(&mut self) -> LintResult<Expression> {
        // check for leading operators
        if matches!(
            self.peek().token_type,
            TokenType::And | TokenType::Or | TokenType::Near(_) | TokenType::NearForward(_)
        ) {
            return Err(LintError::ParserError {
                span: self.peek().span.clone(),
                message: format!(
                    "'{}' operator requires a term before it",
                    self.peek().token_type
                ),
            });
        }

        let mut left = self.parse_and_expression()?;

        while self.match_token(&TokenType::Or) {
            let operator = BooleanOperator::Or;
            let _operator_span = self.previous().span.clone();
            let right = self.parse_and_expression()?;

            let span = Span::new(left.span().start.clone(), right.span().end.clone());
            left = Expression::BooleanOp {
                operator,
                left: Box::new(left),
                right: Some(Box::new(right)),
                span,
            };
        }

        Ok(left)
    }

    fn parse_and_expression(&mut self) -> LintResult<Expression> {
        let mut left = self.parse_not_expression()?;

        loop {
            if self.match_token(&TokenType::And) {
                let operator = BooleanOperator::And;
                let _operator_span = self.previous().span.clone();
                let right = self.parse_not_expression()?;

                let span = Span::new(left.span().start.clone(), right.span().end.clone());
                left = Expression::BooleanOp {
                    operator,
                    left: Box::new(left),
                    right: Some(Box::new(right)),
                    span,
                };
            } else if self.is_implicit_and_candidate() {
                // warn on implicit AND (space-separated terms)
                let right = self.parse_not_expression()?;

                // Create span for the gap between terms (for the warning)
                let gap_span = Span::new(left.span().end.clone(), right.span().start.clone());

                // Create span for the full expression (for the AST node)
                let full_span = Span::new(left.span().start.clone(), right.span().end.clone());

                left = Expression::BooleanOp {
                    operator: BooleanOperator::And,
                    left: Box::new(left),
                    right: Some(Box::new(right)),
                    span: full_span,
                };

                self.implicit_and_spans.push(gap_span);
            } else {
                break;
            }
        }

        Ok(left)
    }

    fn parse_not_expression(&mut self) -> LintResult<Expression> {
        let mut left = if self.match_token(&TokenType::Not) {
            // handle leading NOT operator
            let operator_span = self.previous().span.clone();
            let right = self.parse_proximity_expression()?;

            let span = Span::new(operator_span.start.clone(), right.span().end.clone());
            Expression::BooleanOp {
                operator: BooleanOperator::Not,
                left: Box::new(right),
                right: None,
                span,
            }
        } else {
            self.parse_proximity_expression()?
        };

        while self.match_token(&TokenType::Not) {
            let operator = BooleanOperator::Not;
            let _operator_span = self.previous().span.clone();
            let right = self.parse_proximity_expression()?;

            let span = Span::new(left.span().start.clone(), right.span().end.clone());
            left = Expression::BooleanOp {
                operator,
                left: Box::new(left),
                right: Some(Box::new(right)),
                span,
            };
        }

        Ok(left)
    }

    fn parse_proximity_expression(&mut self) -> LintResult<Expression> {
        let left = self.parse_primary()?;

        // handle proximity operators
        if self.match_token(&TokenType::Tilde) {
            let tilde_span = self.previous().span.clone();
            let distance;

            if left.span().end.offset != tilde_span.start.offset {
                return Err(LintError::ParserError {
                    span: tilde_span,
                    message: "The ~ operator must be immediately attached to the preceding term (e.g., apple~5, not apple ~5).".to_string(),
                });
            }

            // require distance number immediately after tilde (no spaces)
            if let TokenType::Number(num_str) = &self.peek().token_type {
                let number_token = self.peek();
                if tilde_span.end.offset == number_token.span.start.offset {
                    distance = num_str.parse::<u32>().ok();
                    self.advance();
                    if distance.is_none() {
                        return Err(LintError::ParserError {
                            span: tilde_span,
                            message:
                                "Invalid proximity distance. Distance must be a positive number."
                                    .to_string(),
                        });
                    }
                } else {
                    return Err(LintError::ParserError {
                        span: tilde_span,
                        message: "The ~ operator requires a distance number immediately after it (e.g., ~5 for proximity within 5 words).".to_string(),
                    });
                }
            } else {
                return Err(LintError::ParserError {
                    span: tilde_span,
                    message: "The ~ operator requires a distance number (e.g., ~5 for proximity within 5 words).".to_string(),
                });
            }

            // tilde is valid after quoted phrases, grouped expressions, or single terms
            let is_valid_tilde_context =
                matches!(&left, Expression::Term { .. } | Expression::Group { .. });

            if !is_valid_tilde_context {
                return Err(LintError::ParserError {
                    span: tilde_span,
                    message: "The ~ operator should be used after a search term, quoted phrase, or grouped expression. If this should be part of a search term, it must be quoted (or escaped using the \\ character).".to_string(),
                });
            }

            let terms = vec![left];
            let end_span = tilde_span.end.clone();
            let span = Span::new(terms[0].span().start.clone(), end_span);

            return Ok(Expression::Proximity {
                operator: ProximityOperator::Proximity {
                    distance: Some(distance.unwrap()),
                },
                terms,
                span,
            });
        }

        // handle NEAR/x and NEAR/xf
        if let TokenType::Near(distance) = &self.peek().token_type {
            let distance = *distance;
            self.advance();
            let _operator_span = self.previous().span.clone();
            let right = self.parse_primary()?;

            let span = Span::new(left.span().start.clone(), right.span().end.clone());
            return Ok(Expression::Proximity {
                operator: ProximityOperator::Near { distance },
                terms: vec![left, right],
                span,
            });
        }

        if let TokenType::NearForward(distance) = &self.peek().token_type {
            let distance = *distance;
            self.advance();
            let _operator_span = self.previous().span.clone();
            let right = self.parse_primary()?;

            let span = Span::new(left.span().start.clone(), right.span().end.clone());
            return Ok(Expression::Proximity {
                operator: ProximityOperator::NearForward { distance },
                terms: vec![left, right],
                span,
            });
        }

        Ok(left)
    }

    fn parse_primary(&mut self) -> LintResult<Expression> {
        // Dispatch to specialized parsing methods based on token type
        match &self.peek().token_type {
            TokenType::LeftParen => self.parse_grouped_expression(),
            TokenType::LeftBrace => self.parse_case_sensitive_term(),
            TokenType::LeftBracket => {
                self.match_token(&TokenType::LeftBracket);
                self.parse_range()
            }
            TokenType::Word(_) => {
                if self.peek_ahead(1).map(|t| &t.token_type) == Some(&TokenType::Colon) {
                    self.parse_field_operation()
                } else {
                    self.parse_term()
                }
            }
            _ => self.parse_term(),
        }
    }

    fn parse_grouped_expression(&mut self) -> LintResult<Expression> {
        let start_span = self.advance().span.clone(); // consume '('

        // Check for empty parentheses
        if matches!(self.peek().token_type, TokenType::RightParen) {
            let end_span = self.peek().span.clone();
            let span = Span::new(start_span.start, end_span.end);
            return Err(LintError::ParserError {
                span,
                message: "Empty parentheses are not allowed".to_string(),
            });
        }

        let expr = self.parse_expression()?;

        if !self.match_token(&TokenType::RightParen) {
            return Err(LintError::ExpectedToken {
                span: self.peek().span.clone(),
                expected: ")".to_string(),
                found: self.peek().token_type.to_string(),
            });
        }

        let end_span = self.previous().span.clone();
        let span = Span::new(start_span.start, end_span.end);

        Ok(Expression::Group {
            expression: Box::new(expr),
            span,
        })
    }

    fn parse_case_sensitive_term(&mut self) -> LintResult<Expression> {
        let start_span = self.advance().span.clone(); // consume '{'

        let mut content_tokens = Vec::new();
        let mut found_closing_brace = false;

        while !self.is_at_end() {
            if matches!(self.peek().token_type, TokenType::RightBrace) {
                found_closing_brace = true;
                break;
            }
            content_tokens.push(self.advance().clone());
        }

        if !found_closing_brace {
            return Err(LintError::ExpectedToken {
                span: self.peek().span.clone(),
                expected: "}".to_string(),
                found: self.peek().token_type.to_string(),
            });
        }

        let end_span = self.advance().span.clone();
        let span = Span::new(start_span.start, end_span.end);

        let mut value = String::new();
        for token in content_tokens.iter() {
            match &token.token_type {
                TokenType::Word(word) => {
                    value.push_str(word);
                }
                TokenType::Number(num) => {
                    value.push_str(num);
                }
                TokenType::QuotedString(s) => {
                    value.push_str(&format!("\"{s}\""));
                }
                TokenType::Whitespace => {
                    value.push(' ');
                }
                _ => {
                    // For other token types, use their raw representation
                    value.push_str(&token.raw);
                }
            }
        }

        // Allow empty braces syntactically
        if value.trim().is_empty() {
            value = " ".to_string();
        }

        Ok(Expression::Term {
            term: Term::CaseSensitive { value },
            span,
        })
    }

    fn parse_field_operation(&mut self) -> LintResult<Expression> {
        let TokenType::Word(word) = &self.peek().token_type else {
            return Err(LintError::UnexpectedToken {
                span: self.peek().span.clone(),
                token: self.peek().token_type.to_string(),
            });
        };

        let word = word.clone();
        let word_span = self.peek().span.clone();

        self.advance(); // consume field name
        self.advance(); // consume colon

        // Check if we're at EOF or next token is not a valid value token
        if self.is_at_end() || matches!(self.peek().token_type, TokenType::Eof) {
            return Err(LintError::ParserError {
                span: self.peek().span.clone(),
                message: format!("Field '{word}' requires a value"),
            });
        }

        let value = Box::new(self.parse_primary()?);

        // Handle special case where field value is a range
        let value = if let Expression::Range {
            start,
            end,
            span: range_span,
            ..
        } = value.as_ref()
        {
            if let Some(field_type) = FieldType::parse(&word) {
                Box::new(Expression::Range {
                    field: Some(field_type),
                    start: start.clone(),
                    end: end.clone(),
                    span: range_span.clone(),
                })
            } else {
                value
            }
        } else {
            value
        };

        let span = Span::new(word_span.start, value.span().end.clone());

        if let Some(field_type) = FieldType::parse(&word) {
            Ok(Expression::Field {
                field: field_type,
                value,
                span,
            })
        } else {
            Ok(Expression::Term {
                term: Term::Word {
                    value: format!(
                        "{}:{}",
                        word,
                        match value.as_ref() {
                            Expression::Term {
                                term: Term::Word { value },
                                ..
                            } => value.clone(),
                            Expression::Term {
                                term: Term::Phrase { value },
                                ..
                            } => format!("\"{value}\""),
                            _ => "unknown".to_string(),
                        }
                    ),
                },
                span,
            })
        }
    }

    fn parse_range(&mut self) -> LintResult<Expression> {
        let start_span = self.previous().span.clone();

        let start_value = match &self.peek().token_type {
            TokenType::Word(w) | TokenType::Number(w) => {
                let val = w.clone();
                self.advance();
                val
            }
            _ => {
                return Err(LintError::ExpectedToken {
                    span: self.peek().span.clone(),
                    expected: "number or word".to_string(),
                    found: self.peek().token_type.to_string(),
                });
            }
        };

        if !self.match_token(&TokenType::To) {
            return Err(LintError::ExpectedToken {
                span: self.peek().span.clone(),
                expected: "TO".to_string(),
                found: self.peek().token_type.to_string(),
            });
        }

        let end_value = match &self.peek().token_type {
            TokenType::Word(w) | TokenType::Number(w) => {
                let val = w.clone();
                self.advance();
                val
            }
            _ => {
                return Err(LintError::ExpectedToken {
                    span: self.peek().span.clone(),
                    expected: "number or word".to_string(),
                    found: self.peek().token_type.to_string(),
                });
            }
        };

        if !self.match_token(&TokenType::RightBracket) {
            return Err(LintError::ExpectedToken {
                span: self.peek().span.clone(),
                expected: "]".to_string(),
                found: self.peek().token_type.to_string(),
            });
        }

        let end_span = self.previous().span.clone();
        let span = Span::new(start_span.start, end_span.end);

        Ok(Expression::Range {
            field: None,
            start: start_value,
            end: end_value,
            span,
        })
    }

    // parse_comment function removed - comments are now filtered out during parser construction

    fn parse_term(&mut self) -> LintResult<Expression> {
        let token = self.peek().clone();

        match &token.token_type {
            TokenType::Word(word) => {
                self.advance();
                let term = if word.contains('*') || word.contains('?') {
                    Term::Wildcard {
                        value: word.clone(),
                    }
                } else {
                    Term::Word {
                        value: word.clone(),
                    }
                };

                Ok(Expression::Term {
                    term,
                    span: token.span,
                })
            }
            TokenType::QuotedString(string) => {
                self.advance();
                Ok(Expression::Term {
                    term: Term::Phrase {
                        value: string.clone(),
                    },
                    span: token.span,
                })
            }
            TokenType::Number(number) => {
                self.advance();
                Ok(Expression::Term {
                    term: Term::Word {
                        value: number.clone(),
                    },
                    span: token.span,
                })
            }
            TokenType::Hashtag(hashtag) => {
                self.advance();
                Ok(Expression::Term {
                    term: Term::Hashtag {
                        value: hashtag.clone(),
                    },
                    span: token.span,
                })
            }
            TokenType::Mention(mention) => {
                self.advance();
                Ok(Expression::Term {
                    term: Term::Mention {
                        value: mention.clone(),
                    },
                    span: token.span,
                })
            }
            _ => Err(LintError::UnexpectedToken {
                span: token.span,
                token: token.token_type.to_string(),
            }),
        }
    }

    fn match_token(&mut self, token_type: &TokenType) -> bool {
        if self.check(token_type) {
            self.advance();
            true
        } else {
            false
        }
    }

    fn check(&self, token_type: &TokenType) -> bool {
        if self.is_at_end() {
            false
        } else {
            std::mem::discriminant(&self.peek().token_type) == std::mem::discriminant(token_type)
        }
    }

    fn advance(&mut self) -> &Token {
        if !self.is_at_end() {
            self.current += 1;
        }
        self.previous()
    }

    fn is_at_end(&self) -> bool {
        matches!(self.peek().token_type, TokenType::Eof) || self.current >= self.tokens.len()
    }

    fn peek(&self) -> &Token {
        &self.tokens[self.current]
    }

    fn peek_ahead(&self, offset: usize) -> Option<&Token> {
        let index = self.current + offset;
        if index < self.tokens.len() {
            Some(&self.tokens[index])
        } else {
            None
        }
    }

    fn previous(&self) -> &Token {
        &self.tokens[self.current - 1]
    }

    fn is_implicit_and_candidate(&self) -> bool {
        if self.is_at_end() {
            return false;
        }

        match &self.peek().token_type {
            TokenType::Word(_)
            | TokenType::QuotedString(_)
            | TokenType::Number(_)
            | TokenType::Hashtag(_)
            | TokenType::Mention(_)
            | TokenType::LeftParen
            | TokenType::LeftBrace => true,

            TokenType::Or
            | TokenType::RightParen
            | TokenType::RightBracket
            | TokenType::RightBrace
            | TokenType::Eof => false,

            TokenType::And | TokenType::Not => false,

            _ => false,
        }
    }

    // skip_comments function removed - comments are now filtered out during parser construction
}

impl Expression {
    pub fn span(&self) -> &Span {
        match self {
            Expression::BooleanOp { span, .. } => span,
            Expression::Group { span, .. } => span,
            Expression::Proximity { span, .. } => span,
            Expression::Field { span, .. } => span,
            Expression::Range { span, .. } => span,
            Expression::Term { span, .. } => span,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::Lexer;

    #[test]
    fn test_basic_parsing() {
        let mut lexer = Lexer::new("apple AND juice");
        let tokens = lexer.tokenize().unwrap();
        let mut parser = Parser::new(tokens).unwrap();
        let result = parser.parse().unwrap();

        match result.query.expression {
            Expression::BooleanOp { operator, .. } => {
                assert_eq!(operator, BooleanOperator::And);
            }
            _ => panic!("Expected BooleanOp"),
        }
    }

    #[test]
    fn test_quoted_phrase() {
        let mut lexer = Lexer::new("\"apple juice\"");
        let tokens = lexer.tokenize().unwrap();
        let mut parser = Parser::new(tokens).unwrap();
        let result = parser.parse().unwrap();

        match result.query.expression {
            Expression::Term {
                term: Term::Phrase { value },
                ..
            } => {
                assert_eq!(value, "apple juice");
            }
            _ => panic!("Expected Term with Phrase"),
        }
    }

    #[test]
    fn test_field_operation() {
        let mut lexer = Lexer::new("title:\"apple juice\"");
        let tokens = lexer.tokenize().unwrap();
        let mut parser = Parser::new(tokens).unwrap();
        let result = parser.parse().unwrap();

        match result.query.expression {
            Expression::Field { field, .. } => {
                assert_eq!(field, FieldType::Title);
            }
            _ => panic!("Expected Field operation"),
        }
    }

    #[test]
    fn test_implicit_and() {
        let mut lexer = Lexer::new("apple banana");
        let tokens = lexer.tokenize().unwrap();
        let mut parser = Parser::new(tokens).unwrap();
        let result = parser.parse().unwrap();

        match result.query.expression {
            Expression::BooleanOp { operator, .. } => {
                assert_eq!(operator, BooleanOperator::And);
            }
            _ => panic!("Expected BooleanOp with implicit AND"),
        }

        assert!(!result.warnings.is_empty());
    }

    #[test]
    fn test_colon_in_field_vs_non_field_terms() {
        // valid field operations are parsed as fields
        let mut lexer = Lexer::new("url:example.com");
        let tokens = lexer.tokenize().unwrap();
        let mut parser = Parser::new(tokens).unwrap();
        let result = parser.parse().unwrap();

        match result.query.expression {
            Expression::Field { field, .. } => {
                assert_eq!(field, FieldType::Url);
            }
            _ => panic!("Expected Field operation for valid field type"),
        }

        // non-field colons get combined into a single term
        let mut lexer = Lexer::new("test:test");
        let tokens = lexer.tokenize().unwrap();
        let mut parser = Parser::new(tokens).unwrap();
        let result = parser.parse().unwrap();

        match result.query.expression {
            Expression::Term {
                term: Term::Word { value },
                ..
            } => {
                assert_eq!(value, "test:test");
            }
            _ => panic!("Expected Term with combined colon value for non-field"),
        }

        // quoted value after non-field colon
        let mut lexer = Lexer::new("protocol:\"https\"");
        let tokens = lexer.tokenize().unwrap();
        let mut parser = Parser::new(tokens).unwrap();
        let result = parser.parse().unwrap();

        match result.query.expression {
            Expression::Term {
                term: Term::Word { value },
                ..
            } => {
                assert_eq!(value, "protocol:\"https\"");
            }
            _ => panic!("Expected Term with combined colon and quoted value"),
        }
    }
}
