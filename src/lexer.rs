use crate::error::{BccError, Span};
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq)]
pub enum TokenType {
    // Single-character tokens
    LeftParen,
    RightParen,
    LeftBrace,
    RightBrace,
    LeftBracket,
    RightBracket,
    Comma,
    Colon,
    Dot,
    Minus,
    Plus,
    Semicolon,
    Slash,
    Star,

    // One or two character tokens
    Bang,
    BangEqual,
    Equal,
    EqualEqual,
    Greater,
    GreaterEqual,
    Less,
    LessEqual,

    // Literals
    Identifier,
    String,
    Integer,
    Double,

    // Keywords
    And,
    Else,
    False,
    For,
    Fun,
    If,
    In,
    Nil,
    Not,
    Or,
    Return,
    True,
    While,

    // Special
    Eof,
}

/// Simplified token using owned strings for better maintainability
#[derive(Debug, Clone)]
pub struct Token {
    pub token_type: TokenType,
    pub lexeme: String,  // Simplified: owned string for better maintainability
    pub span: Span,
}

impl Token {
    pub fn new(token_type: TokenType, lexeme: String, span: Span) -> Self {
        Self {
            token_type,
            lexeme,
            span,
        }
    }
}

pub struct Lexer {
    source: String,  // Simplified: owned string for better maintainability
    tokens: Vec<Token>,
    start: usize,
    current: usize,
    keywords: HashMap<&'static str, TokenType>,
}

impl Lexer {
    pub fn new(source: String) -> Self {
        let mut keywords = HashMap::new();
        keywords.insert("and", TokenType::And);
        keywords.insert("else", TokenType::Else);
        keywords.insert("false", TokenType::False);
        keywords.insert("for", TokenType::For);
        keywords.insert("fun", TokenType::Fun);
        keywords.insert("if", TokenType::If);
        keywords.insert("in", TokenType::In);
        keywords.insert("nil", TokenType::Nil);
        keywords.insert("not", TokenType::Not);
        keywords.insert("or", TokenType::Or);
        keywords.insert("return", TokenType::Return);
        keywords.insert("true", TokenType::True);
        keywords.insert("while", TokenType::While);

        Self {
            source,
            tokens: Vec::new(),
            start: 0,
            current: 0,
            keywords,
        }
    }

    pub fn scan_tokens(&mut self) -> Result<Vec<Token>, BccError> {
        while !self.is_at_end() {
            self.start = self.current;
            self.scan_token()?;
        }

        self.tokens.push(Token::new(
            TokenType::Eof,
            "".to_string(),
            Span::single(self.current),
        ));

        Ok(self.tokens.clone())
    }

    fn is_at_end(&self) -> bool {
        self.current >= self.source.len()
    }

    fn scan_token(&mut self) -> Result<(), BccError> {
        let c = self.advance();

        match c {
            '(' => self.add_token(TokenType::LeftParen),
            ')' => self.add_token(TokenType::RightParen),
            '{' => self.add_token(TokenType::LeftBrace),
            '}' => self.add_token(TokenType::RightBrace),
            '[' => self.add_token(TokenType::LeftBracket),
            ']' => self.add_token(TokenType::RightBracket),
            ',' => self.add_token(TokenType::Comma),
            ':' => self.add_token(TokenType::Colon),
            '.' => self.add_token(TokenType::Dot),
            '-' => self.add_token(TokenType::Minus),
            '+' => self.add_token(TokenType::Plus),
            ';' => self.add_token(TokenType::Semicolon),
            '*' => self.add_token(TokenType::Star),
            '!' => {
                let token_type = if self.match_char('=') {
                    TokenType::BangEqual
                } else {
                    TokenType::Bang
                };
                self.add_token(token_type);
            }
            '=' => {
                let token_type = if self.match_char('=') {
                    TokenType::EqualEqual
                } else {
                    TokenType::Equal
                };
                self.add_token(token_type);
            }
            '<' => {
                let token_type = if self.match_char('=') {
                    TokenType::LessEqual
                } else {
                    TokenType::Less
                };
                self.add_token(token_type);
            }
            '>' => {
                let token_type = if self.match_char('=') {
                    TokenType::GreaterEqual
                } else {
                    TokenType::Greater
                };
                self.add_token(token_type);
            }
            '/' => {
                if self.match_char('/') {
                    // Comment goes until end of line
                    while self.peek() != '\n' && !self.is_at_end() {
                        self.advance();
                    }
                } else {
                    self.add_token(TokenType::Slash);
                }
            }
            ' ' | '\r' | '\t' => {
                // Ignore whitespace
            }
            '\n' => {
                // Ignore newlines but track them for line numbers if needed later
            }
            '"' => self.string()?,
            c if c.is_ascii_digit() => self.number()?,
            c if c.is_alphabetic() || c == '_' => self.identifier(),
            _ => {
                return Err(BccError::lex_error(
                    Span::single(self.current - 1),
                    format!("Unexpected character: '{}'", c),
                ));
            }
        }

        Ok(())
    }

    fn advance(&mut self) -> char {
        if self.current >= self.source.len() {
            return '\0';
        }
        
        let c = self.source.chars().nth(self.char_count()).unwrap_or('\0');
        self.current += c.len_utf8();
        c
    }

    fn match_char(&mut self, expected: char) -> bool {
        if self.is_at_end() || self.peek() != expected {
            false
        } else {
            self.advance();
            true
        }
    }

    fn peek(&self) -> char {
        if self.current >= self.source.len() {
            return '\0';
        }
        self.source.chars().nth(self.char_count()).unwrap_or('\0')
    }

    fn peek_next(&self) -> char {
        let char_pos = self.char_count();
        if char_pos + 1 >= self.source.chars().count() {
            return '\0';
        }
        self.source.chars().nth(char_pos + 1).unwrap_or('\0')
    }

    fn char_count(&self) -> usize {
        self.source[..self.current].chars().count()
    }

    fn string(&mut self) -> Result<(), BccError> {
        while self.peek() != '"' && !self.is_at_end() {
            self.advance();
        }

        if self.is_at_end() {
            return Err(BccError::lex_error(
                Span::new(self.start, self.current),
                "Unterminated string".to_string(),
            ));
        }

        // Consume the closing "
        self.advance();

        // The string value is the slice between the quotes
        let start_content = self.start + 1; // Skip opening quote
        let end_content = self.current - 1; // Skip closing quote
        let string_slice = &self.source[start_content..end_content];
        
        self.add_token_with_content(TokenType::String, string_slice.to_string());
        Ok(())
    }

    fn number(&mut self) -> Result<(), BccError> {
        while self.peek().is_ascii_digit() {
            self.advance();
        }

        let mut is_double = false;
        
        // Look for fractional part
        if self.peek() == '.' && self.peek_next().is_ascii_digit() {
            is_double = true;
            // Consume the "."
            self.advance();

            while self.peek().is_ascii_digit() {
                self.advance();
            }
        }

        let number_slice = &self.source[self.start..self.current];
        
        if is_double {
            // Validate the double
            if number_slice.parse::<f64>().is_err() {
                return Err(BccError::lex_error(
                    Span::new(self.start, self.current),
                    format!("Invalid double: {}", number_slice),
                ));
            }
            self.add_token_with_content(TokenType::Double, number_slice.to_string());
        } else {
            // Validate the integer
            if number_slice.parse::<i64>().is_err() {
                return Err(BccError::lex_error(
                    Span::new(self.start, self.current),
                    format!("Invalid integer: {}", number_slice),
                ));
            }
            self.add_token_with_content(TokenType::Integer, number_slice.to_string());
        }
        
        Ok(())
    }

    fn identifier(&mut self) {
        while self.peek().is_alphanumeric() || self.peek() == '_' {
            self.advance();
        }

        let text = &self.source[self.start..self.current];
        let token_type = self
            .keywords
            .get(text)
            .cloned()
            .unwrap_or(TokenType::Identifier);

        self.add_token(token_type);
    }

    fn add_token(&mut self, token_type: TokenType) {
        let text = &self.source[self.start..self.current];
        self.add_token_with_content(token_type, text.to_string());
    }

    fn add_token_with_content(&mut self, token_type: TokenType, lexeme: String) {
        self.tokens.push(Token::new(
            token_type,
            lexeme,
            Span::new(self.start, self.current),
        ));
    }
}