# Case Function Implementation Guide

This document provides a comprehensive walkthrough of implementing the `case()` built-in function and property access in the BCC interpreter.

## Overview

The `case()` function provides conditional selection similar to SQL's CASE statement or functional pattern matching. It takes alternating condition-result pairs and returns the first matching result wrapped in a `CaseResult` struct, accessible via `.result` property.

**Usage Pattern:**
```javascript
result = case(condition1, value1, condition2, value2, ..., default_condition, default_value)
output = result.result
```

## Implementation Architecture

### 1. Value System Extension (`src/value.rs`)

**Challenge**: Adding a new runtime value type that can hold any other value as its result.

**Solution**: Extended the `Value` enum with a `CaseResult` variant:

```rust
#[derive(Debug, Clone, PartialEq)]
pub struct CaseResult {
    pub result: Box<Value>,
}

pub enum Value {
    // ... existing variants
    CaseResult(CaseResult),
}
```

**Key Design Decisions:**
- **`Box<Value>`**: Used to avoid recursive type definition issues since `Value` contains `CaseResult` which contains `Value`
- **Public `result` field**: Direct access for the evaluator, keeping implementation simple
- **Derives**: `Debug`, `Clone`, `PartialEq` for consistency with other value types

**Integration Points:**
```rust
impl Value {
    pub fn is_truthy(&self) -> bool {
        match self {
            // ... other cases
            Value::CaseResult(case_result) => case_result.result.is_truthy(),
        }
    }

    pub fn type_name(&self) -> &'static str {
        match self {
            // ... other cases  
            Value::CaseResult(_) => "case_result",
        }
    }
}

impl std::fmt::Display for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            // ... other cases
            Value::CaseResult(case_result) => write!(f, "case_result({})", case_result.result),
        }
    }
}
```

### 2. AST Extension for Property Access (`src/ast.rs`)

**Challenge**: Adding property access syntax (`object.property`) to the expression grammar.

**Solution**: Added `PropertyAccess` variant to the `Expr` enum:

```rust
pub enum Expr {
    // ... existing variants
    PropertyAccess {
        object: Box<Expr>,
        property: String,  
        span: Span,
    },
}
```

**Key Design Decisions:**
- **`Box<Expr>`**: The object being accessed is itself an expression
- **`String` property**: Property names are identifiers, owned strings fit the architecture
- **`Span` tracking**: Essential for error reporting and IDE integration

**Integration with span tracking:**
```rust
impl Expr {
    pub fn span(&self) -> &Span {
        match self {
            // ... other cases
            Expr::PropertyAccess { span, .. } => span,
        }
    }
}
```

### 3. Parser Extension (`src/parser.rs`)

**Challenge**: Extending the expression parser to handle both function calls and property access with correct precedence.

**Solution**: Modified the `call()` method to handle both patterns:

```rust
fn call(&mut self) -> Result<Expr, ParseError> {
    let mut expr = self.primary()?;

    loop {
        match &self.current().token_type {
            TokenType::LeftParen => {
                // Function call logic
                expr = self.finish_call(expr)?;
            }
            TokenType::Dot => {
                // Property access logic
                self.advance(); // consume '.'
                let property = self.consume_identifier("Expected property name after '.'")?;
                let span = Span::new(expr.span().start, property.span.end);
                expr = Expr::PropertyAccess {
                    object: Box::new(expr),
                    property: property.lexeme,
                    span,
                };
            }
            _ => break,
        }
    }

    Ok(expr)
}
```

**Key Design Decisions:**
- **Loop structure**: Allows chaining like `obj.prop.method()`
- **Left-to-right associativity**: `a.b.c` parses as `(a.b).c`
- **Span calculation**: Combines object and property spans for accurate error reporting
- **Precedence**: Property access has same precedence as function calls (high precedence, left associative)

**Error Handling:**
```rust
fn consume_identifier(&mut self, message: &str) -> Result<Token, ParseError> {
    if let TokenType::Identifier = &self.current().token_type {
        Ok(self.advance())
    } else {
        Err(ParseError::new(
            message.to_string(),
            self.current().span.clone(),
        ))
    }
}
```

### 4. Built-in Function Implementation (`src/evaluator.rs`)

**Challenge**: Implementing the core `case()` logic with lazy evaluation and proper error handling.

**Solution**: Added to the built-in function dispatch:

```rust
fn call_builtin_function(&mut self, name: &str, args: &[Expr]) -> Result<Value, EvalError> {
    match name {
        // ... existing built-ins
        "__builtin_case__" => {
            // Validate argument count (must be even - condition/result pairs)
            if args.len() % 2 != 0 {
                return Err(EvalError::new(
                    "case() requires an even number of arguments (condition-result pairs)".to_string(),
                    args.last().unwrap().span().clone(),
                ));
            }

            // Lazy evaluation: check conditions in order
            for chunk in args.chunks(2) {
                let condition = self.evaluate_expression(&chunk[0])?;
                if condition.is_truthy() {
                    let result = self.evaluate_expression(&chunk[1])?;
                    return Ok(Value::CaseResult(CaseResult {
                        result: Box::new(result),
                    }));
                }
            }

            // No conditions matched - return nil wrapped in CaseResult
            Ok(Value::CaseResult(CaseResult {
                result: Box::new(Value::Nil),
            }))
        }
        // ... other built-ins
    }
}
```

**Key Design Decisions:**
- **Lazy evaluation**: Only evaluate condition/result pairs until first match
- **Even argument validation**: Must have pairs, not odd numbers of arguments
- **Default case**: Return `nil` if no conditions match (wrapped in `CaseResult`)
- **Early return**: Stop at first truthy condition for efficiency

### 5. Property Access Evaluation (`src/evaluator.rs`)

**Challenge**: Implementing property access that's extensible but currently only supports `CaseResult.result`.

**Solution**: Added property access evaluation:

```rust
fn evaluate_expression(&mut self, expr: &Expr) -> Result<Value, EvalError> {
    match expr {
        // ... existing expression types
        Expr::PropertyAccess { object, property, span } => {
            let obj_value = self.evaluate_expression(object)?;
            
            match (&obj_value, property.as_str()) {
                (Value::CaseResult(case_result), "result") => {
                    Ok((*case_result.result).clone())
                }
                (_, "result") => {
                    Err(EvalError::new(
                        format!("Property access '.result' is only supported on case_result objects, not {}", 
                                obj_value.type_name()),
                        span.clone(),
                    ))
                }
                (Value::CaseResult(_), prop) => {
                    Err(EvalError::new(
                        format!("Property '{}' is not supported on case_result objects. Did you mean '.result'?", prop),
                        span.clone(),
                    ))
                }
                (obj, prop) => {
                    Err(EvalError::new(
                        format!("Property access is not supported for {} objects", obj.type_name()),
                        span.clone(),
                    ))
                }
            }
        }
        // ... other expression types
    }
}
```

**Key Design Decisions:**
- **Pattern matching**: Exhaustive matching on object type and property name
- **Helpful error messages**: Different messages for different error scenarios
- **Clone semantics**: Return owned values, consistent with interpreter architecture
- **Extensibility**: Easy to add support for other object types and properties later

### 6. Function Registration (`src/evaluator.rs`)

**Challenge**: Making the `case()` function available in the global environment.

**Solution**: Added to the built-in function registration in `Environment::new()`:

```rust
impl Environment {
    pub fn new() -> Self {
        let mut globals = HashMap::new();
        
        // ... existing built-ins
        globals.insert("case".to_string(), Value::BuiltinFunction("__builtin_case__".to_string()));
        
        // ... rest of initialization
    }
}
```

## Error Handling Strategy

### 1. Argument Validation
```rust
if args.len() % 2 != 0 {
    return Err(EvalError::new(
        "case() requires an even number of arguments (condition-result pairs)".to_string(),
        args.last().unwrap().span().clone(),
    ));
}
```

### 2. Property Access Errors
- **Wrong object type**: "Property access '.result' is only supported on case_result objects"
- **Wrong property**: "Property 'foo' is not supported on case_result objects. Did you mean '.result'?"
- **Unsupported type**: "Property access is not supported for int objects"

### 3. Parse Errors
- **Missing property name**: "Expected property name after '.'"
- **Malformed syntax**: Handled by existing parser error recovery

## Testing Strategy

### 1. Basic Functionality Tests
```javascript
// Basic true condition
x = case(true, "yes", false, "no")
assert(x.result == "yes")

// Basic false condition  
x = case(false, "no", true, "yes")
assert(x.result == "yes")

// No match returns nil
x = case(false, "no", false, "never")  
assert(x.result == nil)
```

### 2. Complex Expression Tests
```javascript
// Expressions as conditions
age = 25
status = case(age < 18, "minor", age >= 18, "adult")
assert(status.result == "adult")

// Expressions as results
x = case(true, 10 + 5, false, 20)
assert(x.result == 15)
```

### 3. Error Condition Tests
```javascript
// Odd number of arguments
case(true)  // Error: requires even number of arguments

// Wrong property access
42.result   // Error: Property access not supported for int

// Wrong property name  
x = case(true, "yes")
x.foo       // Error: Property 'foo' not supported, did you mean '.result'?
```

### 4. Edge Cases
```javascript
// Empty case (no arguments)
case()  // Error: requires even number of arguments

// Complex nesting
outer = case(true, case(false, "inner", true, "found"))
assert(outer.result.result == "found")

// Chained property access preparation
// (architecture supports obj.prop.method() for future extension)
```

## Architecture Benefits

### 1. **Modularity**
- Each component (AST, Parser, Evaluator, Value) handles its own concerns
- Changes are localized to appropriate modules
- Easy to understand and maintain

### 2. **Extensibility**  
- Property access system can easily support other object types
- Parser structure supports method calls (`obj.method()`) for future features
- Value system can accommodate new result wrapper types

### 3. **Error Quality**
- Comprehensive error messages with suggestions
- Precise error locations with span tracking  
- Context-aware error handling

### 4. **Performance**
- Lazy evaluation stops at first match
- Minimal memory overhead (Box<Value> only when needed)
- No unnecessary allocations during evaluation

### 5. **Consistency**
- Follows existing owned-string architecture
- Uses same error reporting patterns as rest of interpreter
- Integrates cleanly with existing built-in function system

## Future Extensions

### 1. **Additional Properties**
```rust
match (&obj_value, property.as_str()) {
    (Value::CaseResult(case_result), "result") => Ok((*case_result.result).clone()),
    (Value::CaseResult(case_result), "matched_index") => Ok(Value::Int(case_result.matched_index)),
    // ... other properties
}
```

### 2. **Method Calls**
The property access architecture already supports chaining, making method calls a natural extension:
```javascript
result = case(true, [1, 2, 3], false, [])
length = result.result.len()  // Future: method calls on results
```

### 3. **Other Object Types**
```rust
match (&obj_value, property.as_str()) {
    // Case results
    (Value::CaseResult(case_result), "result") => { /* ... */ }
    
    // Future: Dictionary property access
    (Value::Dictionary(dict), prop) => dict.get(prop),
    
    // Future: List properties  
    (Value::List(list), "length") => Ok(Value::Int(list.len() as i64)),
}
```

This implementation provides a solid foundation that's both immediately useful and easily extensible for future language features.