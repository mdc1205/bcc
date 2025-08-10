use crate::ast::{BinaryOp, Expr, LogicalOp, Program, Stmt, UnaryOp};
use crate::error::{BccError, Span};
use crate::lexer::{Token, TokenType};
use crate::value::Value;

pub struct Parser {
    tokens: Vec<Token>,
    current: usize,
}

impl Parser {
    pub fn new(tokens: Vec<Token>) -> Self {
        Self { tokens, current: 0 }
    }

    pub fn parse(&mut self) -> Result<Program, BccError> {
        let mut statements = Vec::new();

        while !self.is_at_end() {
            statements.push(self.declaration()?);
        }

        Ok(Program { statements })
    }

    fn declaration(&mut self) -> Result<Stmt, BccError> {
        self.statement()
    }

    fn statement(&mut self) -> Result<Stmt, BccError> {
        if self.check(&TokenType::LeftBrace) {
            // Look ahead to determine if this is a dictionary expression or a block statement
            if self.is_dictionary_literal() {
                // Parse as expression statement containing a dictionary
                self.expression_statement()
            } else {
                // Parse as block statement
                self.advance(); // consume the '{'
                Ok(Stmt::Block {
                    statements: self.block()?,
                    span: self.previous().span.clone(),
                })
            }
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

    fn block(&mut self) -> Result<Vec<Stmt>, BccError> {
        let mut statements = Vec::new();

        while !self.check(&TokenType::RightBrace) && !self.is_at_end() {
            statements.push(self.declaration()?);
        }

        self.consume_with_help(
            TokenType::RightBrace, 
            "Expected '}' after block",
            "Block statements must be closed with '}' after the opening '{'.".to_string()
        )?;
        Ok(statements)
    }

    fn if_statement(&mut self) -> Result<Stmt, BccError> {
        let start_span = self.previous().span.start;
        
        self.consume_with_help(
            TokenType::LeftParen, 
            "Expected '(' after 'if'",
            "If statements require parentheses around the condition: if (condition) { ... }".to_string()
        )?;
        let condition = self.expression()?;
        self.consume_with_help(
            TokenType::RightParen, 
            "Expected ')' after if condition",
            "If conditions must be enclosed in parentheses: if (condition) { ... }".to_string()
        )?;

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

    fn expression(&mut self) -> Result<Expr, BccError> {
        self.assignment()
    }

    fn assignment(&mut self) -> Result<Expr, BccError> {
        let expr = self.or()?;

        if self.match_types(&[TokenType::Equal]) {
            let equals = self.previous().clone();
            let value = self.assignment()?;

            // Check if it's a single variable assignment
            if let Expr::Variable { name, span } = expr {
                return Ok(Expr::Assign {
                    name,
                    value: Box::new(value),
                    span: Span::new(span.start, self.previous().span.end),
                });
            }

            // Check if it's a tuple for multi-assignment: a, b, c = expr
            if let Expr::Tuple { elements, span } = expr {
                let mut targets = Vec::new();
                
                for element in elements {
                    match element {
                        Expr::Variable { name, span } => {
                            if name == "_" {
                                targets.push(crate::ast::AssignTarget::Ignore { span });
                            } else {
                                targets.push(crate::ast::AssignTarget::Variable { name, span });
                            }
                        }
                        _ => {
                            return Err(BccError::parse_error_with_help(
                                element.span().clone(),
                                "Invalid assignment target in multi-assignment".to_string(),
                                "Multi-assignment targets must be variables or underscores. Example: 'a, b, _ = expr'".to_string(),
                            ));
                        }
                    }
                }

                return Ok(Expr::MultiAssign {
                    targets,
                    value: Box::new(value),
                    span: Span::new(span.start, self.previous().span.end),
                });
            }

            return Err(BccError::parse_error_with_help(
                equals.span,
                "Invalid assignment target".to_string(),
                "Only variables and tuples can be assigned to. Examples: 'x = 10' or 'a, b = expr'".to_string(),
            ));
        }

        Ok(expr)
    }

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

    fn and(&mut self) -> Result<Expr, BccError> {
        let mut expr = self.equality()?;

        while self.match_types(&[TokenType::And]) {
            let start = expr.span().start;
            let right = self.equality()?;
            let end = right.span().end;
            
            expr = Expr::Logical {
                left: Box::new(expr),
                operator: LogicalOp::And,
                right: Box::new(right),
                span: Span::new(start, end),
            };
        }

        Ok(expr)
    }

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
                BccError::parse_error_with_help(
                    operator_token.span.clone(),
                    format!("Expected expression after '{}'", operator_token.lexeme),
                    "Equality operators like '==' and '!=' require expressions on both sides.".to_string(),
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

    fn comparison(&mut self) -> Result<Expr, BccError> {
        let mut expr = self.term()?;

        while self.match_types(&[
            TokenType::Greater,
            TokenType::GreaterEqual,
            TokenType::Less,
            TokenType::LessEqual,
            TokenType::In,
        ]) {
            let operator_token = self.previous().clone();
            let operator = match operator_token.token_type {
                TokenType::Greater => BinaryOp::Greater,
                TokenType::GreaterEqual => BinaryOp::GreaterEqual,
                TokenType::Less => BinaryOp::Less,
                TokenType::LessEqual => BinaryOp::LessEqual,
                TokenType::In => BinaryOp::In,
                _ => unreachable!(),
            };
            
            let start = expr.span().start;
            let right = self.term().map_err(|_| {
                BccError::parse_error_with_help(
                    operator_token.span.clone(),
                    format!("Expected expression after '{}'", operator_token.lexeme),
                    "Comparison operators like '>', '<', '>=', '<=' and 'in' require expressions on both sides.".to_string(),
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

    fn term(&mut self) -> Result<Expr, BccError> {
        let mut expr = self.factor()?;

        while self.match_types(&[TokenType::Minus, TokenType::Plus]) {
            let operator_token = self.previous().clone();
            let operator = match operator_token.token_type {
                TokenType::Minus => BinaryOp::Subtract,
                TokenType::Plus => BinaryOp::Add,
                _ => unreachable!(),
            };
            
            let start = expr.span().start;
            let right = self.factor().map_err(|_| {
                BccError::parse_error_with_help(
                    operator_token.span.clone(),
                    format!("Expected expression after '{}'", operator_token.lexeme),
                    "Arithmetic operators like '+' and '-' require expressions on both sides.".to_string(),
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

    fn factor(&mut self) -> Result<Expr, BccError> {
        let mut expr = self.unary()?;

        while self.match_types(&[TokenType::Slash, TokenType::Star]) {
            let operator_token = self.previous().clone();
            let operator = match operator_token.token_type {
                TokenType::Slash => BinaryOp::Divide,
                TokenType::Star => BinaryOp::Multiply,
                _ => unreachable!(),
            };
            
            let start = expr.span().start;
            let right = self.unary().map_err(|_| {
                BccError::parse_error_with_help(
                    operator_token.span.clone(),
                    format!("Expected expression after '{}'", operator_token.lexeme),
                    "Multiplication and division operators require expressions on both sides.".to_string(),
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

    fn unary(&mut self) -> Result<Expr, BccError> {
        if self.match_types(&[TokenType::Bang, TokenType::Not, TokenType::Minus]) {
            let operator = match self.previous().token_type {
                TokenType::Bang | TokenType::Not => UnaryOp::Not,
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

    fn call(&mut self) -> Result<Expr, BccError> {
        let mut expr = self.primary()?;

        loop {
            if self.match_types(&[TokenType::LeftParen]) {
                expr = self.finish_call(expr)?;
            } else if self.match_types(&[TokenType::Dot]) {
                let property_token = self.consume(
                    TokenType::Identifier,
                    "Expected property name after '.'.",
                )?;
                
                let property_name = if let TokenType::Identifier = &property_token.token_type {
                    property_token.lexeme.clone()
                } else {
                    return Err(BccError::parse_error(
                        property_token.span.clone(),
                        "Expected property name after '.'".to_string(),
                    ));
                };

                let start_span = expr.span().start;
                let end_span = property_token.span.end;
                expr = Expr::PropertyAccess {
                    object: Box::new(expr),
                    property: property_name,
                    span: Span::new(start_span, end_span),
                };
            } else {
                break;
            }
        }

        Ok(expr)
    }

    fn finish_call(&mut self, callee: Expr) -> Result<Expr, BccError> {
        let mut positional_args = Vec::new();
        let mut keyword_args = Vec::new();
        let start_span = callee.span().start;
        let mut found_kwarg = false;

        if !self.check(&TokenType::RightParen) {
            loop {
                // Check if we've hit EOF or other invalid tokens before trying to parse expression
                if self.is_at_end() {
                    return Err(BccError::parse_error_with_help(
                        Span::single(self.current),
                        "Unexpected end of input in function call".to_string(),
                        "Function calls must be closed with ')' after the arguments. Example: func(arg1, arg2)".to_string(),
                    ));
                }
                
                // Check for common error cases that could cause infinite recursion
                if self.check(&TokenType::RightBrace) || self.check(&TokenType::RightBracket) {
                    return Err(BccError::parse_error_with_help(
                        self.peek().span.clone(),
                        "Expected ')' to close function call".to_string(),
                        "Function calls must be closed with ')' after the arguments. Example: func(arg1, arg2)".to_string(),
                    ));
                }
                
                // Check for keyword argument: identifier=expression
                if self.check(&TokenType::Identifier) {
                    let checkpoint = self.current;
                    let name_token = self.advance();
                    
                    if self.match_types(&[TokenType::Equal]) {
                        // This is a keyword argument
                        found_kwarg = true;
                        let value = self.expression().map_err(|_e| {
                            BccError::parse_error_with_help(
                                self.peek().span.clone(),
                                "Invalid expression in keyword argument".to_string(),
                                "Keyword arguments must have valid expressions. Example: func(name=value)".to_string(),
                            )
                        })?;
                        
                        keyword_args.push(crate::ast::KeywordArg {
                            name: name_token.lexeme.clone(),
                            value,
                            span: name_token.span.clone(),
                        });
                    } else {
                        // Not a keyword argument, backtrack and parse as positional
                        self.current = checkpoint;
                        
                        if found_kwarg {
                            return Err(BccError::parse_error_with_help(
                                self.peek().span.clone(),
                                "Positional argument after keyword argument".to_string(),
                                "All positional arguments must come before keyword arguments. Example: func(pos1, pos2, kw1=val1, kw2=val2)".to_string(),
                            ));
                        }
                        
                        positional_args.push(self.expression().map_err(|_e| {
                            BccError::parse_error_with_help(
                                self.peek().span.clone(),
                                "Invalid expression in function call arguments".to_string(),
                                "Function arguments must be valid expressions separated by commas. Example: func(arg1, arg2)".to_string(),
                            )
                        })?);
                    }
                } else {
                    // Not an identifier, so it's a positional argument
                    if found_kwarg {
                        return Err(BccError::parse_error_with_help(
                            self.peek().span.clone(),
                            "Positional argument after keyword argument".to_string(),
                            "All positional arguments must come before keyword arguments. Example: func(pos1, pos2, kw1=val1, kw2=val2)".to_string(),
                        ));
                    }
                    
                    positional_args.push(self.expression().map_err(|_e| {
                        BccError::parse_error_with_help(
                            self.peek().span.clone(),
                            "Invalid expression in function call arguments".to_string(),
                            "Function arguments must be valid expressions separated by commas. Example: func(arg1, arg2)".to_string(),
                        )
                    })?);
                }
                
                if !self.match_types(&[TokenType::Comma]) {
                    break;
                }
                
                // After consuming a comma, check if we immediately hit EOF or invalid tokens
                if self.is_at_end() {
                    return Err(BccError::parse_error_with_help(
                        Span::single(self.current),
                        "Unexpected end of input after ',' in function call".to_string(),
                        "Function calls must be closed with ')' after the arguments. You have a trailing comma.".to_string(),
                    ));
                }
            }
        }

        let paren = self.consume_with_help(
            TokenType::RightParen, 
            "Expected ')' after arguments",
            "Function calls must be closed with ')' after the arguments. Example: func(arg1, arg2)".to_string()
        )?;

        // Return appropriate call type based on whether we found keyword arguments
        if keyword_args.is_empty() {
            Ok(Expr::Call {
                callee: Box::new(callee),
                args: positional_args,
                span: Span::new(start_span, paren.span.end),
            })
        } else {
            Ok(Expr::CallWithKwargs {
                callee: Box::new(callee),
                positional_args,
                keyword_args,
                span: Span::new(start_span, paren.span.end),
            })
        }
    }

    fn primary(&mut self) -> Result<Expr, BccError> {
        // Check for EOF before advancing to prevent infinite recursion
        if self.is_at_end() {
            return Err(BccError::parse_error_with_help(
                self.peek().span.clone(),
                "Unexpected end of input".to_string(),
                "Expected an expression here. Check for unmatched parentheses, brackets, or incomplete statements.".to_string(),
            ));
        }
        
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
            TokenType::Double => {
                let value = token.lexeme.parse::<f64>().map_err(|_| {
                    BccError::parse_error(token.span.clone(), "Invalid double".to_string())
                })?;
                Ok(Expr::Literal {
                    value: Value::Double(value),
                    span: token.span,
                })
            }
            TokenType::String => Ok(Expr::Literal {
                value: Value::String(token.lexeme), // Simplified: lexeme is already an owned String
                span: token.span,
            }),
            TokenType::Identifier => Ok(Expr::Variable {
                name: token.lexeme, // Simplified: lexeme is already an owned String
                span: token.span,
            }),
            TokenType::LeftParen => {
                let start_span = token.span.clone();
                
                // Check if we immediately hit EOF - this prevents infinite recursion
                if self.is_at_end() {
                    return Err(BccError::parse_error_with_help(
                        start_span,
                        "Expected expression after '('".to_string(),
                        "Opening parentheses '(' must contain a valid expression. Example: (x + 1)".to_string(),
                    ));
                }
                
                // Check for empty parentheses () which is a syntax error in this language
                if self.check(&TokenType::RightParen) {
                    return Err(BccError::parse_error_with_help(
                        Span::new(start_span.start, self.peek().span.end),
                        "Empty parentheses are not allowed".to_string(),
                        "Parentheses must contain an expression. Use 'nil' for a null value: (nil)".to_string(),
                    ));
                }
                
                let expr = self.expression()?;
                let end_token = self.consume_with_help(
                    TokenType::RightParen, 
                    "Expected ')' after expression",
                    "Every opening parenthesis '(' must have a matching closing parenthesis ')'.".to_string()
                )?;
                Ok(Expr::Grouping {
                    expr: Box::new(expr),
                    span: Span::new(start_span.start, end_token.span.end),
                })
            }
            TokenType::LeftBracket => {
                self.list_literal(token.span)
            }
            TokenType::LeftBrace => {
                self.dict_literal(token.span)
            }
            _ => {
                let help_msg = match token.token_type {
                    TokenType::RightParen => "Found ')' without matching '('. Check for unbalanced parentheses.",
                    TokenType::RightBrace => "Found '}' without matching '{'. Check for unbalanced braces.",
                    TokenType::RightBracket => "Found ']' without matching '['. Check for unbalanced brackets.",
                    TokenType::Eof => "Reached end of input while expecting an expression.",
                    _ => "Expected a literal value, variable, or parenthesized expression here."
                };
                
                Err(BccError::parse_error_with_help(
                    token.span,
                    format!("Expected expression, found '{}'", token.lexeme),
                    help_msg.to_string(),
                ))
            },
        }
    }

    fn list_literal(&mut self, start_span: Span) -> Result<Expr, BccError> {
        let mut elements = Vec::new();

        if !self.check(&TokenType::RightBracket) {
            loop {
                elements.push(self.expression()?);
                if !self.match_types(&[TokenType::Comma]) {
                    break;
                }
            }
        }

        let end_token = self.consume_with_help(
            TokenType::RightBracket, 
            "Expected ']' after list elements",
            "List literals must be closed with ']' after the opening '['. Example: [1, 2, 3]".to_string()
        )?;
        Ok(Expr::List {
            elements,
            span: Span::new(start_span.start, end_token.span.end),
        })
    }

    fn dict_literal(&mut self, start_span: Span) -> Result<Expr, BccError> {
        let mut pairs = Vec::new();

        if !self.check(&TokenType::RightBrace) {
            loop {
                let key = self.expression()?;
                self.consume_with_help(
                    TokenType::Colon, 
                    "Expected ':' after dictionary key",
                    "Dictionary entries require a colon ':' between key and value. Example: {\"key\": \"value\"}".to_string()
                )?;
                let value = self.expression()?;
                pairs.push((key, value));
                
                if !self.match_types(&[TokenType::Comma]) {
                    break;
                }
            }
        }

        let end_token = self.consume_with_help(
            TokenType::RightBrace, 
            "Expected '}' after dictionary pairs",
            "Dictionary literals must be closed with '}' after the opening '{'. Example: {\"key\": \"value\"}".to_string()
        )?;
        Ok(Expr::Dict {
            pairs,
            span: Span::new(start_span.start, end_token.span.end),
        })
    }

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

    fn is_at_end(&self) -> bool {
        self.peek().token_type == TokenType::Eof
    }

    fn peek(&self) -> &Token {
        &self.tokens[self.current]
    }

    fn previous(&self) -> &Token {
        &self.tokens[self.current - 1]
    }

    fn consume(&mut self, token_type: TokenType, message: &str) -> Result<&Token, BccError> {
        if self.check(&token_type) {
            Ok(self.advance())
        } else {
            // Create a more helpful error span
            let error_span = if self.is_at_end() {
                // If we're at EOF, point to the end of the last non-EOF token
                if self.current > 0 {
                    let last_token = &self.tokens[self.current - 1];
                    Span::single(last_token.span.end)
                } else {
                    // Fallback to current position
                    self.peek().span.clone()
                }
            } else {
                // Point to the current unexpected token
                self.peek().span.clone()
            };
            
            Err(BccError::parse_error(
                error_span,
                message.to_string(),
            ))
        }
    }

    fn consume_with_help(&mut self, token_type: TokenType, message: &str, help: String) -> Result<&Token, BccError> {
        if self.check(&token_type) {
            Ok(self.advance())
        } else {
            // Create a more helpful error span
            let error_span = if self.is_at_end() {
                // If we're at EOF, point to the end of the last non-EOF token
                if self.current > 0 {
                    let last_token = &self.tokens[self.current - 1];
                    Span::single(last_token.span.end)
                } else {
                    // Fallback to current position
                    self.peek().span.clone()
                }
            } else {
                // Point to the current unexpected token
                self.peek().span.clone()
            };
            
            Err(BccError::parse_error_with_help(
                error_span,
                message.to_string(),
                help,
            ))
        }
    }

    /// Look ahead to determine if a '{' starts a dictionary literal or a block statement.
    /// Returns true if it's likely a dictionary, false if it's likely a block.
    fn is_dictionary_literal(&self) -> bool {
        assert!(self.check(&TokenType::LeftBrace), "is_dictionary_literal() called when current token is not '{{'");
        
        // Look at the token after the '{'
        if self.current + 1 >= self.tokens.len() {
            return false; // EOF after '{', assume block
        }
        
        let token_after_brace = &self.tokens[self.current + 1];
        
        match token_after_brace.token_type {
            // Empty dictionary: {}
            TokenType::RightBrace => true,
            
            // Dictionary patterns: {"key": value} or {variable: value} or {(expr): value}
            // Look for the pattern: expression followed by ':'
            _ => {
                // Scan ahead to find either ':' or patterns that suggest a block
                self.scan_for_dictionary_pattern()
            }
        }
    }
    
    /// Scan ahead from the current position to look for dictionary patterns.
    /// Returns true if we find a colon that suggests this is a dictionary, or
    /// if we find clear dictionary-like patterns (but only for unambiguous cases).
    fn scan_for_dictionary_pattern(&self) -> bool {
        let mut pos = self.current + 1; // Start after the '{'
        let mut paren_depth = 0;
        let mut bracket_depth = 0;
        
        // Scan up to a reasonable limit to avoid infinite loops
        let limit = std::cmp::min(pos + 20, self.tokens.len());
        
        while pos < limit {
            let token = &self.tokens[pos];
            
            match token.token_type {
                // Track nested parentheses and brackets
                TokenType::LeftParen => paren_depth += 1,
                TokenType::RightParen => paren_depth -= 1,
                TokenType::LeftBracket => bracket_depth += 1,
                TokenType::RightBracket => bracket_depth -= 1,
                
                // If we find a colon at the top level, it's definitely a dictionary
                TokenType::Colon if paren_depth == 0 && bracket_depth == 0 => {
                    return true;
                }
                
                // If we find a semicolon at the top level, it's likely a block
                TokenType::Semicolon if paren_depth == 0 && bracket_depth == 0 => {
                    return false;
                }
                
                // If we hit the closing brace without finding a colon, it's likely a block
                TokenType::RightBrace if paren_depth == 0 && bracket_depth == 0 => {
                    return false;
                }
                
                // EOF
                TokenType::Eof => return false,
                
                _ => {}
            }
            
            pos += 1;
        }
        
        // If we couldn't determine, default to block (safer choice)
        false
    }
}