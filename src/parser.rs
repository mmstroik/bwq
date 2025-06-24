use crate::ast::*;
use crate::error::{LintError, LintResult, LintWarning, Span};
use crate::lexer::{Token, TokenType};

/// Result type that includes both the parsed query and any parser warnings
pub struct ParseResult {
    pub query: Query,
    pub warnings: Vec<LintWarning>,
}

/// Recursive descent parser for Brandwatch boolean queries
pub struct Parser {
    tokens: Vec<Token>,
    current: usize,
    implicit_and_spans: Vec<Span>,
}

impl Parser {
    pub fn new(tokens: Vec<Token>) -> Self {
        Self { 
            tokens, 
            current: 0, 
            implicit_and_spans: Vec::new(),
        }
    }

    /// Parse the tokens into a Query AST
    pub fn parse(&mut self) -> LintResult<ParseResult> {
        let expression = self.parse_expression()?;
        let span = expression.span().clone();
        
        // Ensure we've consumed all tokens except EOF
        if !self.is_at_end() && !matches!(self.peek().token_type, TokenType::Eof) {
            return Err(LintError::UnexpectedToken {
                span: self.peek().span.clone(),
                token: self.peek().token_type.to_string(),
            });
        }

        // Generate warnings for implicit AND operations
        let mut warnings = Vec::new();
        for span in &self.implicit_and_spans {
            warnings.push(LintWarning::PotentialTypo {
                span: span.clone(),
                suggestion: "Consider using explicit 'AND' operator for clarity".to_string(),
            });
        }

        Ok(ParseResult {
            query: Query { expression, span },
            warnings,
        })
    }

    fn parse_expression(&mut self) -> LintResult<Expression> {
        let mut left = self.parse_and_expression()?;

        while {
            // Skip any comments between expressions
            self.skip_comments();
            self.match_token(&TokenType::Or)
        } {
            let operator = BooleanOperator::Or;
            let operator_span = self.previous().span.clone();
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
            // Skip any comments between expressions
            self.skip_comments();
            
            if self.match_token(&TokenType::And) {
                // Explicit AND
                let operator = BooleanOperator::And;
                let operator_span = self.previous().span.clone();
                let right = self.parse_not_expression()?;
                
                let span = Span::new(left.span().start.clone(), right.span().end.clone());
                left = Expression::BooleanOp {
                    operator,
                    left: Box::new(left),
                    right: Some(Box::new(right)),
                    span,
                };
            } else if self.is_implicit_and_candidate() {
                // Implicit AND (space-separated terms)
                let right = self.parse_not_expression()?;
                
                let span = Span::new(left.span().start.clone(), right.span().end.clone());
                left = Expression::BooleanOp {
                    operator: BooleanOperator::And,
                    left: Box::new(left),
                    right: Some(Box::new(right)),
                    span: span.clone(),
                };
                
                // Mark this for warning generation
                self.implicit_and_spans.push(span);
            } else {
                break;
            }
        }

        Ok(left)
    }

    fn parse_not_expression(&mut self) -> LintResult<Expression> {
        // Handle leading NOT operator
        if self.match_token(&TokenType::Not) {
            let operator_span = self.previous().span.clone();
            let dummy_left = Expression::Term {
                term: Term::Word { value: "".to_string() },
                span: operator_span.clone(),
            };
            let right = self.parse_proximity_expression()?;
            
            let span = Span::new(operator_span.start.clone(), right.span().end.clone());
            return Ok(Expression::BooleanOp {
                operator: BooleanOperator::Not,
                left: Box::new(dummy_left),
                right: Some(Box::new(right)),
                span,
            });
        }

        let mut left = self.parse_proximity_expression()?;

        while {
            // Skip any comments between expressions
            self.skip_comments();
            self.match_token(&TokenType::Not)
        } {
            let operator = BooleanOperator::Not;
            let operator_span = self.previous().span.clone();
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
        let mut left = self.parse_primary()?;

        // Handle proximity operators
        if self.match_token(&TokenType::Tilde) {
            let tilde_span = self.previous().span.clone();
            let mut distance = None;
            
            // Check for optional distance number
            if let TokenType::Number(num_str) = &self.peek().token_type {
                distance = num_str.parse::<u32>().ok();
                self.advance();
            }
            
            // For tilde operator, we can have either:
            // 1. "quoted phrase"~5 (single term with proximity)
            // 2. term1 ~ term2 (two terms with proximity)
            let mut terms = vec![left];
            let mut end_span = tilde_span.end.clone();
            
            // Check if there's a right-hand side term (not EOF or other operator)
            if !self.is_at_end() && 
               !matches!(self.peek().token_type, TokenType::And | TokenType::Or | TokenType::Not | 
                        TokenType::RightParen | TokenType::LeftParen) {
                if let Ok(right) = self.parse_primary() {
                    end_span = right.span().end.clone();
                    terms.push(right);
                }
            }
            
            let span = Span::new(terms[0].span().start.clone(), end_span);
            
            return Ok(Expression::Proximity {
                operator: ProximityOperator::Proximity { distance },
                terms,
                span,
            });
        }

        // Handle NEAR operators
        if let TokenType::Near(distance) = &self.peek().token_type {
            let distance = *distance;
            self.advance();
            let operator_span = self.previous().span.clone();
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
            let operator_span = self.previous().span.clone();
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
        // Handle parenthesized expressions
        if self.match_token(&TokenType::LeftParen) {
            let start_span = self.previous().span.clone();
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
            
            return Ok(Expression::Group {
                expression: Box::new(expr),
                span,
            });
        }

        // Handle case-sensitive terms {word}
        if self.match_token(&TokenType::LeftBrace) {
            let start_span = self.previous().span.clone();
            
            if let TokenType::Word(word) = &self.peek().token_type {
                let word = word.clone();
                self.advance();
                let word_span = self.previous().span.clone();
                
                if !self.match_token(&TokenType::RightBrace) {
                    return Err(LintError::ExpectedToken {
                        span: self.peek().span.clone(),
                        expected: "}".to_string(),
                        found: self.peek().token_type.to_string(),
                    });
                }
                
                let end_span = self.previous().span.clone();
                let span = Span::new(start_span.start, end_span.end);
                
                return Ok(Expression::Term {
                    term: Term::CaseSensitive { value: word },
                    span,
                });
            } else {
                return Err(LintError::ExpectedToken {
                    span: self.peek().span.clone(),
                    expected: "word".to_string(),
                    found: self.peek().token_type.to_string(),
                });
            }
        }

        // Handle ranges [x TO y]
        if self.match_token(&TokenType::LeftBracket) {
            return self.parse_range();
        }

        // Handle comments <<<text>>>
        if self.match_token(&TokenType::CommentStart) {
            return self.parse_comment();
        }

        // Handle field operations
        if let TokenType::Word(word) = &self.peek().token_type {
            let word = word.clone();
            let word_span = self.peek().span.clone();
            
            // Look ahead for colon to determine if this is a field operation
            if self.peek_ahead(1).map(|t| &t.token_type) == Some(&TokenType::Colon) {
                self.advance(); // consume field name
                self.advance(); // consume colon
                
                // Always parse field operations, even for unknown fields
                // Let validation catch unknown fields later
                let value = Box::new(self.parse_primary()?);
                
                // If the value is a range, associate the field with it
                let value = if let Expression::Range { start, end, span: range_span, .. } = value.as_ref() {
                    if let Some(field_type) = FieldType::from_str(&word) {
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
                
                // Create expression with known or unknown field
                if let Some(field_type) = FieldType::from_str(&word) {
                    return Ok(Expression::Field {
                        field: field_type,
                        value,
                        span,
                    });
                } else {
                    // For unknown fields, create a generic term and let validator handle it
                    return Ok(Expression::Term {
                        term: Term::Word { value: format!("{}:{}", word, match value.as_ref() {
                            Expression::Term { term: Term::Word { value }, .. } => value.clone(),
                            Expression::Term { term: Term::Phrase { value }, .. } => format!("\"{}\"", value),
                            _ => "unknown".to_string(),
                        })},
                        span,
                    });
                }
            }
        }

        // Handle regular terms
        self.parse_term()
    }

    fn parse_range(&mut self) -> LintResult<Expression> {
        let start_span = self.previous().span.clone(); // [
        
        // Parse start value
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

        // Expect TO
        if !self.match_token(&TokenType::To) {
            return Err(LintError::ExpectedToken {
                span: self.peek().span.clone(),
                expected: "TO".to_string(),
                found: self.peek().token_type.to_string(),
            });
        }

        // Parse end value
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

        // Expect ]
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
            field: None, // Will be handled by field parsing if needed
            start: start_value,
            end: end_value,
            span,
        })
    }

    fn parse_comment(&mut self) -> LintResult<Expression> {
        let start_span = self.previous().span.clone(); // <<<
        let mut comment_text = String::new();

        // Collect all text until >>>
        while !self.is_at_end() && !matches!(self.peek().token_type, TokenType::CommentEnd) {
            match &self.peek().token_type {
                TokenType::Word(w) => comment_text.push_str(w),
                TokenType::QuotedString(s) => comment_text.push_str(&format!("\"{}\"", s)),
                TokenType::Number(n) => comment_text.push_str(n),
                TokenType::Whitespace => comment_text.push(' '),
                _ => comment_text.push_str(&self.peek().raw),
            }
            self.advance();
        }

        if !self.match_token(&TokenType::CommentEnd) {
            return Err(LintError::ExpectedToken {
                span: self.peek().span.clone(),
                expected: ">>>".to_string(),
                found: self.peek().token_type.to_string(),
            });
        }

        let end_span = self.previous().span.clone();
        let span = Span::new(start_span.start, end_span.end);

        Ok(Expression::Comment {
            text: comment_text.trim().to_string(),
            span,
        })
    }

    fn parse_term(&mut self) -> LintResult<Expression> {
        let token = self.peek().clone();
        
        match &token.token_type {
            TokenType::Word(word) => {
                self.advance();
                let term = if word.contains('*') {
                    Term::Wildcard { value: word.clone() }
                } else if word.contains('?') {
                    Term::Replacement { value: word.clone() }
                } else {
                    Term::Word { value: word.clone() }
                };
                
                Ok(Expression::Term {
                    term,
                    span: token.span,
                })
            }
            TokenType::QuotedString(string) => {
                self.advance();
                Ok(Expression::Term {
                    term: Term::Phrase { value: string.clone() },
                    span: token.span,
                })
            }
            TokenType::Number(number) => {
                self.advance();
                Ok(Expression::Term {
                    term: Term::Word { value: number.clone() },
                    span: token.span,
                })
            }
            TokenType::Hashtag(hashtag) => {
                self.advance();
                Ok(Expression::Term {
                    term: Term::Hashtag { value: hashtag.clone() },
                    span: token.span,
                })
            }
            TokenType::Mention(mention) => {
                self.advance();
                Ok(Expression::Term {
                    term: Term::Mention { value: mention.clone() },
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

        // Check if current token could start a new term/expression
        // but exclude operators that would end the current expression
        match &self.peek().token_type {
            // These tokens can start new terms (implicit AND candidates)
            TokenType::Word(_) |
            TokenType::QuotedString(_) |
            TokenType::Number(_) |
            TokenType::Hashtag(_) |
            TokenType::Mention(_) |
            TokenType::LeftParen |
            TokenType::LeftBrace => true,
            
            // These tokens end the current expression level
            TokenType::Or |
            TokenType::RightParen |
            TokenType::RightBracket |
            TokenType::RightBrace |
            TokenType::Eof => false,
            
            // AND and NOT are handled explicitly in their respective parsing methods
            TokenType::And |
            TokenType::Not => false,
            
            // NEAR operators and other special cases
            _ => false,
        }
    }

    /// Skip any comments at the current position
    fn skip_comments(&mut self) {
        while self.match_token(&TokenType::CommentStart) {
            // Consume comment text until we find the end
            while !self.is_at_end() && !matches!(self.peek().token_type, TokenType::CommentEnd) {
                self.advance();
            }
            // Consume the closing >>>
            self.match_token(&TokenType::CommentEnd);
        }
    }
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
            Expression::Comment { span, .. } => span,
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
        let mut parser = Parser::new(tokens);
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
        let mut parser = Parser::new(tokens);
        let result = parser.parse().unwrap();
        
        match result.query.expression {
            Expression::Term { term: Term::Phrase { value }, .. } => {
                assert_eq!(value, "apple juice");
            }
            _ => panic!("Expected Term with Phrase"),
        }
    }

    #[test]
    fn test_field_operation() {
        let mut lexer = Lexer::new("title:\"apple juice\"");
        let tokens = lexer.tokenize().unwrap();
        let mut parser = Parser::new(tokens);
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
        let mut parser = Parser::new(tokens);
        let result = parser.parse().unwrap();
        
        // Should parse as implicit AND
        match result.query.expression {
            Expression::BooleanOp { operator, .. } => {
                assert_eq!(operator, BooleanOperator::And);
            }
            _ => panic!("Expected BooleanOp with implicit AND"),
        }
        
        // Should have warning about implicit AND
        assert!(!result.warnings.is_empty());
    }
}