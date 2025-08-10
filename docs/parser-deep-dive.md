# The Parser: A Deep Dive

The parser is the second phase of the BCC interpreter, responsible for transforming the flat sequence of tokens from the lexer into a structured Abstract Syntax Tree (AST). This document provides a comprehensive exploration of how our recursive descent parser works.

## Table of Contents
1. [Overview](#overview)
2. [Grammar and Precedence](#grammar-and-precedence)
3. [Core Data Structures](#core-data-structures)
4. [Recursive Descent Strategy](#recursive-descent-strategy)
5. [Function-by-Function Analysis](#function-by-function-analysis)
6. [Error Handling and Recovery](#error-handling-and-recovery)
7. [Design Decisions](#design-decisions)
8. [AST Construction Patterns](#ast-construction-patterns)

## Overview

The BCC parser implements a **recursive descent parser** - a top-down parsing technique where each grammar rule is implemented as a function. This approach is intuitive, maintainable, and produces excellent error messages.

**Key characteristics:**
- **Recursive descent**: Each grammar rule becomes a function
- **Precedence climbing**: Lower precedence rules call higher precedence rules
- **Rich AST**: Every node includes span information for error reporting
- **Error recovery**: Attempts to continue parsing after errors when possible
- **Optional semicolons**: Python-like syntax flexibility

## Grammar and Precedence

Our language follows this grammar (from lowest to highest precedence):

```
program        → declaration* EOF
declaration    → statement
statement      → printStmt | blockStmt | ifStmt | whileStmt | forStmt | exprStmt
printStmt      → "print" expression (";" | ε)
blockStmt      → "{" declaration* "}"
ifStmt         → "if" "(" expression ")" statement ("else" statement)?
whileStmt      → "while" "(" expression ")" statement
forStmt        → "for" "(" (exprStmt | ";") expression? ";" expression? ")" statement
exprStmt       → expression (";" | ε)

expression     → assignment
assignment     → IDENTIFIER "=" assignment | or
or             → and ("or" and)*
and            → equality ("and" equality)*
equality       → comparison (("!=" | "==") comparison)*
comparison     → term ((">" | ">=" | "<" | "<=") term)*
term           → factor (("-" | "+") factor)*
factor         → unary (("/" | "*") unary)*
unary          → ("!" | "-") unary | call
call           → primary ("(" arguments? ")")*
primary        → "true" | "false" | "nil" | INTEGER | DOUBLE | STRING 
                | IDENTIFIER | "(" expression ")"
```

**Precedence levels** (lowest to highest):
1. **Assignment** (`=`) - right-associative
2. **Logical OR** (`or`) - left-associative
3. **Logical AND** (`and`) - left-associative
4. **Equality** (`==`, `!=`) - left-associative
5. **Comparison** (`<`, `<=`, `>`, `>=`) - left-associative
6. **Term** (`+`, `-`) - left-associative
7. **Factor** (`*`, `/`) - left-associative
8. **Unary** (`!`, `-`) - right-associative
9. **Call** (`()`) - left-associative
10. **Primary** - literals, identifiers, grouping

## Core Data Structures

### Parser State

```rust
pub struct Parser {
    tokens: Vec<Token>,    // Input token stream
    current: usize,        // Current position in token stream
}
```

The parser maintains minimal state - just the tokens and current position. This simplicity makes the parser easy to reason about and debug.

### AST Node Types

**Statements** (things that execute):
```rust
pub enum Stmt {
    Expression { expr: Expr, span: Span },
    Print { expr: Expr, span: Span },
    Block { statements: Vec<Stmt>, span: Span },
    If { condition: Expr, then_branch: Box<Stmt>, else_branch: Option<Box<Stmt>>, span: Span },
    While { condition: Expr, body: Box<Stmt>, span: Span },
    For { initializer: Option<Box<Stmt>>, condition: Option<Expr>, 
          increment: Option<Expr>, body: Box<Stmt>, span: Span },
}
```

**Expressions** (things that evaluate to values):
```rust
pub enum Expr {
    Literal { value: Value, span: Span },
    Variable { name: String, span: Span },
    Assign { name: String, value: Box<Expr>, span: Span },
    Binary { left: Box<Expr>, operator: BinaryOp, right: Box<Expr>, span: Span },
    Unary { operator: UnaryOp, operand: Box<Expr>, span: Span },
    Logical { left: Box<Expr>, operator: LogicalOp, right: Box<Expr>, span: Span },
    Call { callee: Box<Expr>, args: Vec<Expr>, span: Span },
    Grouping { expr: Box<Expr>, span: Span },
}
```

## Recursive Descent Strategy

Recursive descent works by having each grammar rule implemented as a function that:
1. **Consumes** tokens that match the rule
2. **Calls** other rule functions for sub-expressions
3. **Returns** an AST node representing the parsed construct
4. **Reports errors** when expectations aren't met

The **precedence hierarchy** ensures correct operator precedence by having lower-precedence rules call higher-precedence rules.

## Function-by-Function Analysis

### Entry Point: `parse() -> Result<Program, BccError>`

```rust
pub fn parse(&mut self) -> Result<Program, BccError> {
    let mut statements = Vec::new();

    while !self.is_at_end() {
        statements.push(self.declaration()?);
    }

    Ok(Program { statements })
}
```

**Purpose**: Main driver that parses the entire program.

**Algorithm**:
1. Create empty statement list
2. Parse declarations until EOF
3. Wrap in Program node

**Error propagation**: Uses `?` to bubble up any parsing errors immediately.

### Top-Level Parsing: `declaration() -> Result<Stmt, BccError>`

```rust
fn declaration(&mut self) -> Result<Stmt, BccError> {
    self.statement()
}
```

**Purpose**: Handle top-level constructs (currently just statements).

**Design note**: This function exists to support future language features like variable declarations or function definitions. It's a common pattern in extensible parsers.

### Statement Parsing: `statement() -> Result<Stmt, BccError>`

```rust
fn statement(&mut self) -> Result<Stmt, BccError> {
    if self.match_types(&[TokenType::Print]) {
        self.print_statement()
    } else if self.match_types(&[TokenType::LeftBrace]) {
        Ok(Stmt::Block {
            statements: self.block()?,
            span: self.previous().span.clone(),
        })
    } else if self.match_types(&[TokenType::If]) {
        self.if_statement()
    } else if self.match_types(&[TokenType::While]) {
        self.while_statement()
    } else if self.match_types(&[TokenType::For]) {
        self.for_statement()
    } else {
        self.expression_statement()
    }
}
```

**Purpose**: Dispatch to appropriate statement parser based on leading token.

**Pattern**: Classic **first-set parsing** - use the first token to determine which rule to apply.

### Print Statement: `print_statement() -> Result<Stmt, BccError>`

```rust
fn print_statement(&mut self) -> Result<Stmt, BccError> {
    let start_span = self.previous().span.start;
    let expr = self.expression()?;
    
    // Make semicolon optional
    if self.check(&TokenType::Semicolon) {
        self.advance();
    }
    
    let end_span = self.previous().span.end;

    Ok(Stmt::Print {
        expr,
        span: Span::new(start_span, end_span),
    })
}
```

**Purpose**: Parse `print expression` statements.

**Features**:
- **Optional semicolons**: Checks for semicolon but doesn't require it
- **Span tracking**: Combines spans from keyword to end of expression
- **Expression delegation**: Calls `expression()` for the value to print

### Block Parsing: `block() -> Result<Vec<Stmt>, BccError>`

```rust
fn block(&mut self) -> Result<Vec<Stmt>, BccError> {
    let mut statements = Vec::new();

    while !self.check(&TokenType::RightBrace) && !self.is_at_end() {
        statements.push(self.declaration()?);
    }

    self.consume(TokenType::RightBrace, "Expected '}' after block")?;
    Ok(statements)
}
```

**Purpose**: Parse sequences of statements between braces.

**Algorithm**:
1. Collect statements until `}` or EOF
2. Ensure closing brace exists
3. Return statement list

**Error handling**: Clear error message if closing brace is missing.

### If Statement: `if_statement() -> Result<Stmt, BccError>`

```rust
fn if_statement(&mut self) -> Result<Stmt, BccError> {
    let start_span = self.previous().span.start;
    
    self.consume(TokenType::LeftParen, "Expected '(' after 'if'")?;
    let condition = self.expression()?;
    self.consume(TokenType::RightParen, "Expected ')' after if condition")?;

    let then_branch = Box::new(self.statement()?);
    let else_branch = if self.match_types(&[TokenType::Else]) {
        Some(Box::new(self.statement()?))
    } else {
        None
    };

    let end_span = if let Some(ref else_stmt) = else_branch {
        else_stmt.span().end
    } else {
        then_branch.span().end
    };

    Ok(Stmt::If {
        condition,
        then_branch,
        else_branch,
        span: Span::new(start_span, end_span),
    })
}
```

**Purpose**: Parse if-else statements.

**Structure**: `if ( condition ) statement ( else statement )?`

**Key points**:
- **Mandatory parentheses**: Follows C-style syntax
- **Optional else**: Uses `Option<Box<Stmt>>`
- **Statement recursion**: Both branches can be any statement type
- **Span calculation**: Handles both if-only and if-else cases

### While Loop: `while_statement() -> Result<Stmt, BccError>`

```rust
fn while_statement(&mut self) -> Result<Stmt, BccError> {
    let start_span = self.previous().span.start;
    
    self.consume(TokenType::LeftParen, "Expected '(' after 'while'")?;
    let condition = self.expression()?;
    self.consume(TokenType::RightParen, "Expected ')' after while condition")?;
    
    let body = Box::new(self.statement()?);
    let end_span = body.span().end;

    Ok(Stmt::While {
        condition,
        body,
        span: Span::new(start_span, end_span),
    })
}
```

**Purpose**: Parse while loops.

**Structure**: `while ( condition ) statement`

**Design**: Simple condition-body structure, consistent with if statement parsing.

### For Loop: `for_statement() -> Result<Stmt, BccError>`

```rust
fn for_statement(&mut self) -> Result<Stmt, BccError> {
    let start_span = self.previous().span.start;
    
    self.consume(TokenType::LeftParen, "Expected '(' after 'for'")?;

    let initializer = if self.match_types(&[TokenType::Semicolon]) {
        None
    } else {
        Some(Box::new(self.expression_statement()?))
    };

    let condition = if !self.check(&TokenType::Semicolon) {
        Some(self.expression()?)
    } else {
        None
    };
    self.consume(TokenType::Semicolon, "Expected ';' after loop condition")?;

    let increment = if !self.check(&TokenType::RightParen) {
        Some(self.expression()?)
    } else {
        None
    };
    self.consume(TokenType::RightParen, "Expected ')' after for clauses")?;

    let body = Box::new(self.statement()?);
    let end_span = body.span().end;

    Ok(Stmt::For {
        initializer,
        condition,
        increment,
        body,
        span: Span::new(start_span, end_span),
    })
}
```

**Purpose**: Parse C-style for loops.

**Structure**: `for ( init?; condition?; increment? ) statement`

**Flexibility**:
- All three clauses are optional
- Initializer must be a statement (allows assignments)
- Condition and increment are expressions

### Expression Statement: `expression_statement() -> Result<Stmt, BccError>`

```rust
fn expression_statement(&mut self) -> Result<Stmt, BccError> {
    let start_span = self.peek().span.start;
    let expr = self.expression()?;
    
    // Make semicolon optional
    if self.check(&TokenType::Semicolon) {
        self.advance();
    }
    
    let end_span = self.previous().span.end;

    Ok(Stmt::Expression {
        expr,
        span: Span::new(start_span, end_span),
    })
}
```

**Purpose**: Parse expressions used as statements.

**Examples**: `x = 5`, `foo()`, `x + y`

**Feature**: Optional semicolons for Python-like feel.

## Expression Parsing Hierarchy

The expression parsing functions implement operator precedence through the call hierarchy:

### Assignment: `assignment() -> Result<Expr, BccError>`

```rust
fn assignment(&mut self) -> Result<Expr, BccError> {
    let expr = self.or()?;

    if self.match_types(&[TokenType::Equal]) {
        let equals = self.previous().clone();
        let value = self.assignment()?;

        if let Expr::Variable { name, span } = expr {
            return Ok(Expr::Assign {
                name,
                value: Box::new(value),
                span: Span::new(span.start, self.previous().span.end),
            });
        }

        return Err(BccError::parse_error(
            equals.span,
            "Invalid assignment target".to_string(),
        ));
    }

    Ok(expr)
}
```

**Purpose**: Handle variable assignment (lowest precedence).

**Key features**:
- **Right-associative**: `a = b = c` parses as `a = (b = c)`
- **L-value validation**: Only variables can be assigned to
- **Clear error messages**: "Invalid assignment target" for `1 = 2`

### Logical Operators: `or()` and `and()`

```rust
fn or(&mut self) -> Result<Expr, BccError> {
    let mut expr = self.and()?;

    while self.match_types(&[TokenType::Or]) {
        let start = expr.span().start;
        let right = self.and()?;
        let end = right.span().end;
        
        expr = Expr::Logical {
            left: Box::new(expr),
            operator: LogicalOp::Or,
            right: Box::new(right),
            span: Span::new(start, end),
        };
    }

    Ok(expr)
}
```

**Purpose**: Handle logical OR with short-circuit evaluation.

**Pattern**: **Left-associative binary operators** using a while loop:
1. Parse left operand
2. While operator matches, parse right operand
3. Create binary expression
4. Left operand becomes the binary expression (left-associative)

### Binary Operators: `equality()`, `comparison()`, `term()`, `factor()`

These all follow the same pattern:

```rust
fn equality(&mut self) -> Result<Expr, BccError> {
    let mut expr = self.comparison()?;

    while self.match_types(&[TokenType::BangEqual, TokenType::EqualEqual]) {
        let operator_token = self.previous().clone();
        let operator = match operator_token.token_type {
            TokenType::BangEqual => BinaryOp::NotEqual,
            TokenType::EqualEqual => BinaryOp::Equal,
            _ => unreachable!(),
        };
        
        let start = expr.span().start;
        let right = self.comparison().map_err(|_| {
            BccError::parse_error(
                operator_token.span.clone(),
                format!("Expected expression after '{}'", operator_token.lexeme)
            )
        })?;
        let end = right.span().end;
        
        expr = Expr::Binary {
            left: Box::new(expr),
            operator,
            right: Box::new(right),
            span: Span::new(start, end),
        };
    }

    Ok(expr)
}
```

**Common pattern**:
1. Parse left operand at next precedence level
2. While current token matches operator
3. Convert token to operator enum
4. Parse right operand with error handling
5. Create binary expression
6. Continue with new expression as left operand

### Unary Operators: `unary() -> Result<Expr, BccError>`

```rust
fn unary(&mut self) -> Result<Expr, BccError> {
    if self.match_types(&[TokenType::Bang, TokenType::Minus]) {
        let operator = match self.previous().token_type {
            TokenType::Bang => UnaryOp::Not,
            TokenType::Minus => UnaryOp::Negate,
            _ => unreachable!(),
        };
        
        let start = self.previous().span.start;
        let right = self.unary()?;
        let end = right.span().end;
        
        return Ok(Expr::Unary {
            operator,
            operand: Box::new(right),
            span: Span::new(start, end),
        });
    }

    self.call()
}
```

**Purpose**: Handle unary prefix operators.

**Right-associative**: `!!x` parses as `!(!x)` through recursion.

### Function Calls: `call() -> Result<Expr, BccError>`

```rust
fn call(&mut self) -> Result<Expr, BccError> {
    let mut expr = self.primary()?;

    while self.match_types(&[TokenType::LeftParen]) {
        expr = self.finish_call(expr)?;
    }

    Ok(expr)
}

fn finish_call(&mut self, callee: Expr) -> Result<Expr, BccError> {
    let mut args = Vec::new();

    if !self.check(&TokenType::RightParen) {
        loop {
            args.push(self.expression()?);
            if !self.match_types(&[TokenType::Comma]) {
                break;
            }
        }
    }

    let paren = self.consume(TokenType::RightParen, "Expected ')' after arguments")?;

    Ok(Expr::Call {
        callee: Box::new(callee),
        args,
        span: paren.span.clone(),
    })
}
```

**Purpose**: Handle function calls (not yet implemented in evaluator).

**Features**:
- **Multiple calls**: `foo()()` works
- **Argument parsing**: Comma-separated expressions
- **Empty argument lists**: `foo()` works

### Primary Expressions: `primary() -> Result<Expr, BccError>`

```rust
fn primary(&mut self) -> Result<Expr, BccError> {
    let token = self.advance().clone();

    match token.token_type {
        TokenType::False => Ok(Expr::Literal {
            value: Value::Bool(false),
            span: token.span,
        }),
        TokenType::True => Ok(Expr::Literal {
            value: Value::Bool(true),
            span: token.span,
        }),
        TokenType::Nil => Ok(Expr::Literal {
            value: Value::Nil,
            span: token.span,
        }),
        TokenType::Integer => {
            let value = token.lexeme.parse::<i64>().map_err(|_| {
                BccError::parse_error(token.span.clone(), "Invalid integer".to_string())
            })?;
            Ok(Expr::Literal {
                value: Value::Int(value),
                span: token.span,
            })
        }
        // ... more cases ...
        TokenType::LeftParen => {
            let expr = self.expression()?;
            self.consume(TokenType::RightParen, "Expected ')' after expression")?;
            Ok(Expr::Grouping {
                expr: Box::new(expr),
                span: token.span,
            })
        }
        _ => Err(BccError::parse_error(
            token.span,
            "Expected expression".to_string(),
        )),
    }
}
```

**Purpose**: Handle the simplest expressions (highest precedence).

**Cases**:
- **Literals**: `true`, `false`, `nil`, numbers, strings
- **Variables**: Identifiers
- **Grouping**: Parenthesized expressions

## Utility Functions

### Token Stream Navigation

```rust
fn match_types(&mut self, types: &[TokenType]) -> bool {
    for token_type in types {
        if self.check(token_type) {
            self.advance();
            return true;
        }
    }
    false
}

fn check(&self, token_type: &TokenType) -> bool {
    if self.is_at_end() {
        false
    } else {
        &self.peek().token_type == token_type
    }
}

fn advance(&mut self) -> &Token {
    if !self.is_at_end() {
        self.current += 1;
    }
    self.previous()
}

fn consume(&mut self, token_type: TokenType, message: &str) -> Result<&Token, BccError> {
    if self.check(&token_type) {
        Ok(self.advance())
    } else {
        Err(BccError::parse_error(
            self.peek().span.clone(),
            message.to_string(),
        ))
    }
}
```

**Purpose**: Safe token stream manipulation.

**Functions**:
- `match_types()`: Try to match any of several token types
- `check()`: Test current token without advancing
- `advance()`: Move to next token
- `consume()`: Require specific token or error

## Error Handling and Recovery

### Immediate Error Reporting

The parser uses **panic-mode error recovery** - when an error occurs, it reports the error and stops parsing. This is simple but effective for a teaching interpreter.

```rust
self.consume(TokenType::RightParen, "Expected ')' after condition")?;
```

**Benefits**:
- **Clear error messages**: Specific context for each error
- **Precise locations**: Span information for excellent diagnostics
- **Fail-fast behavior**: Prevents cascading errors

### Context-Sensitive Messages

Different parsing contexts provide specific error messages:

```rust
// In assignment parsing
return Err(BccError::parse_error(
    equals.span,
    "Invalid assignment target".to_string(),
));

// In call parsing  
self.consume(TokenType::RightParen, "Expected ')' after arguments")?;

// In primary parsing
_ => Err(BccError::parse_error(
    token.span,
    "Expected expression".to_string(),
)),
```

## AST Construction Patterns

### Span Tracking

Every AST node includes span information:

```rust
Ok(Stmt::If {
    condition,
    then_branch,
    else_branch,
    span: Span::new(start_span, end_span),
})
```

**Benefits**:
- **Error reporting**: Precise error locations
- **IDE integration**: Hover information and debugging
- **Source maps**: Future tooling support

### Boxing for Recursion

Recursive structures use `Box` to avoid infinite size:

```rust
pub enum Expr {
    Binary {
        left: Box<Expr>,      // Heap allocation
        operator: BinaryOp,
        right: Box<Expr>,     // Heap allocation
        span: Span,
    },
    // ...
}
```

### Optional Elements

Optional syntax uses `Option<T>`:

```rust
pub struct Stmt::If {
    condition: Expr,
    then_branch: Box<Stmt>,
    else_branch: Option<Box<Stmt>>,  // Optional else clause
    span: Span,
}
```

## Design Decisions

### Why Recursive Descent?

**Advantages**:
- **Intuitive**: Grammar rules map directly to functions
- **Maintainable**: Easy to modify and extend
- **Debuggable**: Standard function call stack
- **Error messages**: Natural context for errors

**Trade-offs**:
- **Stack usage**: Deep expressions can cause stack overflow
- **Left recursion**: Can't handle left-recursive grammars directly

### Why Optional Semicolons?

**Python-like feel**: More natural for scripting
**Implementation**: Check for semicolon but don't require it
**Compatibility**: Still accepts C-style code with semicolons

### Why Rich AST Nodes?

**Span information**: Essential for good error reporting
**Type safety**: Enums prevent invalid AST construction
**Pattern matching**: Elegant handling in evaluator

### Precedence Through Call Hierarchy

**Natural precedence**: Lower precedence functions call higher precedence
**No operator table**: Grammar encodes precedence directly
**Extensible**: Easy to add new operators at any level

## Performance Characteristics

**Time Complexity**: O(n) where n is the number of tokens
- Each token is processed once
- Recursive calls don't increase overall complexity

**Space Complexity**: O(d) where d is expression depth
- Call stack depth proportional to expression nesting
- AST size proportional to program size

**Memory allocation**: Lots of `Box` allocations for tree structure
- Could be optimized with arena allocation
- Fine for a teaching interpreter

The parser prioritizes clarity and maintainability over micro-optimization. The recursive descent approach makes the code easy to understand and modify, which aligns with the educational goals of the project.