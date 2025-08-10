# Creating Built-in Functions

This guide explains how to extend the BCC interpreter with built-in functions. Built-ins are functions implemented in Rust that can be called from BCC code, providing access to system functionality and essential operations that would be difficult or impossible to implement in the language itself.

## Table of Contents
1. [Overview](#overview)
2. [Current State](#current-state)
3. [Implementation Strategy](#implementation-strategy)
4. [Step-by-Step Implementation](#step-by-step-implementation)
5. [Built-in Function Examples](#built-in-function-examples)
6. [Best Practices](#best-practices)
7. [Testing Built-ins](#testing-built-ins)
8. [Common Patterns](#common-patterns)

## Overview

Built-in functions are essential for any practical programming language. They provide:

- **System access**: File I/O, network operations, system calls
- **Mathematical functions**: Advanced math beyond basic operators
- **String manipulation**: Regular expressions, formatting, parsing
- **Data structures**: Arrays, maps, sets
- **Utility functions**: Type conversion, debugging, introspection

In BCC, built-ins are implemented as Rust functions that integrate seamlessly with the evaluator's call mechanism.

## Current State

**What exists now**:
- Function call syntax: `foo(arg1, arg2)`
- Call expression parsing: `Expr::Call { callee, args, span }`
- Placeholder evaluation code

**What needs to be added**:
- Built-in function registry
- Native function value type
- Call dispatch mechanism
- Standard library functions

## Implementation Strategy

### 1. Extend Value System

Add a new value type for built-in functions:

```rust
// In value.rs
pub enum Value {
    Nil,
    Bool(bool),
    Int(i64),
    Double(f64),
    String(String),
    BuiltinFunction(BuiltinFunction),  // New!
}

pub type BuiltinFunction = fn(&[Value]) -> Result<Value, String>;
```

### 2. Create Function Registry

Build a registry of available built-in functions:

```rust
// In evaluator.rs or new builtins.rs module
use std::collections::HashMap;

pub struct BuiltinRegistry {
    functions: HashMap<String, BuiltinFunction>,
}

impl BuiltinRegistry {
    pub fn new() -> Self {
        let mut functions = HashMap::new();
        
        // Register standard functions
        functions.insert("len".to_string(), builtin_len);
        functions.insert("type".to_string(), builtin_type);
        functions.insert("str".to_string(), builtin_str);
        functions.insert("int".to_string(), builtin_int);
        functions.insert("double".to_string(), builtin_double);
        
        Self { functions }
    }
    
    pub fn get(&self, name: &str) -> Option<&BuiltinFunction> {
        self.functions.get(name)
    }
}
```

### 3. Integrate with Environment

Make built-ins available as global variables:

```rust
impl Evaluator {
    pub fn new() -> Self {
        let mut evaluator = Self {
            environment: Environment::new(),
        };
        
        // Register built-in functions as global variables
        evaluator.register_builtins();
        evaluator
    }
    
    fn register_builtins(&mut self) {
        let registry = BuiltinRegistry::new();
        
        for (name, function) in registry.functions {
            self.environment.assign(
                &name,
                Value::BuiltinFunction(function),
            ).unwrap();
        }
    }
}
```

## Step-by-Step Implementation

### Step 1: Update Value Type

```rust
// In value.rs
use std::fmt;

pub type BuiltinFunction = fn(&[Value]) -> Result<Value, String>;

#[derive(Debug, Clone)]
pub enum Value {
    Nil,
    Bool(bool),
    Int(i64),
    Double(f64),
    String(String),
    BuiltinFunction {
        name: String,
        function: BuiltinFunction,
    },
}

// Update Display implementation
impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Value::Nil => write!(f, "nil"),
            Value::Bool(b) => write!(f, "{}", b),
            Value::Int(n) => write!(f, "{}", n),
            Value::Double(n) => write!(f, "{}", n),
            Value::String(s) => write!(f, "{}", s),
            Value::BuiltinFunction { name, .. } => write!(f, "<built-in function '{}'>", name),
        }
    }
}

// Update type_name method
impl Value {
    pub fn type_name(&self) -> &'static str {
        match self {
            Value::Nil => "nil",
            Value::Bool(_) => "boolean",
            Value::Int(_) => "int",
            Value::Double(_) => "double",
            Value::String(_) => "string",
            Value::BuiltinFunction { .. } => "function",
        }
    }
}
```

### Step 2: Handle Function Calls in Evaluator

```rust
// In evaluator.rs
impl Evaluator {
    pub fn evaluate_expression(&mut self, expr: &Expr) -> Result<Value, BccError> {
        match expr {
            // ... existing cases ...
            
            Expr::Call { callee, args, span } => {
                let function = self.evaluate_expression(callee)?;
                
                match function {
                    Value::BuiltinFunction { name, function } => {
                        // Evaluate all arguments
                        let mut arg_values = Vec::new();
                        for arg in args {
                            arg_values.push(self.evaluate_expression(arg)?);
                        }
                        
                        // Call the built-in function
                        match function(&arg_values) {
                            Ok(result) => Ok(result),
                            Err(error_msg) => Err(BccError::runtime_error(
                                span.clone(),
                                format!("Error in built-in function '{}': {}", name, error_msg),
                            )),
                        }
                    }
                    _ => Err(BccError::runtime_error(
                        span.clone(),
                        format!("'{}' is not callable", function.type_name()),
                    )),
                }
            }
            
            // ... other cases ...
        }
    }
}
```

### Step 3: Implement Basic Built-in Functions

```rust
// In builtins.rs (new file)
use crate::value::Value;

pub fn builtin_len(args: &[Value]) -> Result<Value, String> {
    if args.len() != 1 {
        return Err(format!("len() takes exactly 1 argument, got {}", args.len()));
    }
    
    match &args[0] {
        Value::String(s) => Ok(Value::Int(s.chars().count() as i64)),
        _ => Err(format!("len() argument must be a string, got {}", args[0].type_name())),
    }
}

pub fn builtin_type(args: &[Value]) -> Result<Value, String> {
    if args.len() != 1 {
        return Err(format!("type() takes exactly 1 argument, got {}", args.len()));
    }
    
    Ok(Value::String(args[0].type_name().to_string()))
}

pub fn builtin_str(args: &[Value]) -> Result<Value, String> {
    if args.len() != 1 {
        return Err(format!("str() takes exactly 1 argument, got {}", args.len()));
    }
    
    Ok(Value::String(args[0].to_string()))
}

pub fn builtin_int(args: &[Value]) -> Result<Value, String> {
    if args.len() != 1 {
        return Err(format!("int() takes exactly 1 argument, got {}", args.len()));
    }
    
    match &args[0] {
        Value::Int(n) => Ok(Value::Int(*n)),
        Value::Double(n) => Ok(Value::Int(*n as i64)),
        Value::String(s) => match s.parse::<i64>() {
            Ok(n) => Ok(Value::Int(n)),
            Err(_) => Err(format!("invalid literal for int(): '{}'", s)),
        },
        Value::Bool(true) => Ok(Value::Int(1)),
        Value::Bool(false) => Ok(Value::Int(0)),
        _ => Err(format!("int() argument must be a string, number, or bool, got {}", args[0].type_name())),
    }
}

pub fn builtin_double(args: &[Value]) -> Result<Value, String> {
    if args.len() != 1 {
        return Err(format!("double() takes exactly 1 argument, got {}", args.len()));
    }
    
    match &args[0] {
        Value::Int(n) => Ok(Value::Double(*n as f64)),
        Value::Double(n) => Ok(Value::Double(*n)),
        Value::String(s) => match s.parse::<f64>() {
            Ok(n) => Ok(Value::Double(n)),
            Err(_) => Err(format!("invalid literal for double(): '{}'", s)),
        },
        Value::Bool(true) => Ok(Value::Double(1.0)),
        Value::Bool(false) => Ok(Value::Double(0.0)),
        _ => Err(format!("double() argument must be a string, number, or bool, got {}", args[0].type_name())),
    }
}
```

### Step 4: Register Built-ins with Evaluator

```rust
// In evaluator.rs
impl Evaluator {
    pub fn new() -> Self {
        let mut evaluator = Self {
            environment: Environment::new(),
        };
        evaluator.register_builtins();
        evaluator
    }
    
    fn register_builtins(&mut self) {
        use crate::builtins::*;
        
        let builtins = vec![
            ("len", builtin_len as BuiltinFunction),
            ("type", builtin_type as BuiltinFunction),
            ("str", builtin_str as BuiltinFunction),
            ("int", builtin_int as BuiltinFunction),
            ("double", builtin_double as BuiltinFunction),
        ];
        
        for (name, function) in builtins {
            self.environment.assign(
                name,
                Value::BuiltinFunction {
                    name: name.to_string(),
                    function,
                },
            ).unwrap();
        }
    }
}
```

## Built-in Function Examples

### Mathematical Functions

```rust
pub fn builtin_abs(args: &[Value]) -> Result<Value, String> {
    if args.len() != 1 {
        return Err(format!("abs() takes exactly 1 argument, got {}", args.len()));
    }
    
    match &args[0] {
        Value::Int(n) => Ok(Value::Int(n.abs())),
        Value::Double(n) => Ok(Value::Double(n.abs())),
        _ => Err(format!("abs() argument must be a number, got {}", args[0].type_name())),
    }
}

pub fn builtin_max(args: &[Value]) -> Result<Value, String> {
    if args.is_empty() {
        return Err("max() expected at least 1 argument, got 0".to_string());
    }
    
    let mut max_val = &args[0];
    
    for arg in &args[1..] {
        match (max_val, arg) {
            (Value::Int(a), Value::Int(b)) => if b > a { max_val = arg; },
            (Value::Double(a), Value::Double(b)) => if b > a { max_val = arg; },
            (Value::Int(a), Value::Double(b)) => if b > &(*a as f64) { max_val = arg; },
            (Value::Double(a), Value::Int(b)) => if (*b as f64) > *a { max_val = arg; },
            _ => return Err("max() arguments must be numbers".to_string()),
        }
    }
    
    Ok(max_val.clone())
}
```

### String Functions

```rust
pub fn builtin_substr(args: &[Value]) -> Result<Value, String> {
    if args.len() < 2 || args.len() > 3 {
        return Err(format!("substr() takes 2-3 arguments, got {}", args.len()));
    }
    
    let string = match &args[0] {
        Value::String(s) => s,
        _ => return Err(format!("substr() first argument must be a string, got {}", args[0].type_name())),
    };
    
    let start = match &args[1] {
        Value::Int(n) => *n as usize,
        _ => return Err(format!("substr() second argument must be an integer, got {}", args[1].type_name())),
    };
    
    let chars: Vec<char> = string.chars().collect();
    
    if start >= chars.len() {
        return Ok(Value::String(String::new()));
    }
    
    let end = if args.len() == 3 {
        match &args[2] {
            Value::Int(n) => std::cmp::min(start + (*n as usize), chars.len()),
            _ => return Err(format!("substr() third argument must be an integer, got {}", args[2].type_name())),
        }
    } else {
        chars.len()
    };
    
    let substring: String = chars[start..end].iter().collect();
    Ok(Value::String(substring))
}
```

### I/O Functions

```rust
use std::io::{self, Write};

pub fn builtin_input(args: &[Value]) -> Result<Value, String> {
    // Optional prompt argument
    if args.len() > 1 {
        return Err(format!("input() takes 0-1 arguments, got {}", args.len()));
    }
    
    if args.len() == 1 {
        match &args[0] {
            Value::String(prompt) => {
                print!("{}", prompt);
                io::stdout().flush().unwrap();
            },
            _ => return Err(format!("input() prompt must be a string, got {}", args[0].type_name())),
        }
    }
    
    let mut input = String::new();
    match io::stdin().read_line(&mut input) {
        Ok(_) => {
            // Remove trailing newline
            if input.ends_with('\n') {
                input.pop();
                if input.ends_with('\r') {
                    input.pop();
                }
            }
            Ok(Value::String(input))
        },
        Err(e) => Err(format!("Failed to read input: {}", e)),
    }
}
```

## Best Practices

### 1. Argument Validation

Always validate argument count and types:

```rust
pub fn builtin_example(args: &[Value]) -> Result<Value, String> {
    // Check argument count
    if args.len() != 2 {
        return Err(format!("example() takes exactly 2 arguments, got {}", args.len()));
    }
    
    // Check argument types
    let first = match &args[0] {
        Value::String(s) => s,
        _ => return Err(format!("example() first argument must be a string, got {}", args[0].type_name())),
    };
    
    let second = match &args[1] {
        Value::Int(n) => *n,
        _ => return Err(format!("example() second argument must be an integer, got {}", args[1].type_name())),
    };
    
    // Function logic here...
    Ok(Value::Nil)
}
```

### 2. Error Messages

Provide clear, helpful error messages:

```rust
// Good
"len() takes exactly 1 argument, got 3"
"substr() first argument must be a string, got int"

// Bad  
"Invalid arguments"
"Error"
```

### 3. Type Coercion

Be consistent about when to coerce types:

```rust
pub fn builtin_numeric_function(args: &[Value]) -> Result<Value, String> {
    let num = match &args[0] {
        Value::Int(n) => *n as f64,
        Value::Double(n) => *n,
        // Decide: allow string parsing or not?
        Value::String(s) => match s.parse::<f64>() {
            Ok(n) => n,
            Err(_) => return Err(format!("Cannot convert '{}' to number", s)),
        },
        _ => return Err(format!("Argument must be a number, got {}", args[0].type_name())),
    };
    
    // Use num...
    Ok(Value::Double(result))
}
```

### 4. Resource Management

For functions that use system resources:

```rust
use std::fs;

pub fn builtin_read_file(args: &[Value]) -> Result<Value, String> {
    if args.len() != 1 {
        return Err(format!("read_file() takes exactly 1 argument, got {}", args.len()));
    }
    
    let filename = match &args[0] {
        Value::String(s) => s,
        _ => return Err(format!("read_file() argument must be a string, got {}", args[0].type_name())),
    };
    
    match fs::read_to_string(filename) {
        Ok(content) => Ok(Value::String(content)),
        Err(e) => Err(format!("Failed to read file '{}': {}", filename, e)),
    }
}
```

## Testing Built-ins

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_builtin_len() {
        // Test with string
        let args = vec![Value::String("hello".to_string())];
        let result = builtin_len(&args).unwrap();
        assert_eq!(result, Value::Int(5));
        
        // Test with wrong type
        let args = vec![Value::Int(42)];
        let result = builtin_len(&args);
        assert!(result.is_err());
        
        // Test with wrong argument count
        let args = vec![];
        let result = builtin_len(&args);
        assert!(result.is_err());
    }
}
```

### Integration Tests

Create test files that use the built-ins:

```javascript
// tests/builtins.bcc
print type(42)          // "int"
print type(3.14)        // "double"  
print type("hello")     // "string"
print type(true)        // "boolean"
print type(nil)         // "nil"

print len("hello")      // 5
print str(42)           // "42"
print int("123")        // 123
print double(7)         // 7.0

print abs(-5)           // 5
print max(1, 5, 3)      // 5
```

## Common Patterns

### 1. Variadic Functions

Functions that accept variable numbers of arguments:

```rust
pub fn builtin_print_all(args: &[Value]) -> Result<Value, String> {
    for (i, arg) in args.iter().enumerate() {
        if i > 0 {
            print!(" ");
        }
        print!("{}", arg);
    }
    println!();
    Ok(Value::Nil)
}
```

### 2. Optional Arguments

Functions with default values:

```rust
pub fn builtin_round(args: &[Value]) -> Result<Value, String> {
    if args.is_empty() || args.len() > 2 {
        return Err(format!("round() takes 1-2 arguments, got {}", args.len()));
    }
    
    let number = match &args[0] {
        Value::Double(n) => *n,
        Value::Int(n) => *n as f64,
        _ => return Err(format!("round() first argument must be a number, got {}", args[0].type_name())),
    };
    
    let digits = if args.len() == 2 {
        match &args[1] {
            Value::Int(n) => *n,
            _ => return Err(format!("round() second argument must be an integer, got {}", args[1].type_name())),
        }
    } else {
        0  // Default value
    };
    
    let multiplier = 10_f64.powi(digits as i32);
    let result = (number * multiplier).round() / multiplier;
    Ok(Value::Double(result))
}
```

### 3. Higher-Order Functions (Future)

When user-defined functions are added, built-ins can accept them:

```rust
// This would work once user functions are implemented
pub fn builtin_map(args: &[Value]) -> Result<Value, String> {
    if args.len() != 2 {
        return Err(format!("map() takes exactly 2 arguments, got {}", args.len()));
    }
    
    let function = &args[0];
    let array = &args[1];
    
    // Apply function to each element of array
    // (This requires array type and function calling)
    todo!("Implement when arrays and user functions exist")
}
```

## Module Organization

For a larger standard library, organize built-ins by category:

```rust
// builtins/mod.rs
pub mod math;
pub mod string;
pub mod io;
pub mod conversion;

pub use math::*;
pub use string::*;
pub use io::*;
pub use conversion::*;

// builtins/math.rs
pub fn builtin_abs(args: &[Value]) -> Result<Value, String> { ... }
pub fn builtin_max(args: &[Value]) -> Result<Value, String> { ... }
pub fn builtin_min(args: &[Value]) -> Result<Value, String> { ... }

// builtins/string.rs  
pub fn builtin_len(args: &[Value]) -> Result<Value, String> { ... }
pub fn builtin_substr(args: &[Value]) -> Result<Value, String> { ... }
```

Built-in functions are the bridge between your language and the outside world. They should be carefully designed, well-tested, and documented to provide a solid foundation for programs written in your language.