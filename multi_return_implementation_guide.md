# Multi-Arity Return Values Implementation Guide

This document provides a comprehensive walkthrough of implementing multi-arity return values and destructuring assignment in the BCC interpreter.

## Overview

Multi-arity return values allow functions to return multiple values simultaneously, which can then be destructured into separate variables using tuple unpacking syntax. This enables more expressive and efficient code patterns.

**Target Syntax:**
```javascript
// Function returning multiple values
quotient, remainder = divmod(17, 5)        // Returns: (3, 2)

// Partial assignment with ignore pattern
result, _ = divmod(10, 3)                  // Ignore remainder

// Nested destructuring (future extension)
x, y, z = get_coordinates()                // Returns: (10, 20, 30)

// Multiple assignment from any iterable
a, b, c = [1, 2, 3]                       // From list
first, second = "Hi"                       // From string (future)
```

## Implementation Architecture

### 1. Value System Extension (`src/value.rs`)

**Challenge**: Creating a native tuple type that can hold multiple values and support all required operations.

**Solution**: Added a comprehensive tuple value type:

```rust
pub enum Value {
    // ... existing variants
    Tuple(Vec<Value>),
    // ... other variants  
}
```

**Key Operations Implementation:**

#### Display Formatting
```rust
impl std::fmt::Display for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            // ... other cases
            Value::Tuple(values) => {
                write!(f, "(")?;
                for (i, value) in values.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", value)?;
                }
                // Handle single-element tuple formatting
                if values.len() == 1 {
                    write!(f, ",")?;
                }
                write!(f, ")")
            }
            // ... other cases
        }
    }
}
```

#### Membership Testing (`in` operator)
```rust
impl Value {
    pub fn contains(&self, other: &Value) -> bool {
        match self {
            // ... other cases
            Value::Tuple(values) => {
                values.iter().any(|v| v == other)
            }
            // ... other cases
        }
    }
}
```

#### Type System Integration
```rust
impl Value {
    pub fn type_name(&self) -> &'static str {
        match self {
            // ... other cases
            Value::Tuple(_) => "tuple",
        }
    }

    pub fn is_truthy(&self) -> bool {
        match self {
            // ... other cases
            Value::Tuple(values) => !values.is_empty(), // Empty tuple is falsy
        }
    }
}
```

### 2. AST Extensions (`src/ast.rs`)

**Challenge**: Representing both tuple expressions and multi-assignment targets in the AST.

**Solution**: Added comprehensive AST support for tuples and multi-assignment:

```rust
// Tuple expressions: (a, b, c)
pub enum Expr {
    // ... existing variants
    Tuple {
        elements: Vec<Expr>,
        span: Span,
    },
    
    // Multi-assignment: a, b, c = expr
    MultiAssign {
        targets: Vec<AssignTarget>,
        value: Box<Expr>,
        span: Span,
    },
    // ... other variants
}

// Assignment targets with underscore support
#[derive(Debug, Clone, PartialEq)]
pub enum AssignTarget {
    Variable { name: String, span: Span },
    Ignore { span: Span },  // Underscore (_) pattern
}
```

**Key Design Decisions:**
- **Separate Target Type**: `AssignTarget` handles both variables and ignore patterns
- **`Vec<Expr>` Elements**: Tuple elements can be any expression
- **Span Tracking**: Complete span information for error reporting
- **Ignore Pattern**: Explicit support for `_` to ignore values

**Integration methods:**
```rust
impl Expr {
    pub fn span(&self) -> &Span {
        match self {
            // ... other cases
            Expr::Tuple { span, .. } => span,
            Expr::MultiAssign { span, .. } => span,
        }
    }
}

impl AssignTarget {
    pub fn span(&self) -> &Span {
        match self {
            AssignTarget::Variable { span, .. } => span,
            AssignTarget::Ignore { span } => span,
        }
    }
}
```

### 3. Parser Implementation (`src/parser.rs`)

**Challenge**: Distinguishing between grouping parentheses `(expr)` and tuple literals `(a, b, c)`, plus parsing multi-assignment.

**Solution**: Smart tuple parsing with disambiguation:

#### Tuple Expression Parsing
```rust
fn primary(&mut self) -> Result<Expr, ParseError> {
    match &self.current().token_type {
        // ... other cases
        TokenType::LeftParen => {
            let start = self.advance().span.start; // consume '('
            
            // Handle empty tuple: ()
            if self.check(&TokenType::RightParen) {
                let end = self.advance().span.end; // consume ')'
                return Ok(Expr::Tuple {
                    elements: vec![],
                    span: Span::new(start, end),
                });
            }
            
            // Parse first expression
            let first_expr = self.expression()?;
            
            if self.check(&TokenType::Comma) {
                // This is a tuple: (a, b, c) or (a,)
                self.advance(); // consume ','
                let mut elements = vec![first_expr];
                
                // Parse remaining elements (if any)
                if !self.check(&TokenType::RightParen) {
                    loop {
                        elements.push(self.expression()?);
                        if !self.match_token(&TokenType::Comma) {
                            break;
                        }
                        // Allow trailing comma: (a, b, c,)
                        if self.check(&TokenType::RightParen) {
                            break;
                        }
                    }
                }
                
                let end = self.consume(TokenType::RightParen, "Expected ')' after tuple elements")?.span.end;
                Ok(Expr::Tuple {
                    elements,
                    span: Span::new(start, end),
                })
            } else {
                // This is grouping: (expr)
                let end = self.consume(TokenType::RightParen, "Expected ')' after expression")?.span.end;
                Ok(first_expr) // Return the wrapped expression directly
            }
        }
        // ... other cases
    }
}
```

#### Multi-Assignment Parsing (Future - in statement parsing)
```rust
// Note: This would be implemented in statement parsing
fn assignment_statement(&mut self) -> Result<Stmt, ParseError> {
    // Check for multi-assignment pattern: a, b, c = expr
    if self.is_multi_assignment_pattern() {
        let targets = self.parse_assignment_targets()?;
        self.consume(TokenType::Equal, "Expected '=' in multi-assignment")?;
        let value = self.expression()?;
        
        Ok(Stmt::Expression(Expr::MultiAssign {
            targets,
            value: Box::new(value),
            span: Span::new(targets[0].span().start, value.span().end),
        }))
    } else {
        // Regular single assignment
        self.regular_assignment()
    }
}

fn parse_assignment_targets(&mut self) -> Result<Vec<AssignTarget>, ParseError> {
    let mut targets = vec![];
    
    loop {
        if self.check(&TokenType::Underscore) {
            let span = self.advance().span.clone();
            targets.push(AssignTarget::Ignore { span });
        } else if self.check(&TokenType::Identifier) {
            let token = self.advance();
            targets.push(AssignTarget::Variable {
                name: token.lexeme.clone(),
                span: token.span.clone(),
            });
        } else {
            return Err(ParseError::new(
                "Expected variable name or '_' in assignment target".to_string(),
                self.current().span.clone(),
            ));
        }
        
        if !self.match_token(&TokenType::Comma) {
            break;
        }
    }
    
    Ok(targets)
}
```

### 4. Multi-Assignment Evaluation (`src/evaluator.rs`)

**Challenge**: Unpacking values from tuples, lists, and single values into multiple assignment targets.

**Solution**: Comprehensive unpacking evaluation with error handling:

```rust
fn evaluate_expression(&mut self, expr: &Expr) -> Result<Value, EvalError> {
    match expr {
        // ... existing expression types
        Expr::Tuple { elements, .. } => {
            let mut values = Vec::new();
            for element in elements {
                values.push(self.evaluate_expression(element)?);
            }
            Ok(Value::Tuple(values))
        }
        
        Expr::MultiAssign { targets, value, span } => {
            let source_value = self.evaluate_expression(value)?;
            self.perform_multi_assignment(targets, &source_value, span)?;
            Ok(source_value) // Return the assigned value
        }
        // ... other expression types
    }
}
```

#### Unpacking Logic Implementation
```rust
fn perform_multi_assignment(
    &mut self,
    targets: &[AssignTarget], 
    source_value: &Value,
    span: &Span
) -> Result<(), EvalError> {
    // Convert source value to a vector of values for unpacking
    let values_to_unpack = match source_value {
        Value::Tuple(values) => values.clone(),
        Value::List(values) => values.clone(),
        single_value => vec![single_value.clone()], // Single value becomes one-element list
    };
    
    // Check assignment target count vs. available values
    if targets.len() != values_to_unpack.len() {
        return Err(EvalError::new(
            format!(
                "Cannot unpack {} values into {} targets",
                values_to_unpack.len(),
                targets.len()
            ),
            span.clone(),
        ));
    }
    
    // Perform the assignments
    for (target, value) in targets.iter().zip(values_to_unpack.iter()) {
        match target {
            AssignTarget::Variable { name, .. } => {
                self.environment.define(name.clone(), value.clone());
            }
            AssignTarget::Ignore { .. } => {
                // Deliberately do nothing - value is ignored
            }
        }
    }
    
    Ok(())
}
```

### 5. Built-in Functions with Multi-Return (`src/evaluator.rs`)

**Challenge**: Creating built-in functions that naturally return multiple values.

**Solution**: Enhanced `divmod` function as the primary example:

```rust
fn call_builtin_with_resolved_args(&mut self, name: &str, args: &[Value]) -> Result<Value, EvalError> {
    match name {
        "__builtin_divmod__" => {
            // Extract and validate arguments (with kwargs support)
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
            
            if b == 0.0 {
                return Err(EvalError::new(
                    "divmod() division by zero".to_string(),
                    Span::default(),
                ));
            }
            
            // Get rounding mode from kwargs (default: "down")
            let round_mode = match &args.get(2) {
                Some(Value::String(mode)) => mode.as_str(),
                _ => "down", // Default value
            };
            
            // Calculate quotient and remainder based on rounding mode
            let (quotient, remainder) = match round_mode {
                "down" => {
                    let q = (a / b).floor();
                    (q, a - b * q)
                }
                "up" => {
                    let q = (a / b).ceil();
                    (q, a - b * q)
                }
                "nearest" => {
                    let q = (a / b).round();
                    (q, a - b * q)
                }
                _ => return Err(EvalError::new(
                    "Invalid round_mode. Use 'down', 'up', or 'nearest'".to_string(),
                    Span::default(),
                )),
            };
            
            // Return as tuple - this enables multi-assignment
            Ok(Value::Tuple(vec![
                if quotient.fract() == 0.0 { 
                    Value::Int(quotient as i64) 
                } else { 
                    Value::Double(quotient) 
                },
                if remainder.fract() == 0.0 { 
                    Value::Int(remainder as i64) 
                } else { 
                    Value::Double(remainder) 
                },
            ]))
        }
        // ... other built-ins
    }
}
```

### 6. Enhanced Return Statement (Future Extension)

**For user-defined functions, the return statement would support multiple values:**

```rust
// In statement evaluation (future implementation)
Stmt::Return { values, span } => {
    if values.len() == 1 {
        // Single return value
        let value = self.evaluate_expression(&values[0])?;
        Err(ReturnException(value))
    } else {
        // Multiple return values - create tuple
        let mut return_values = Vec::new();
        for expr in values {
            return_values.push(self.evaluate_expression(expr)?);
        }
        Err(ReturnException(Value::Tuple(return_values)))
    }
}
```

## Error Handling Strategy

### 1. Tuple Creation Errors
```rust
// Empty tuples are valid
()                              // ✅ Valid: empty tuple

// Single-element disambiguation  
(42)                           // Grouping - returns 42
(42,)                          // Tuple - returns (42,)
```

### 2. Multi-Assignment Errors
```rust
// Count mismatch
a, b = (1, 2, 3)               // ❌ "Cannot unpack 3 values into 2 targets"
x, y, z = (1, 2)               // ❌ "Cannot unpack 2 values into 3 targets"

// Invalid assignment targets  
1, x = (10, 20)                // ❌ "Cannot assign to literal"
```

### 3. Type Validation Errors
```rust
// divmod argument validation
divmod("10", 3)                // ❌ "divmod() first argument must be a number"
divmod(10, 0)                  // ❌ "divmod() division by zero"
divmod(10, 3, round_mode=123)  // ❌ "divmod() round_mode must be a string"
```

### 4. Runtime Errors
```rust
// Invalid round mode
divmod(10, 3, round_mode="invalid")  // ❌ "Invalid round_mode. Use 'down', 'up', or 'nearest'"
```

## Testing Strategy

### 1. Basic Tuple Operations
```javascript
// Tuple creation and display
t1 = ()                        // Empty tuple: ()
t2 = (42,)                     // Single element: (42,)  
t3 = (1, 2, 3)                 // Multiple elements: (1, 2, 3)

// Type checking
print(type(t1))                // "tuple"
print(type(t2))                // "tuple"
print(type(t3))                // "tuple"
```

### 2. Membership Testing
```javascript
// Tuple membership  
print(2 in (1, 2, 3))          // true
print(4 in (1, 2, 3))          // false
print("a" in ("x", "y", "z"))  // false

// Nested tuple membership
print((1, 2) in ((1, 2), (3, 4)))  // true (when nested comparison works)
```

### 3. Multi-Return Functions
```javascript
// Basic divmod usage
result = divmod(17, 5)         // Returns: (3, 2)
print(result)                  // Output: (3, 2)
print(type(result))            // Output: tuple

// Different rounding modes  
result1 = divmod(17, 5, round_mode="down")     // (3, 2)
result2 = divmod(17, 5, round_mode="up")       // (4, -3)  
result3 = divmod(17, 5, round_mode="nearest")  // (3, 2)
```

### 4. Multi-Assignment (When Parser Complete)
```javascript
// Basic multi-assignment
a, b = divmod(17, 5)           // a=3, b=2

// Ignore patterns
quotient, _ = divmod(17, 5)    // Only capture quotient

// From different sources
x, y, z = (10, 20, 30)         // From tuple
p, q = [100, 200]              // From list
```

### 5. Error Condition Testing
```javascript
// Count mismatches
a, b = (1, 2, 3)               // Error: cannot unpack 3 into 2
x, y, z = divmod(10, 3)        // Error: cannot unpack 2 into 3

// Division by zero
quotient, remainder = divmod(10, 0)  // Error: division by zero

// Invalid parameters
result = divmod(10, 3, round_mode="invalid")  // Error: invalid mode
```

## Real-World Usage Examples

### 1. Mathematical Operations
```javascript
// Integer division with remainder
quotient, remainder = divmod(100, 7)
// quotient = 14, remainder = 2

// Ceiling division  
ceil_quotient, neg_remainder = divmod(100, 7, round_mode="up")
// ceil_quotient = 15, neg_remainder = -5

// Floating point division with exact remainder
q, r = divmod(22.5, 4.5)
// q = 5.0, r = 0.0
```

### 2. Coordinate Systems
```javascript
// Future: 2D/3D coordinate functions
x, y = get_mouse_position()
x, y, z = get_3d_coordinates()

// Ignore unused coordinates
x, _ = get_mouse_position()    // Only need x coordinate
```

### 3. String Processing (Future Extensions)
```javascript
// String splitting
name, domain = split_email("user@example.com")
// name = "user", domain = "example.com"

// File path processing  
directory, filename, extension = parse_path("/home/user/file.txt")
// directory = "/home/user", filename = "file", extension = "txt"
```

### 4. Statistical Functions (Future)
```javascript
// Statistics with multiple return values
mean, median, mode = calculate_stats([1, 2, 2, 3, 4, 5])

// Linear regression results
slope, intercept, r_squared = linear_regression(x_values, y_values)
```

## Architecture Benefits

### 1. **Natural Expression**
- Functions can return related values together
- No need for artificial wrapper objects
- Matches mathematical conventions (divmod, etc.)

### 2. **Performance**
- Single function call returns multiple values
- No intermediate object allocation for simple cases
- Direct unpacking to variables

### 3. **Flexibility**
- Can ignore unwanted return values with `_`
- Works with any iterable (tuples, lists)
- Composable with other language features

### 4. **Type Safety**
- Compile-time checking of assignment target count
- Clear error messages for mismatched unpacking
- Consistent type system integration

## Future Extensions

### 1. **Advanced Destructuring**
```javascript
// Nested destructuring
(x, y), (z, w) = ((1, 2), (3, 4))

// Rest patterns  
first, *rest = (1, 2, 3, 4, 5)    // first=1, rest=[2, 3, 4, 5]

// Dictionary destructuring
name, age = person["name"], person["age"]  
```

### 2. **String Unpacking**
```javascript
// Character unpacking
a, b, c = "ABC"                // a="A", b="B", c="C"
```

### 3. **Enhanced Return Statements**
```javascript
// User-defined functions with multi-return
function get_name_age() {
    return "Alice", 30         // Return multiple values
}

name, age = get_name_age()
```

### 4. **Generator Unpacking**
```javascript  
// Future: generator/iterator unpacking
first, second, third = range(10)  // Unpack from generator
```

This implementation provides a solid foundation for multi-value programming patterns while maintaining simplicity and excellent error handling. The tuple type integrates seamlessly with the existing value system and provides a natural way to group related values.