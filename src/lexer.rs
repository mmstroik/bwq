use crate::error::{LintError, LintResult, Position, Span};
use std::fmt;

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
}

impl Lexer {
    pub fn new(input: &str) -> Self {
        Self {
            input: input.chars().collect(),
            position: 0,
            line: 1,
            column: 1,
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
                            position: start_pos,
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
                self.advance();
                self.column += 1;
                Ok(Some(Token::new(
                    TokenType::Colon,
                    Span::new(start_pos, self.current_position()),
                    ":".to_string(),
                )))
            }

            '<' if self.peek_ahead(2) == "<<" => self.read_comment_start(),

            '>' if self.peek_ahead(2) == ">>" => self.read_comment_end(),

            '#' => self.read_hashtag(),

            '@' => self.read_mention(),

            _ if ch.is_ascii_digit() || ch == '-' => {
                // look ahead to see if this is actually an alphanumeric word starting with digits
                if (ch.is_ascii_digit() || ch == '-') && self.has_letters_ahead() {
                    self.read_word_or_operator()
                } else {
                    self.read_number()
                }
            }
            _ if ch.is_alphabetic() || ch == '_' || ch == '*' || ch == '?' || ch == '$' => {
                self.read_word_or_operator()
            }

            _ => {
                self.advance();
                self.column += 1;
                Err(LintError::LexerError {
                    position: start_pos,
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
                position: start_pos,
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

        while !self.is_at_end()
            && (self.current_char().is_alphanumeric()
                || self.current_char() == '_'
                || self.current_char() == '.'
                || self.current_char() == '-'
                || self.current_char() == '/'
                || self.current_char() == '*'
                || self.current_char() == '?'
                || self.current_char() == '$')
        {
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

        while !self.is_at_end()
            && (self.current_char().is_alphanumeric()
                || self.current_char() == '_'
                || self.current_char() == '*'
                || self.current_char() == '?'
                || self.current_char() == '$')
        {
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

        while !self.is_at_end()
            && (self.current_char().is_alphanumeric()
                || self.current_char() == '_'
                || self.current_char() == '*'
                || self.current_char() == '?'
                || self.current_char() == '$')
        {
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

    fn has_letters_ahead(&self) -> bool {
        let mut pos = self.position;

        // Skip initial minus sign if present
        if pos < self.input.len() && self.input[pos] == '-' {
            pos += 1;
        }

        // Skip initial digits
        while pos < self.input.len() && self.input[pos].is_ascii_digit() {
            pos += 1;
        }

        // check if we find letters before hitting a word boundary
        while pos < self.input.len() {
            let ch = self.input[pos];
            if ch.is_alphabetic() {
                return true;
            } else if ch.is_whitespace()
                || matches!(
                    ch,
                    '(' | ')' | '[' | ']' | '{' | '}' | ':' | '"' | '#' | '@' | '<' | '>' | '~'
                )
            {
                return false;
            } else if ch.is_alphanumeric()
                || ch == '_'
                || ch == '.'
                || ch == '-'
                || ch == '/'
                || ch == '*'
                || ch == '?'
                || ch == '$'
            {
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
        let mut lexer = Lexer::new("\"apple juice\"");
        let tokens = lexer.tokenize().unwrap();

        assert_eq!(tokens.len(), 2);
        assert!(
            matches!(tokens[0].token_type, TokenType::QuotedString(ref s) if s == "apple juice")
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
            "42 3.14 -5 0xcharlie 18RahulJoshi user123 $UBER U$BER uber$ 123$abc -5$test",
        );
        let tokens = lexer.tokenize().unwrap();

        assert_eq!(tokens.len(), 12); // 11 tokens + EOF

        // Pure numbers
        assert!(matches!(tokens[0].token_type, TokenType::Number(ref n) if n == "42"));
        assert!(matches!(tokens[1].token_type, TokenType::Number(ref n) if n == "3.14"));
        assert!(matches!(tokens[2].token_type, TokenType::Number(ref n) if n == "-5"));

        // Alphanumeric starting with digits (should be words due to has_letters_ahead)
        assert!(matches!(tokens[3].token_type, TokenType::Word(ref w) if w == "0xcharlie"));
        assert!(matches!(tokens[4].token_type, TokenType::Word(ref w) if w == "18RahulJoshi"));
        assert!(matches!(tokens[5].token_type, TokenType::Word(ref w) if w == "user123"));

        // Dollar sign variations
        assert!(matches!(tokens[6].token_type, TokenType::Word(ref w) if w == "$UBER"));
        assert!(matches!(tokens[7].token_type, TokenType::Word(ref w) if w == "U$BER"));
        assert!(matches!(tokens[8].token_type, TokenType::Word(ref w) if w == "uber$"));
        assert!(matches!(tokens[9].token_type, TokenType::Word(ref w) if w == "123$abc"));
        assert!(matches!(tokens[10].token_type, TokenType::Word(ref w) if w == "-5$test"));

        assert!(matches!(tokens[11].token_type, TokenType::Eof));
    }
}
