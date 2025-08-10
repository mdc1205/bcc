# Keyword Arguments (kwargs) Implementation Guide

This document provides a comprehensive walkthrough of implementing keyword arguments (kwargs) in the BCC interpreter.

## Overview

Keyword arguments allow function calls with named parameters, providing flexibility and clarity. Functions can define default values for parameters, and callers can specify arguments by name in any order.

**Target Syntax:**
```javascript
// Function calls with kwargs
divmod(17, 5)                              // Positional args only
divmod(17, 5, round_mode="up")             // Mixed positional + kwargs  
divmod(a=17, b=5, round_mode="down")       // All kwargs
divmod(17, b=5)                            // Mixed order allowed
```

## Implementation Architecture

### 1. AST Extensions (`src/ast.rs`)

**Challenge**: Representing keyword arguments separately from positional arguments in function calls.

**Solution**: Enhanced the AST with new expression types and structures:

```rust
// New AST node for keyword argument pairs
#[derive(Debug, Clone, PartialEq)]
pub struct KeywordArg {
    pub name: String,
    pub value: Box<Expr>,
    pub span: Span,
}

// Enhanced function call expression
pub enum Expr {
    // ... existing variants
    CallWithKwargs {
        callee: Box<Expr>,
        args: Vec<Expr>,           // Positional arguments  
        kwargs: Vec<KeywordArg>,   // Keyword arguments
        span: Span,
    },
    // ... other variants
}
```

**Key Design Decisions:**
- **Separate Collections**: `args` for positional, `kwargs` for keyword arguments
- **`KeywordArg` Structure**: Contains name, value expression, and span for error reporting
- **`Box<Expr>` Values**: Keyword values are full expressions, not just literals
- **Span Tracking**: Every component tracks location for precise error reporting

**Integration with existing code:**
```rust
impl Expr {
    pub fn span(&self) -> &Span {
        match self {
            // ... other cases
            Expr::CallWithKwargs { span, .. } => span,
        }
    }
}
```

### 2. Parser Implementation (`src/parser.rs`)

**Challenge**: Parsing function calls that distinguish between positional and keyword arguments with proper validation.

**Solution**: Extended the `finish_call` method with intelligent argument parsing:

```rust
fn finish_call(&mut self, callee: Expr) -> Result<Expr, ParseError> {
    let mut args = Vec::new();
    let mut kwargs = Vec::new();
    let mut seen_keyword = false;

    if !self.check(&TokenType::RightParen) {
        loop {
            // Try to parse as keyword argument first
            if self.check(&TokenType::Identifier) && self.peek_ahead(1).token_type == TokenType::Equal {
                seen_keyword = true;
                
                // Parse keyword argument
                let name_token = self.advance();
                self.consume(TokenType::Equal, "Expected '=' after parameter name")?;
                let value = self.assignment()?;
                
                kwargs.push(KeywordArg {
                    name: name_token.lexeme.clone(),
                    value: Box::new(value),
                    span: name_token.span.clone(),
                });
            } else {
                // Parse as positional argument
                if seen_keyword {
                    return Err(ParseError::new(
                        "Positional arguments must come before keyword arguments".to_string(),
                        self.current().span.clone(),
                    ));
                }
                args.push(self.assignment()?);
            }

            if !self.match_token(&TokenType::Comma) {
                break;
            }
        }
    }

    let right_paren = self.consume(TokenType::RightParen, "Expected ')' after arguments")?;
    let span = Span::new(callee.span().start, right_paren.span.end);

    // Choose appropriate AST node
    if kwargs.is_empty() {
        Ok(Expr::Call { callee: Box::new(callee), args, span })
    } else {
        Ok(Expr::CallWithKwargs { callee: Box::new(callee), args, kwargs, span })
    }
}
```

**Key Design Decisions:**
- **Lookahead Parsing**: Uses `peek_ahead()` to detect `identifier = value` pattern
- **Order Validation**: Enforces positional arguments before keyword arguments
- **Smart AST Selection**: Uses `Call` for positional-only, `CallWithKwargs` when keywords present
- **Error Recovery**: Provides clear error messages for invalid argument order

**Helper method for lookahead:**
```rust
fn peek_ahead(&self, distance: usize) -> &Token {
    let index = (self.current_index + distance).min(self.tokens.len() - 1);
    &self.tokens[index]
}
```

### 3. Built-in Function Framework (`src/evaluator.rs`)

**Challenge**: Creating a framework that allows built-in functions to specify parameter names, default values, and validation.

**Solution**: Enhanced built-in function system with parameter metadata:

```rust
#[derive(Debug, Clone)]
pub struct Parameter {
    pub name: String,
    pub default_value: Option<Value>,
    pub required: bool,
}

impl Parameter {
    pub fn required(name: &str) -> Self {
        Self {
            name: name.to_string(),
            default_value: None,
            required: true,
        }
    }

    pub fn optional(name: &str, default: Value) -> Self {
        Self {
            name: name.to_string(),
            default_value: Some(default),
            required: false,
        }
    }
}
```

**Built-in function registration with kwargs support:**
```rust
fn get_builtin_parameters(name: &str) -> Option<Vec<Parameter>> {
    match name {
        "__builtin_divmod__" => Some(vec![
            Parameter::required("a"),
            Parameter::required("b"), 
            Parameter::optional("round_mode", Value::String("down".to_string())),
        ]),
        "__builtin_case__" => None, // Variable arguments, no kwargs
        "__builtin_print__" => Some(vec![
            Parameter::required("value"),
            Parameter::optional("sep", Value::String(" ".to_string())),
            Parameter::optional("end", Value::String("\n".to_string())),
        ]),
        _ => None,
    }
}
```

### 4. Argument Resolution (`src/evaluator.rs`)

**Challenge**: Resolving function calls by matching positional and keyword arguments to parameter definitions.

**Solution**: Comprehensive argument resolution algorithm:

```rust
fn resolve_function_arguments(
    &mut self,
    parameters: &[Parameter],
    args: &[Expr],
    kwargs: &[KeywordArg],
    call_span: &Span,
) -> Result<Vec<Value>, EvalError> {
    let mut resolved_args = vec![Value::Nil; parameters.len()];
    let mut provided = vec![false; parameters.len()];

    // Step 1: Process positional arguments
    if args.len() > parameters.len() {
        return Err(EvalError::new(
            format!("Too many positional arguments: expected at most {}, got {}", 
                    parameters.len(), args.len()),
            call_span.clone(),
        ));
    }

    for (i, arg) in args.iter().enumerate() {
        resolved_args[i] = self.evaluate_expression(arg)?;
        provided[i] = true;
    }

    // Step 2: Process keyword arguments
    for kwarg in kwargs {
        // Find parameter by name
        let param_index = parameters.iter().position(|p| p.name == kwarg.name);
        
        match param_index {
            Some(index) => {
                if provided[index] {
                    return Err(EvalError::new(
                        format!("Duplicate argument for parameter '{}'", kwarg.name),
                        kwarg.span.clone(),
                    ));
                }
                
                resolved_args[index] = self.evaluate_expression(&kwarg.value)?;
                provided[index] = true;
            }
            None => {
                let valid_params: Vec<&str> = parameters.iter().map(|p| p.name.as_str()).collect();
                return Err(EvalError::new(
                    format!("Unknown parameter '{}'. Valid parameters: {}", 
                            kwarg.name, valid_params.join(", ")),
                    kwarg.span.clone(),
                ));
            }
        }
    }

    // Step 3: Apply defaults and validate required parameters
    for (i, param) in parameters.iter().enumerate() {
        if !provided[i] {
            if param.required {
                return Err(EvalError::new(
                    format!("Missing required parameter '{}'", param.name),
                    call_span.clone(),
                ));
            } else if let Some(default) = &param.default_value {
                resolved_args[i] = default.clone();
            }
        }
    }

    Ok(resolved_args)
}
```

**Key Features:**
- **Positional First**: Processes positional arguments in order
- **Keyword Matching**: Matches keyword arguments by parameter name
- **Duplicate Detection**: Prevents same parameter being specified twice
- **Default Application**: Uses default values for unspecified optional parameters
- **Comprehensive Validation**: Clear error messages for all failure cases

### 5. Function Call Evaluation (`src/evaluator.rs`)

**Challenge**: Integrating kwargs evaluation into the main expression evaluation pipeline.

**Solution**: Enhanced expression evaluator with kwargs support:

```rust
fn evaluate_expression(&mut self, expr: &Expr) -> Result<Value, EvalError> {
    match expr {
        // ... existing expression types
        Expr::CallWithKwargs { callee, args, kwargs, span } => {
            if let Expr::Variable { name, .. } = callee.as_ref() {
                // Handle built-in functions with kwargs
                if let Some(builtin_name) = self.get_builtin_name(name) {
                    if let Some(parameters) = get_builtin_parameters(&builtin_name) {
                        let resolved_args = self.resolve_function_arguments(
                            &parameters, args, kwargs, span
                        )?;
                        return self.call_builtin_with_resolved_args(&builtin_name, &resolved_args);
                    }
                }
            }
            
            // Handle user-defined functions (future implementation)
            Err(EvalError::new(
                "User-defined functions with kwargs not yet implemented".to_string(),
                span.clone(),
            ))
        }
        // ... other expression types
    }
}
```

### 6. Enhanced Built-in Functions

**Example: `divmod` with kwargs support:**

```rust
fn call_builtin_with_resolved_args(&mut self, name: &str, args: &[Value]) -> Result<Value, EvalError> {
    match name {
        "__builtin_divmod__" => {
            let a = match &args[0] {
                Value::Int(n) => *n as f64,
                Value::Double(n) => *n,
                _ => return Err(EvalError::new(
                    "divmod() first argument must be a number".to_string(),
                    Span::default(),
                )),
            };
            
            let b = match &args[1] {
                Value::Int(n) => *n as f64,
                Value::Double(n) => *n,
                _ => return Err(EvalError::new(
                    "divmod() second argument must be a number".to_string(),
                    Span::default(),
                )),
            };

            let round_mode = match &args[2] {
                Value::String(mode) => mode.as_str(),
                _ => return Err(EvalError::new(
                    "divmod() round_mode must be a string".to_string(),
                    Span::default(),
                )),
            };

            let (quotient, remainder) = match round_mode {
                "down" => ((a / b).floor(), a % b),
                "up" => ((a / b).ceil(), a - b * (a / b).ceil()),
                "nearest" => {
                    let q = (a / b).round();
                    (q, a - b * q)
                }
                _ => return Err(EvalError::new(
                    "Invalid round_mode. Use 'down', 'up', or 'nearest'".to_string(),
                    Span::default(),
                )),
            };

            Ok(Value::Tuple(vec![
                Value::Double(quotient),
                Value::Double(remainder),
            ]))
        }
        // ... other built-ins
    }
}
```

## Error Handling Strategy

### 1. Parse-Time Errors
```rust
// Invalid argument order
"Positional arguments must come before keyword arguments"

// Missing components
"Expected '=' after parameter name"
"Expected ')' after arguments"
```

### 2. Resolution Errors
```rust
// Too many arguments
"Too many positional arguments: expected at most 2, got 3"

// Duplicate arguments
"Duplicate argument for parameter 'round_mode'"

// Unknown parameters
"Unknown parameter 'mode'. Valid parameters: a, b, round_mode"

// Missing required
"Missing required parameter 'a'"
```

### 3. Type Validation Errors
```rust
// Wrong argument types
"divmod() first argument must be a number"
"divmod() round_mode must be a string"

// Invalid option values
"Invalid round_mode. Use 'down', 'up', or 'nearest'"
```

## Testing Strategy

### 1. Basic Kwargs Functionality
```javascript
// Test default values
result = divmod(10, 3)                    // Uses default "down"
// Expected: (3.0, 1.0)

// Test keyword override
result = divmod(10, 3, round_mode="up")   // Override to "up"
// Expected: (4.0, -2.0)

// Test different modes
result = divmod(7, 2, round_mode="nearest")
// Expected: (3.0, 1.0)
```

### 2. Argument Order Validation
```javascript
// Valid: positional then keyword
divmod(10, 3, round_mode="up")           // ✅ Valid

// Invalid: keyword then positional  
divmod(a=10, 3)                          // ❌ Parse error
```

### 3. Error Conditions
```javascript
// Missing required parameter
divmod(round_mode="up")                  // ❌ Missing 'a' and 'b'

// Unknown parameter
divmod(10, 3, mode="up")                 // ❌ Unknown 'mode'

// Duplicate parameter
divmod(10, a=5, b=3)                     // ❌ Duplicate 'a' (position 0)
```

### 4. Type Validation
```javascript
// Wrong types
divmod("10", 3)                          // ❌ String not number
divmod(10, 3, round_mode=123)           // ❌ Number not string

// Invalid enum value
divmod(10, 3, round_mode="invalid")     // ❌ Invalid mode
```

## Real-World Usage Examples

### 1. Mathematical Functions
```javascript
// Basic division with different rounding modes
q1, r1 = divmod(17, 5)                   // (3.0, 2.0) - default down
q2, r2 = divmod(17, 5, round_mode="up")  // (4.0, -3.0) - round up  
q3, r3 = divmod(17, 5, round_mode="nearest") // (3.0, 2.0) - round nearest
```

### 2. Print Function Enhancement (Future)
```javascript
// Enhanced print with formatting options
print("Hello")                           // Default: "Hello\n"
print("Hello", end="")                   // No newline: "Hello"
print("A", "B", "C", sep="-")           // Custom separator: "A-B-C\n"
print("Debug:", x, sep=" ", end="\n\n") // Custom sep and end
```

### 3. String Manipulation (Future Extensions)
```javascript
// String formatting with kwargs
format("{name} is {age} years old", name="Alice", age=30)
// Result: "Alice is 30 years old"

replace("hello world", old="world", new="universe")
// Result: "hello universe"
```

## Architecture Benefits

### 1. **Flexibility**
- Functions can evolve without breaking existing calls
- Optional parameters with sensible defaults
- Named parameters improve code clarity

### 2. **Extensibility**
- Easy to add new parameters to existing functions
- Parameter validation framework supports type checking
- Clear error messages guide correct usage

### 3. **Performance**
- Argument resolution happens once per call
- Default values are pre-computed and stored
- Minimal runtime overhead for positional-only calls

### 4. **Maintainability**  
- Parameter definitions are centralized and documented
- Type checking is systematic and comprehensive
- Error handling follows consistent patterns

## Future Extensions

### 1. **Variable Arguments (\*args)**
```rust
Parameter::variadic("args")  // Collects remaining positional args
```

### 2. **Keyword-Only Parameters**
```rust  
Parameter::keyword_only("verbose", Value::Boolean(false))
```

### 3. **Parameter Documentation**
```rust
pub struct Parameter {
    pub name: String,
    pub default_value: Option<Value>,
    pub required: bool,
    pub doc: Option<String>,        // New: parameter documentation
    pub type_hint: Option<String>,  // New: expected type
}
```

### 4. **User-Defined Function Support**
The architecture fully supports extending kwargs to user-defined functions:

```javascript
// Future: user-defined function with kwargs
function greet(name, greeting="Hello", punctuation="!") {
    print(greeting + " " + name + punctuation)
}

greet("Alice")                              // "Hello Alice!"
greet("Bob", greeting="Hi")                 // "Hi Bob!"  
greet(name="Carol", punctuation=".")        // "Hello Carol."
```

This implementation provides a robust foundation for professional-grade function call semantics with excellent error handling and clear architectural patterns.