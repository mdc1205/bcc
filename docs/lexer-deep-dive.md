# The Lexer: A Deep Dive

The lexer (also known as a tokenizer or scanner) is the first phase of the BCC interpreter. It transforms raw source code text into a sequence of meaningful tokens that the parser can work with. This document provides a comprehensive, function-by-function overview of how our lexer works.

## Table of Contents
1. [Overview](#overview)
2. [Core Data Structures](#core-data-structures)
3. [Function-by-Function Analysis](#function-by-function-analysis)
4. [Token Recognition Patterns](#token-recognition-patterns)
5. [Error Handling](#error-handling)
6. [Design Decisions](#design-decisions)

## Overview

The BCC lexer follows a simple but effective design pattern: it scans through source code character by character, identifying patterns and converting them into tokens. Unlike some languages that have complex lookahead requirements, our Lox-like language is designed to be lexically simple, making the lexer straightforward to implement and understand.

**Key characteristics:**
- Single-character lookahead (with `peek_next()` for numbers)
- Keyword recognition through hash map lookup
- Separate integer and double types
- Comprehensive error reporting with position information

## Core Data Structures

### TokenType Enum

```rust
#[derive(Debug, Clone, PartialEq)]
pub enum TokenType {
    // Single-character tokens
    LeftParen, RightParen, LeftBrace, RightBrace,
    Comma, Dot, Minus, Plus, Semicolon, Slash, Star,

    // One or two character tokens
    Bang, BangEqual, Equal, EqualEqual,
    Greater, GreaterEqual, Less, LessEqual,

    // Literals
    Identifier, String, Integer, Double,

    // Keywords
    And, Else, False, For, Fun, If, Nil, Or,
    Print, Return, True, While,

    // Special
    Eof,
}
```

The `TokenType` enum represents every possible token in our language. Notice how it's organized by complexity: single characters first, then multi-character operators, then complex tokens like literals.

### Token Structure

```rust
#[derive(Debug, Clone)]
pub struct Token {
    pub token_type: TokenType,
    pub lexeme: String,      // Owned string for simplicity and maintainability
    pub span: Span,          // Position information for errors
}
```

Each token contains:
- **token_type**: What kind of token this is
- **lexeme**: The original text as an owned string (e.g., `"42"` for an integer)
- **span**: Start and end positions for error reporting

### Lexer State

```rust
pub struct Lexer<'src> {
    source: &'src str,              // Reference to source string
    chars: std::str::Chars<'src>,   // Character iterator
    tokens: Vec<Token>,             // Collected tokens
    start: usize,                   // Start of current token
    current: usize,                 // Current character position
    keywords: HashMap<&'static str, TokenType>, // Keyword lookup table
}
```

The lexer maintains minimal state:
- **source**: Reference to original source string
- **chars**: Character iterator for traversal
- **tokens**: Building the result list
- **start/current**: Track the current token being built
- **keywords**: Fast keyword lookup with string literals

## Function-by-Function Analysis

### Constructor: `new(source: String) -> Self`

```rust
pub fn new(source: String) -> Self {
    let mut keywords = HashMap::new();
    keywords.insert("and", TokenType::And);
    keywords.insert("else", TokenType::Else);
    // ... more keywords ...

    Self {
        source,
        chars: source.chars(),
        tokens: Vec::new(),
        start: 0,
        current: 0,
        keywords,
    }
}
```

**Purpose**: Initializes a new lexer with the source code.

**Key decisions:**
- Pre-populates keyword hash map for O(1) lookup  
- Uses string slices in keyword table for efficiency
- Maintains reference to source string for position tracking
- Uses character iterator for safe Unicode traversal

**Why character iterator?** Rust strings are UTF-8, so the character iterator ensures proper Unicode handling while maintaining simplicity.

### Main Entry Point: `scan_tokens(&mut self) -> Result<Vec<Token>, BccError>`

```rust
pub fn scan_tokens(&mut self) -> Result<Vec<Token>, BccError> {
    while !self.is_at_end() {
        self.start = self.current;  // Mark start of new token
        self.scan_token()?;         // Process one token
    }

    // Add EOF token
    self.tokens.push(Token::new(
        TokenType::Eof,
        "".to_string(),
        Span::single(self.current),
    ));

    Ok(self.tokens.clone())
}
```

**Purpose**: Main driver that processes the entire source file.

**Algorithm**:
1. Reset `start` to current position (beginning of new token)
2. Scan one complete token
3. Repeat until end of file
4. Add special EOF token for parser convenience

**Design note**: The EOF token simplifies parser logic by providing a clear end marker.

### Core Logic: `scan_token(&mut self) -> Result<(), BccError>`

This is the heart of the lexer. It reads one character and decides what to do:

```rust
fn scan_token(&mut self) -> Result<(), BccError> {
    let c = self.advance();  // Get next character

    match c {
        '(' => self.add_token(TokenType::LeftParen),
        ')' => self.add_token(TokenType::RightParen),
        // ... single character tokens ...
        
        '!' => {
            let token_type = if self.match_char('=') {
                TokenType::BangEqual
            } else {
                TokenType::Bang
            };
            self.add_token(token_type);
        }
        // ... other multi-character tokens ...
        
        '/' => {
            if self.match_char('/') {
                // Comment: consume until end of line
                while self.peek() != '\n' && !self.is_at_end() {
                    self.advance();
                }
            } else {
                self.add_token(TokenType::Slash);
            }
        }
        
        ' ' | '\r' | '\t' | '\n' => {
            // Ignore whitespace
        }
        
        '"' => self.string()?,
        c if c.is_ascii_digit() => self.number()?,
        c if c.is_ascii_alphabetic() || c == '_' => self.identifier(),
        
        _ => {
            return Err(BccError::lex_error(
                Span::single(self.current - 1),
                format!("Unexpected character: '{}'", c),
            ));
        }
    }
    Ok(())
}
```

**Purpose**: Determines token type and delegates to appropriate handler.

**Pattern**: Uses Rust's powerful pattern matching to:
1. Handle simple single-character tokens directly
2. Check for multi-character operators with `match_char()`
3. Handle comments (special case of `/`)
4. Ignore whitespace
5. Delegate complex tokens to specialized functions
6. Provide clear error messages for invalid characters

### Character Navigation: `advance()`, `peek()`, `peek_next()`

```rust
fn advance(&mut self) -> char {
    let c = self.source[self.current];
    self.current += 1;
    c
}

fn peek(&self) -> char {
    if self.is_at_end() {
        '\0'
    } else {
        self.source[self.current]
    }
}

fn peek_next(&self) -> char {
    if self.current + 1 >= self.source.len() {
        '\0'
    } else {
        self.source[self.current + 1]
    }
}
```

**Purpose**: Safe character access without bounds checking errors.

**Design patterns**:
- `advance()`: Consumes a character (moves cursor forward)
- `peek()`: Looks at current character without consuming
- `peek_next()`: Looks ahead one character (needed for number parsing)
- All return `'\0'` for end-of-file (C-style null terminator convention)

### Multi-Character Recognition: `match_char(expected: char) -> bool`

```rust
fn match_char(&mut self, expected: char) -> bool {
    if self.is_at_end() || self.source[self.current] != expected {
        false
    } else {
        self.current += 1;
        true
    }
}
```

**Purpose**: Conditionally consume a character if it matches expectation.

**Usage pattern**: Perfect for operators like `!=`, `<=`, `>=`, `==`:
```rust
'!' => {
    let token_type = if self.match_char('=') {
        TokenType::BangEqual  // Found "!="
    } else {
        TokenType::Bang       // Just "!"
    };
    self.add_token(token_type);
}
```

### String Parsing: `string() -> Result<(), BccError>`

```rust
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

    // Get the string value without the surrounding quotes
    let value: String = self.source[self.start + 1..self.current - 1]
        .iter()
        .collect();
    
    self.add_token_with_literal(TokenType::String, Some(value));
    Ok(())
}
```

**Purpose**: Parse string literals enclosed in double quotes.

**Algorithm**:
1. Skip characters until closing quote or end of file
2. Error if unterminated
3. Consume closing quote
4. Extract content between quotes (excluding the quotes themselves)
5. Store the actual string content as the lexeme

**Error handling**: Provides clear error message with span information for unterminated strings.

### Number Parsing: `number() -> Result<(), BccError>`

```rust
fn number(&mut self) -> Result<(), BccError> {
    // Parse integer part
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

    let value: String = self.source[self.start..self.current].iter().collect();
    
    if is_double {
        match value.parse::<f64>() {
            Ok(_) => {
                self.add_token_with_literal(TokenType::Double, Some(value));
                Ok(())
            }
            Err(_) => Err(BccError::lex_error(
                Span::new(self.start, self.current),
                format!("Invalid double: {}", value),
            )),
        }
    } else {
        match value.parse::<i64>() {
            Ok(_) => {
                self.add_token_with_literal(TokenType::Integer, Some(value));
                Ok(())
            }
            Err(_) => Err(BccError::lex_error(
                Span::new(self.start, self.current),
                format!("Invalid integer: {}", value),
            )),
        }
    }
}
```

**Purpose**: Parse numeric literals, distinguishing integers from doubles.

**Algorithm**:
1. Consume all digits (integer part)
2. Check for decimal point followed by digit (using `peek_next()`)
3. If decimal found, consume fractional part
4. Parse as appropriate numeric type
5. Validate the parsed result

**Key insight**: Uses `peek_next()` to avoid consuming `.` in cases like `123.toString()` where the dot isn't part of the number.

### Identifier/Keyword Parsing: `identifier()`

```rust
fn identifier(&mut self) {
    while self.peek().is_ascii_alphanumeric() || self.peek() == '_' {
        self.advance();
    }

    let text: String = self.source[self.start..self.current].iter().collect();
    let token_type = self
        .keywords
        .get(&text)
        .cloned()
        .unwrap_or(TokenType::Identifier);

    self.add_token(token_type);
}
```

**Purpose**: Parse identifiers and keywords.

**Algorithm**:
1. Consume letters, digits, and underscores
2. Extract the complete text
3. Look up in keyword table
4. Default to `Identifier` if not a keyword

**Design note**: This is the classic approach - parse as identifier first, then check if it's a reserved word.

### Token Creation: `add_token()` and `add_token_with_literal()`

```rust
fn add_token(&mut self, token_type: TokenType) {
    self.add_token_with_literal(token_type, None);
}

fn add_token_with_literal(&mut self, token_type: TokenType, literal: Option<String>) {
    let text: String = self.source[self.start..self.current].iter().collect();
    let lexeme = literal.unwrap_or(text);
    
    self.tokens.push(Token::new(
        token_type,
        lexeme,
        Span::new(self.start, self.current),
    ));
}
```

**Purpose**: Create tokens with proper span information.

**Two variants**:
- `add_token()`: Uses source text as lexeme
- `add_token_with_literal()`: Uses custom lexeme (for strings without quotes)

## Token Recognition Patterns

### Single Characters
Simple one-to-one mapping:
- `(` → `LeftParen`
- `+` → `Plus`
- `{` → `LeftBrace`

### Multi-Character Operators
Uses conditional consumption:
- `!` → check next char → `!=` or `!`
- `=` → check next char → `==` or `=`
- `<` → check next char → `<=` or `<`

### Complex Tokens

**Strings**: Delimited parsing with error handling
- Start: `"`
- Content: anything except `"`
- End: `"` (error if missing)

**Numbers**: Lookahead to distinguish integers and doubles
- Pattern: `digit+ ('.' digit+)?`
- Uses `peek_next()` to avoid consuming non-numeric dots

**Identifiers/Keywords**: Greedy parsing with lookup
- Pattern: `(letter|_)(letter|digit|_)*`
- Keyword table lookup after parsing

## Error Handling

The lexer produces rich error information:

```rust
BccError::lex_error(
    Span::new(start, end),
    "Descriptive error message".to_string(),
)
```

**Error categories**:
1. **Unexpected characters**: Clear character identification
2. **Unterminated strings**: Span covers the entire string
3. **Invalid numbers**: Rare but possible with extreme values

**Span information**: Every error includes precise position data for excellent diagnostics with ariadne.

## Design Decisions

### Why Owned Strings for Tokens?
- **Simplicity**: No lifetime management complexity in token handling
- **Maintainability**: Easier to understand and modify code
- **Flexibility**: Tokens can be stored and passed around without restrictions
- **Clarity**: Prioritizes code readability over micro-optimizations

### Why HashMap for Keywords?
- **Performance**: O(1) lookup vs O(n) linear search
- **Maintainability**: Easy to add/remove keywords
- **Memory efficiency**: Uses static string slices as keys

### Why Separate Integer/Double Types?
- **Type preservation**: Maintains programmer intent
- **Performance**: Integer operations can be faster
- **Error messages**: Better type information for runtime errors

### Why Span-Based Errors?
- **Rich diagnostics**: Enables beautiful error reporting with ariadne
- **Tool integration**: IDEs can provide precise error underlining
- **Debugging**: Developers know exactly where problems occur

## Performance Characteristics

**Time Complexity**: O(n) where n is the source code length
- Each character is processed exactly once
- Keyword lookup is O(1) amortized

**Space Complexity**: O(n) for storing tokens with owned lexemes
- Tokens contain spans and owned string lexemes
- Source string is referenced, not duplicated

**Design Trade-offs**:
- Pre-computed keyword hash map for fast lookup
- Single-pass processing
- Owned strings prioritize maintainability over memory efficiency
- Character iterator ensures Unicode safety

The lexer prioritizes clarity and maintainability over micro-optimizations. For an educational interpreter, code simplicity and readability are more valuable than squeezing out every byte of memory.