use std::fmt;

use crate::error::{LintError, LintResult, Position, Span};
#[derive(Debug, Clone, PartialEq)]
pub enum TokenType {
    Word(String),
    QuotedString(String),
    Number(String),

    And,
    Or,
    Not,
    LeftParen,
    RightParen,
    LeftBracket,
    RightBracket,
    LeftBrace,
    RightBrace,

    Tilde,
    Colon,
    Question,
    Asterisk,
    To,

    Near(u32),
    NearForward(u32),

    CommentStart,
    CommentEnd,

    Field(String),

    Hashtag(String),
    Mention(String),

    Whitespace,

    Eof,
}

impl fmt::Display for TokenType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TokenType::Word(w) => write!(f, "word '{w}'"),
            TokenType::QuotedString(s) => write!(f, "quoted string '{s}'"),
            TokenType::Number(n) => write!(f, "number '{n}'"),
            TokenType::And => write!(f, "AND"),
            TokenType::Or => write!(f, "OR"),
            TokenType::Not => write!(f, "NOT"),
            TokenType::LeftParen => write!(f, "("),
            TokenType::RightParen => write!(f, ")"),
            TokenType::LeftBracket => write!(f, "["),
            TokenType::RightBracket => write!(f, "]"),
            TokenType::LeftBrace => write!(f, "{{"),
            TokenType::RightBrace => write!(f, "}}"),
            TokenType::Tilde => write!(f, "~"),
            TokenType::Colon => write!(f, ":"),
            TokenType::Question => write!(f, "?"),
            TokenType::Asterisk => write!(f, "*"),
            TokenType::To => write!(f, "TO"),
            TokenType::Near(n) => write!(f, "NEAR/{n}"),
            TokenType::NearForward(n) => write!(f, "NEAR/{n}f"),
            TokenType::CommentStart => write!(f, "<<<"),
            TokenType::CommentEnd => write!(f, ">>>"),
            TokenType::Field(f_name) => write!(f, "field '{f_name}'"),
            TokenType::Hashtag(h) => write!(f, "hashtag '{h}'"),
            TokenType::Mention(m) => write!(f, "mention '{m}'"),
            TokenType::Whitespace => write!(f, "whitespace"),
            TokenType::Eof => write!(f, "end of file"),
        }
    }
}

/// a token with position information
#[derive(Debug, Clone, PartialEq)]
pub struct Token {
    pub token_type: TokenType,
    pub span: Span,
    pub raw: String,
}

impl Token {
    pub fn new(token_type: TokenType, span: Span, raw: String) -> Self {
        Self {
            token_type,
            span,
            raw,
        }
    }
}

/// lexer for tokenizing  queries
pub struct Lexer {
    input: Vec<char>,
    position: usize,
    line: usize,
    column: usize,
    inside_comment: bool,
}

impl Lexer {
    /// Character classification helpers
    fn is_word_char(&self, ch: char) -> bool {
        // Allow all Unicode letters and numbers, plus specific ASCII symbols
        ch.is_alphabetic() || ch.is_numeric()
            || ch == '_'
            || ch == '.'
            || ch == '-'
            || ch == '/'
            || ch == '*'
            || ch == '?'
            || ch == '$'
            || ch == '&'
            || ch == '#'
            || ch == '+'
            || ch == '%'
            || ch == '='
            || ch == '`'
            || ch == '|'
            || ch == '@'
            || ch == '\''
            // Allow most Unicode characters that are not ASCII control or punctuation
            || (!ch.is_ascii() && !ch.is_control() && !matches!(ch, '(' | ')' | '[' | ']' | '{' | '}' | ':' | '~' | '"' | '<' | '>'))
    }

    fn is_word_boundary_char(&self, ch: char) -> bool {
        ch.is_whitespace()
            || matches!(
                ch,
                '(' | ')' | '[' | ']' | '{' | '}' | ':' | '~' | '"' | '#' | '@' | '<' | '>'
            )
    }

    fn handle_comment_transition(&mut self, ch: char) -> LintResult<Option<Token>> {
        if self.inside_comment {
            if ch == '>' && self.peek_ahead(2) == ">>" {
                self.inside_comment = false;
                return self.read_comment_end();
            } else {
                // Skip any character inside comments
                self.advance_with_position_tracking(ch);
                return self.next_token();
            }
        } else if ch == '<' && self.peek_ahead(2) == "<<" {
            return self.read_comment_start();
        } else if ch == '>' && self.peek_ahead(2) == ">>" {
            return self.read_comment_end();
        }

        Ok(None)
    }

    fn advance_with_position_tracking(&mut self, ch: char) {
        self.advance();
        if ch == '\n' {
            self.line += 1;
            self.column = 1;
        } else {
            self.column += 1;
        }
    }
    pub fn new(input: &str) -> Self {
        Self {
            input: input.chars().collect(),
            position: 0,
            line: 1,
            column: 1,
            inside_comment: false,
        }
    }

    pub fn tokenize(&mut self) -> LintResult<Vec<Token>> {
        let mut tokens = Vec::new();

        while !self.is_at_end() {
            match self.next_token()? {
                Some(token) => {
                    if !matches!(token.token_type, TokenType::Whitespace) {
                        tokens.push(token);
                    }
                }
                None => break,
            }
        }

        let eof_pos = self.current_position();
        tokens.push(Token::new(
            TokenType::Eof,
            Span::single(eof_pos),
            String::new(),
        ));

        Ok(tokens)
    }

    fn next_token(&mut self) -> LintResult<Option<Token>> {
        if self.is_at_end() {
            return Ok(None);
        }

        let start_pos = self.current_position();
        let ch = self.current_char();

        if let Some(token) = self.handle_comment_transition(ch)? {
            return Ok(Some(token));
        }

        // If we're inside a comment and didn't find an end marker, continue
        if self.inside_comment {
            return self.next_token();
        }

        match ch {
            ' ' | '\t' | '\r' | '\n' => {
                self.advance();
                if ch == '\n' {
                    self.line += 1;
                    self.column = 1;
                } else {
                    self.column += 1;
                }
                let end_pos = self.current_position();
                Ok(Some(Token::new(
                    TokenType::Whitespace,
                    Span::new(start_pos, end_pos),
                    ch.to_string(),
                )))
            }

            '"' => self.read_quoted_string(),

            '(' => {
                self.advance();
                self.column += 1;
                Ok(Some(Token::new(
                    TokenType::LeftParen,
                    Span::new(start_pos, self.current_position()),
                    "(".to_string(),
                )))
            }
            ')' => {
                self.advance();
                self.column += 1;
                Ok(Some(Token::new(
                    TokenType::RightParen,
                    Span::new(start_pos, self.current_position()),
                    ")".to_string(),
                )))
            }
            '[' => {
                self.advance();
                self.column += 1;
                Ok(Some(Token::new(
                    TokenType::LeftBracket,
                    Span::new(start_pos, self.current_position()),
                    "[".to_string(),
                )))
            }
            ']' => {
                self.advance();
                self.column += 1;
                Ok(Some(Token::new(
                    TokenType::RightBracket,
                    Span::new(start_pos, self.current_position()),
                    "]".to_string(),
                )))
            }
            '{' => {
                self.advance();
                self.column += 1;
                Ok(Some(Token::new(
                    TokenType::LeftBrace,
                    Span::new(start_pos, self.current_position()),
                    "{".to_string(),
                )))
            }
            '}' => {
                self.advance();
                self.column += 1;
                Ok(Some(Token::new(
                    TokenType::RightBrace,
                    Span::new(start_pos, self.current_position()),
                    "}".to_string(),
                )))
            }

            '~' => {
                self.advance();
                self.column += 1;

                // check if tilde is followed by a number and then invalid characters
                let tilde_end = self.current_position();
                if !self.is_at_end() && self.current_char().is_ascii_digit() {
                    while !self.is_at_end() && self.current_char().is_ascii_digit() {
                        self.advance();
                        self.column += 1;
                    }

                    if !self.is_at_end()
                        && !self.current_char().is_whitespace()
                        && !matches!(
                            self.current_char(),
                            '(' | ')' | '[' | ']' | ':' | '~' | '"' | '#' | '@' | '<' | '>'
                        )
                        && !self.current_char().is_ascii_digit()
                    {
                        return Err(LintError::LexerError {
                            span: Span::single_character(start_pos),
                            message: "Invalid characters after proximity operator. Tilde operator format should be ~5 (with proper word boundary).".to_string(),
                        });
                    }

                    self.position -= self.current_position().offset - tilde_end.offset;
                    self.column = tilde_end.column;
                }

                Ok(Some(Token::new(
                    TokenType::Tilde,
                    Span::new(start_pos, tilde_end),
                    "~".to_string(),
                )))
            }
            ':' => {
                // Check for a space before colon - always fail if there's a space before
                if self.position > 0 && self.input[self.position - 1].is_whitespace() {
                    return Err(LintError::InvalidFieldOperatorSpacing {
                        span: Span::single_character(start_pos),
                        message: "Field operator colon must be directly attached to the field name. If the colon is a search term, you'll need to put it in quote marks".to_string(),
                    });
                }

                self.advance();
                self.column += 1;
                Ok(Some(Token::new(
                    TokenType::Colon,
                    Span::new(start_pos, self.current_position()),
                    ":".to_string(),
                )))
            }

            '#' => self.read_hashtag(),

            '@' => self.read_mention(),

            _ if ch.is_ascii_digit() || ch == '-' => {
                // look ahead to see if this is actually an alphanumeric word starting with digits
                if (ch.is_ascii_digit() || ch == '-') && self.has_word_chars_ahead() {
                    self.read_word_or_operator()
                } else {
                    self.read_number()
                }
            }
            _ if self.is_word_char(ch) => self.read_word_or_operator(),

            _ => {
                self.advance();
                self.column += 1;
                Err(LintError::LexerError {
                    span: Span::single_character(start_pos),
                    message: format!("Unexpected character '{ch}'"),
                })
            }
        }
    }

    fn read_quoted_string(&mut self) -> LintResult<Option<Token>> {
        let start_pos = self.current_position();
        let mut value = String::new();
        let mut raw = String::new();

        raw.push(self.current_char());
        self.advance();
        self.column += 1;

        while !self.is_at_end() && self.current_char() != '"' {
            let ch = self.current_char();
            value.push(ch);
            raw.push(ch);

            if ch == '\n' {
                self.line += 1;
                self.column = 1;
            } else {
                self.column += 1;
            }

            self.advance();
        }

        if self.is_at_end() {
            return Err(LintError::LexerError {
                span: Span::single_character(start_pos),
                message: "Unterminated quoted string".to_string(),
            });
        }

        raw.push(self.current_char());
        self.advance();
        self.column += 1;

        let end_pos = self.current_position();
        Ok(Some(Token::new(
            TokenType::QuotedString(value),
            Span::new(start_pos, end_pos),
            raw,
        )))
    }

    fn read_word_or_operator(&mut self) -> LintResult<Option<Token>> {
        let start_pos = self.current_position();
        let mut value = String::new();

        while !self.is_at_end() && self.is_word_char(self.current_char()) {
            value.push(self.current_char());
            self.advance();
            self.column += 1;
        }

        let end_pos = self.current_position();
        let span = Span::new(start_pos, end_pos);

        let token_type = match value.as_str() {
            "AND" => TokenType::And,
            "OR" => TokenType::Or,
            "NOT" => TokenType::Not,
            "TO" => TokenType::To,
            _ => {
                if let Some(stripped) = value.strip_prefix("NEAR/") {
                    if value.ends_with('f') && value.len() > 6 {
                        let distance_str = &stripped[..stripped.len() - 1];
                        if let Ok(distance) = distance_str.parse::<u32>() {
                            TokenType::NearForward(distance)
                        } else {
                            TokenType::Word(value.clone())
                        }
                    } else if value.len() > 5 {
                        if let Ok(distance) = stripped.parse::<u32>() {
                            TokenType::Near(distance)
                        } else {
                            TokenType::Word(value.clone())
                        }
                    } else {
                        TokenType::Word(value.clone())
                    }
                } else {
                    TokenType::Word(value.clone())
                }
            }
        };

        Ok(Some(Token::new(token_type, span, value)))
    }

    fn read_number(&mut self) -> LintResult<Option<Token>> {
        let start_pos = self.current_position();
        let mut value = String::new();

        if self.current_char() == '-' {
            value.push(self.current_char());
            self.advance();
            self.column += 1;
        }

        while !self.is_at_end()
            && (self.current_char().is_ascii_digit() || self.current_char() == '.')
        {
            value.push(self.current_char());
            self.advance();
            self.column += 1;
        }

        let end_pos = self.current_position();
        Ok(Some(Token::new(
            TokenType::Number(value.clone()),
            Span::new(start_pos, end_pos),
            value,
        )))
    }

    fn read_hashtag(&mut self) -> LintResult<Option<Token>> {
        let start_pos = self.current_position();
        let mut value = String::new();

        self.advance();
        self.column += 1;

        while !self.is_at_end() && self.is_word_char(self.current_char()) {
            value.push(self.current_char());
            self.advance();
            self.column += 1;
        }

        let end_pos = self.current_position();
        Ok(Some(Token::new(
            TokenType::Hashtag(value.clone()),
            Span::new(start_pos, end_pos),
            format!("#{value}"),
        )))
    }

    fn read_mention(&mut self) -> LintResult<Option<Token>> {
        let start_pos = self.current_position();
        let mut value = String::new();

        self.advance();
        self.column += 1;

        while !self.is_at_end() && self.is_word_char(self.current_char()) {
            value.push(self.current_char());
            self.advance();
            self.column += 1;
        }

        let end_pos = self.current_position();
        Ok(Some(Token::new(
            TokenType::Mention(value.clone()),
            Span::new(start_pos, end_pos),
            format!("@{value}"),
        )))
    }

    fn read_comment_start(&mut self) -> LintResult<Option<Token>> {
        let start_pos = self.current_position();

        self.advance();
        self.advance();
        self.advance();
        self.column += 3;

        self.inside_comment = true;

        let end_pos = self.current_position();
        Ok(Some(Token::new(
            TokenType::CommentStart,
            Span::new(start_pos, end_pos),
            "<<<".to_string(),
        )))
    }

    fn read_comment_end(&mut self) -> LintResult<Option<Token>> {
        let start_pos = self.current_position();

        self.advance();
        self.advance();
        self.advance();
        self.column += 3;

        let end_pos = self.current_position();
        Ok(Some(Token::new(
            TokenType::CommentEnd,
            Span::new(start_pos, end_pos),
            ">>>".to_string(),
        )))
    }

    fn current_char(&self) -> char {
        if self.is_at_end() {
            '\0'
        } else {
            self.input[self.position]
        }
    }

    fn advance(&mut self) {
        if !self.is_at_end() {
            self.position += 1;
        }
    }

    fn is_at_end(&self) -> bool {
        self.position >= self.input.len()
    }

    fn current_position(&self) -> Position {
        Position::new(self.line, self.column, self.position)
    }

    fn peek_ahead(&self, n: usize) -> String {
        let mut result = String::new();
        for i in 0..n {
            if self.position + i + 1 < self.input.len() {
                result.push(self.input[self.position + i + 1]);
            } else {
                break;
            }
        }
        result
    }

    fn has_word_chars_ahead(&self) -> bool {
        let mut pos = self.position;

        // Skip initial minus sign if present
        if pos < self.input.len() && self.input[pos] == '-' {
            pos += 1;
        }

        // Skip initial digits
        while pos < self.input.len() && self.input[pos].is_ascii_digit() {
            pos += 1;
        }

        while pos < self.input.len() {
            let ch = self.input[pos];
            if ch.is_alphabetic() || ch == '*' || ch == '?' {
                return true;
            } else if self.is_word_boundary_char(ch) {
                return false;
            } else if self.is_word_char(ch) {
                pos += 1;
            } else {
                return false;
            }
        }

        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_tokenization() {
        let mut lexer = Lexer::new("apple AND juice");
        let tokens = lexer.tokenize().unwrap();

        assert_eq!(tokens.len(), 4);
        assert!(matches!(tokens[0].token_type, TokenType::Word(ref w) if w == "apple"));
        assert!(matches!(tokens[1].token_type, TokenType::And));
        assert!(matches!(tokens[2].token_type, TokenType::Word(ref w) if w == "juice"));
        assert!(matches!(tokens[3].token_type, TokenType::Eof));
    }

    #[test]
    fn test_quoted_string() {
        let mut lexer = Lexer::new("\"apple juice\" \" phrase with spaces \"");
        let tokens = lexer.tokenize().unwrap();

        assert_eq!(tokens.len(), 3);
        assert!(
            matches!(tokens[0].token_type, TokenType::QuotedString(ref s) if s == "apple juice")
        );
        assert!(
            matches!(tokens[1].token_type, TokenType::QuotedString(ref s) if s == " phrase with spaces ")
        );
    }

    #[test]
    fn test_proximity_operators() {
        let mut lexer = Lexer::new("NEAR/5 NEAR/3f");
        let tokens = lexer.tokenize().unwrap();

        assert_eq!(tokens.len(), 3);
        assert!(matches!(tokens[0].token_type, TokenType::Near(5)));
        assert!(matches!(tokens[1].token_type, TokenType::NearForward(3)));
    }

    #[test]
    fn test_numbers_vs_words_and_special_chars() {
        let mut lexer = Lexer::new(
            "42 3.14 -5 0xcharlie 18RahulJoshi user123 $UBER U&BER uber$ 123$abc test+word word%test test=word word`test 5test|word test@word",
        );
        let tokens = lexer.tokenize().unwrap();

        assert_eq!(tokens.len(), 17); // 16 tokens + EOF

        // pure numbers
        assert!(matches!(tokens[0].token_type, TokenType::Number(ref n) if n == "42"));
        assert!(matches!(tokens[1].token_type, TokenType::Number(ref n) if n == "3.14"));
        assert!(matches!(tokens[2].token_type, TokenType::Number(ref n) if n == "-5"));

        // alphanumeric starting with digits (should be words due to has_word_chars_ahead)
        assert!(matches!(tokens[3].token_type, TokenType::Word(ref w) if w == "0xcharlie"));
        assert!(matches!(tokens[4].token_type, TokenType::Word(ref w) if w == "18RahulJoshi"));
        assert!(matches!(tokens[5].token_type, TokenType::Word(ref w) if w == "user123"));

        // original special characters
        assert!(matches!(tokens[6].token_type, TokenType::Word(ref w) if w == "$UBER"));
        assert!(matches!(tokens[7].token_type, TokenType::Word(ref w) if w == "U&BER"));
        assert!(matches!(tokens[8].token_type, TokenType::Word(ref w) if w == "uber$"));
        assert!(matches!(tokens[9].token_type, TokenType::Word(ref w) if w == "123$abc"));

        // new special characters
        assert!(matches!(tokens[10].token_type, TokenType::Word(ref w) if w == "test+word"));
        assert!(matches!(tokens[11].token_type, TokenType::Word(ref w) if w == "word%test"));
        assert!(matches!(tokens[12].token_type, TokenType::Word(ref w) if w == "test=word"));
        assert!(matches!(tokens[13].token_type, TokenType::Word(ref w) if w == "word`test"));
        assert!(matches!(tokens[14].token_type, TokenType::Word(ref w) if w == "5test|word"));
        assert!(matches!(tokens[15].token_type, TokenType::Word(ref w) if w == "test@word"));

        assert!(matches!(tokens[16].token_type, TokenType::Eof));
    }

    #[test]
    fn test_numeric_wildcards() {
        let mut lexer = Lexer::new("24* 12? 100*test");
        let tokens = lexer.tokenize().unwrap();

        assert_eq!(tokens.len(), 4); // 3 tokens + EOF

        // numbers with wildcards should be treated as words
        assert!(matches!(tokens[0].token_type, TokenType::Word(ref w) if w == "24*"));
        assert!(matches!(tokens[1].token_type, TokenType::Word(ref w) if w == "12?"));
        assert!(matches!(tokens[2].token_type, TokenType::Word(ref w) if w == "100*test"));
        assert!(matches!(tokens[3].token_type, TokenType::Eof));
    }
}
