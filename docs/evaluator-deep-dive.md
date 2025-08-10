# The Evaluator: A Deep Dive

The evaluator is the third and final core phase of the BCC interpreter, responsible for executing the Abstract Syntax Tree (AST) produced by the parser. This document provides a comprehensive exploration of how our tree-walking interpreter evaluates programs.

## Table of Contents
1. [Overview](#overview)
2. [Tree-Walking Interpretation](#tree-walking-interpretation)
3. [Environment and Scoping](#environment-and-scoping)
4. [Value System](#value-system)
5. [Function-by-Function Analysis](#function-by-function-analysis)
6. [Statement Execution](#statement-execution)
7. [Expression Evaluation](#expression-evaluation)
8. [Error Handling](#error-handling)
9. [Design Decisions](#design-decisions)

## Overview

The BCC evaluator implements a **tree-walking interpreter** - it directly executes the AST by recursively visiting each node and performing the appropriate operation. This approach is simple, intuitive, and perfect for understanding how interpreters work.

**Key characteristics:**
- **Direct execution**: No bytecode compilation step
- **Recursive evaluation**: Mirrors the recursive AST structure
- **Environment-based scoping**: Lexical scoping with environment chains
- **Rich value system**: Separate integer and double types
- **Python-like semantics**: Dynamic variable creation, truthiness rules

## Tree-Walking Interpretation

Tree-walking interpretation works by:

1. **Visiting each AST node** recursively
2. **Performing the operation** represented by that node
3. **Combining results** from child nodes
4. **Returning values** up the call chain

```rust
// Example: evaluating (2 + 3) * 4
Binary {
    left: Binary {           // Evaluate left: 2 + 3 = 5
        left: Literal(2),    // Returns Value::Int(2)
        op: Add,
        right: Literal(3),   // Returns Value::Int(3)
    },
    op: Multiply,
    right: Literal(4),       // Returns Value::Int(4)
}
// Final result: 5 * 4 = 20
```

This approach directly mirrors the mathematical evaluation we learned in school.

## Environment and Scoping

### Environment Structure

```rust
#[derive(Debug, Clone)]
pub struct Environment {
    values: HashMap<String, Value>,    // Variable bindings
    enclosing: Option<Box<Environment>>, // Parent scope
}
```

The environment implements **lexical scoping** through a chain of hash maps:

- **values**: Current scope's variable bindings
- **enclosing**: Reference to parent scope (if any)

### Scope Chain Example

```javascript
// Global scope: { x: 10 }
x = 10
{
    // Block scope: { y: 20, enclosing: global }
    y = 20
    {
        // Inner block: { z: 30, enclosing: block }
        z = 30
        print x + y + z  // Can access all three variables
    }
    // z is no longer accessible here
}
```

### Environment Methods

#### Variable Lookup: `get(name: &str) -> Option<Value>`

```rust
pub fn get(&self, name: &str) -> Option<Value> {
    if let Some(value) = self.values.get(name) {
        Some(value.clone())  // Found in current scope
    } else if let Some(ref enclosing) = self.enclosing {
        enclosing.get(name)  // Search parent scope
    } else {
        None                 // Not found anywhere
    }
}
```

**Algorithm**: Search current scope first, then recursively search parent scopes.

#### Variable Assignment: `assign(name: &str, value: Value) -> Result<(), BccError>`

```rust
pub fn assign(&mut self, name: &str, value: Value) -> Result<(), BccError> {
    if self.values.contains_key(name) {
        self.values.insert(name.to_string(), value);
        Ok(())
    } else if let Some(ref mut enclosing) = self.enclosing {
        enclosing.assign(name, value)
    } else {
        // For Python-like behavior, create the variable if it doesn't exist
        self.values.insert(name.to_string(), value);
        Ok(())
    }
}
```

**Python-like semantics**: Variables are created automatically when assigned, even if they don't exist.

## Value System

### Value Types

```rust
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Nil,
    Bool(bool),
    Int(i64),          // Separate from doubles
    Double(f64),       // IEEE 754 floating point
    String(String),
}
```

**Design choices**:
- **Separate Int/Double**: Preserves programmer intent and enables optimizations
- **Nil**: Represents absence of value (like null)
- **String**: Owned strings for simplicity

### Truthiness Rules

```rust
pub fn is_truthy(&self) -> bool {
    match self {
        Value::Nil => false,
        Value::Bool(b) => *b,
        Value::Int(n) => *n != 0,          // 0 is falsy
        Value::Double(n) => *n != 0.0,     // 0.0 is falsy
        Value::String(s) => !s.is_empty(), // Empty string is falsy
    }
}
```

**Python-inspired**: Most values are truthy except for "empty" or "zero" values.

## Function-by-Function Analysis

### Evaluator Structure

```rust
pub struct Evaluator {
    environment: Environment,  // Current environment
}

impl Evaluator {
    pub fn new() -> Self {
        Self {
            environment: Environment::new(),
        }
    }
}
```

**State**: Single environment that changes as we enter/exit scopes.

### Program Execution: `evaluate_program(&mut self, program: &Program) -> Result<(), BccError>`

```rust
pub fn evaluate_program(&mut self, program: &Program) -> Result<(), BccError> {
    for statement in &program.statements {
        self.execute_statement(statement)?;
    }
    Ok(())
}
```

**Purpose**: Execute all top-level statements in order.

**Error handling**: Stops on first error (fail-fast behavior).

## Statement Execution

### Statement Dispatcher: `execute_statement(&mut self, stmt: &Stmt) -> Result<(), BccError>`

```rust
fn execute_statement(&mut self, stmt: &Stmt) -> Result<(), BccError> {
    match stmt {
        Stmt::Expression { expr, .. } => {
            self.evaluate_expression(expr)?;
            Ok(())
        }
        Stmt::Print { expr, .. } => {
            let value = self.evaluate_expression(expr)?;
            println!("{}", value);
            Ok(())
        }
        Stmt::Block { statements, .. } => {
            self.execute_block(statements)
        }
        // ... other statement types
    }
}
```

**Purpose**: Dispatch to appropriate execution method based on statement type.

**Pattern matching**: Rust's pattern matching makes this elegant and exhaustive.

### Expression Statements

```rust
Stmt::Expression { expr, .. } => {
    self.evaluate_expression(expr)?;
    Ok(())
}
```

**Behavior**: Evaluate the expression and discard the result.

**Use cases**: Assignments, function calls (when implemented).

### Print Statements

```rust
Stmt::Print { expr, .. } => {
    let value = self.evaluate_expression(expr)?;
    println!("{}", value);
    Ok(())
}
```

**Simple output**: Evaluate expression and print its value.

### Block Statements: `execute_block(&mut self, statements: &[Stmt]) -> Result<(), BccError>`

```rust
fn execute_block(&mut self, statements: &[Stmt]) -> Result<(), BccError> {
    let previous_env = self.environment.clone();
    self.environment = Environment::with_enclosing(previous_env);

    let result = (|| {
        for statement in statements {
            self.execute_statement(statement)?;
        }
        Ok(())
    })();

    // Extract the enclosing environment which may have been modified
    if let Some(enclosing) = self.environment.enclosing.take() {
        self.environment = *enclosing;
    }
    
    result
}
```

**Purpose**: Execute statements in a new scope.

**Algorithm**:
1. **Save current environment**
2. **Create new environment** with current as parent
3. **Execute all statements** in new scope
4. **Restore previous environment**
5. **Return result** (preserving any errors)

**Scope isolation**: Variables created in the block don't affect the outer scope.

### Control Flow: If Statements

```rust
Stmt::If {
    condition,
    then_branch,
    else_branch,
    ..
} => {
    let condition_value = self.evaluate_expression(condition)?;
    if condition_value.is_truthy() {
        self.execute_statement(then_branch)?;
    } else if let Some(else_stmt) = else_branch {
        self.execute_statement(else_stmt)?;
    }
    Ok(())
}
```

**Conditional execution**: Uses truthiness rules to decide which branch to take.

### Loops: While Statements

```rust
Stmt::While { condition, body, .. } => {
    while self.evaluate_expression(condition)?.is_truthy() {
        self.execute_statement(body)?;
    }
    Ok(())
}
```

**Simple loop**: Re-evaluate condition before each iteration.

**Potential issue**: Infinite loops are possible (no built-in break mechanism yet).

### For Loops

```rust
Stmt::For {
    initializer,
    condition,
    increment,
    body,
    ..
} => {
    // Execute initializer if present
    if let Some(init) = initializer {
        self.execute_statement(init)?;
    }

    // Execute loop
    loop {
        // Check condition
        if let Some(cond) = condition {
            if !self.evaluate_expression(cond)?.is_truthy() {
                break;
            }
        }

        // Execute body
        self.execute_statement(body)?;

        // Execute increment
        if let Some(inc) = increment {
            self.evaluate_expression(inc)?;
        }
    }
    Ok(())
}
```

**C-style for loop**: Three optional parts - init, condition, increment.

**Flexibility**: Any or all parts can be omitted.

## Expression Evaluation

### Expression Dispatcher: `evaluate_expression(&mut self, expr: &Expr) -> Result<Value, BccError>`

```rust
pub fn evaluate_expression(&mut self, expr: &Expr) -> Result<Value, BccError> {
    match expr {
        Expr::Literal { value, .. } => Ok(value.clone()),
        Expr::Variable { name, span } => {
            self.environment.get(name).ok_or_else(|| {
                BccError::runtime_error(
                    span.clone(),
                    format!("Undefined variable '{}'", name),
                )
            })
        }
        Expr::Assign { name, value, span } => {
            let val = self.evaluate_expression(value)?;
            self.environment.assign(name, val.clone()).map_err(|_| {
                BccError::runtime_error(
                    span.clone(),
                    format!("Undefined variable '{}'", name),
                )
            })?;
            Ok(val)
        }
        // ... other expression types
    }
}
```

**Purpose**: Recursively evaluate expressions and return values.

### Literal Expressions

```rust
Expr::Literal { value, .. } => Ok(value.clone()),
```

**Simple case**: Just return the stored value.

### Variable Access

```rust
Expr::Variable { name, span } => {
    self.environment.get(name).ok_or_else(|| {
        BccError::runtime_error(
            span.clone(),
            format!("Undefined variable '{}'", name),
        )
    })
}
```

**Algorithm**:
1. Look up variable in environment chain
2. Return value if found
3. Error with clear message if not found

### Assignment Expressions

```rust
Expr::Assign { name, value, span } => {
    let val = self.evaluate_expression(value)?;
    self.environment.assign(name, val.clone()).map_err(|_| {
        BccError::runtime_error(
            span.clone(),
            format!("Undefined variable '{}'", name),
        )
    })?;
    Ok(val)
}
```

**Assignment as expression**: Returns the assigned value (like C).

**Use case**: `print x = 5` prints 5 and assigns 5 to x.

### Binary Operations: `evaluate_binary_op()`

```rust
fn evaluate_binary_op(
    &self,
    operator: &BinaryOp,
    left: Value,
    right: Value,
    span: &Span,
) -> Result<Value, BccError> {
    match operator {
        BinaryOp::Add => match (left, right) {
            (Value::Int(l), Value::Int(r)) => Ok(Value::Int(l + r)),
            (Value::Double(l), Value::Double(r)) => Ok(Value::Double(l + r)),
            (Value::Int(l), Value::Double(r)) => Ok(Value::Double(l as f64 + r)),
            (Value::Double(l), Value::Int(r)) => Ok(Value::Double(l + r as f64)),
            (Value::String(l), Value::String(r)) => Ok(Value::String(l + &r)),
            (l, r) => Err(BccError::runtime_error(
                span.clone(),
                format!("Cannot add {} and {}", l.type_name(), r.type_name()),
            )),
        },
        // ... other operators
    }
}
```

**Comprehensive type handling**:
- **Same types**: Direct operation
- **Mixed numeric**: Automatic promotion to double
- **String concatenation**: Special case for `+`
- **Type errors**: Clear error messages

### Arithmetic Operations

**Addition**:
- `Int + Int → Int`
- `Double + Double → Double`
- `Int + Double → Double` (promotion)
- `String + String → String` (concatenation)

**Division**:
- Always returns `Double` (even `Int / Int`)
- Prevents integer division truncation
- Zero division checking

**Comparison**:
- Numeric types can be compared with automatic promotion
- Non-numeric comparisons are type errors

### Unary Operations: `evaluate_unary_op()`

```rust
fn evaluate_unary_op(
    &self,
    operator: &UnaryOp,
    operand: Value,
    span: &Span,
) -> Result<Value, BccError> {
    match operator {
        UnaryOp::Negate => match operand {
            Value::Int(n) => Ok(Value::Int(-n)),
            Value::Double(n) => Ok(Value::Double(-n)),
            _ => Err(BccError::runtime_error(
                span.clone(),
                format!("Cannot negate {}", operand.type_name()),
            )),
        },
        UnaryOp::Not => Ok(Value::Bool(!operand.is_truthy())),
    }
}
```

**Operations**:
- **Negation**: Only works on numbers
- **Logical NOT**: Works on any value (uses truthiness)

### Logical Operations

```rust
Expr::Logical {
    left,
    operator,
    right,
    ..
} => {
    let left_val = self.evaluate_expression(left)?;

    match operator {
        LogicalOp::Or => {
            if left_val.is_truthy() {
                Ok(left_val)      // Short-circuit: return left if truthy
            } else {
                self.evaluate_expression(right)
            }
        }
        LogicalOp::And => {
            if !left_val.is_truthy() {
                Ok(left_val)      // Short-circuit: return left if falsy
            } else {
                self.evaluate_expression(right)
            }
        }
    }
}
```

**Short-circuit evaluation**:
- **OR**: If left is truthy, return left (don't evaluate right)
- **AND**: If left is falsy, return left (don't evaluate right)

**Returns actual values**: Not just true/false (like Python).

### Equality Comparison: `is_equal()`

```rust
fn is_equal(&self, left: &Value, right: &Value) -> bool {
    match (left, right) {
        (Value::Nil, Value::Nil) => true,
        (Value::Bool(l), Value::Bool(r)) => l == r,
        (Value::Int(l), Value::Int(r)) => l == r,
        (Value::Double(l), Value::Double(r)) => l == r,
        (Value::Int(l), Value::Double(r)) => (*l as f64) == *r,
        (Value::Double(l), Value::Int(r)) => *l == (*r as f64),
        (Value::String(l), Value::String(r)) => l == r,
        _ => false,  // Different types are never equal
    }
}
```

**Mixed numeric equality**: Integers and doubles can be equal (`1 == 1.0`).

**Type-based equality**: Different types are never equal (except numeric).

## Error Handling

### Runtime Error Categories

1. **Undefined variables**: Variable not found in any scope
2. **Type errors**: Incompatible types for operations
3. **Division by zero**: Arithmetic error
4. **Invalid operations**: Like negating a string

### Error Reporting

```rust
BccError::runtime_error(
    span.clone(),
    format!("Cannot add {} and {}", l.type_name(), r.type_name()),
)
```

**Rich context**:
- **Span information**: Exact source location
- **Descriptive messages**: Include types and operation
- **Error propagation**: Uses `?` operator for clean code

### Error Recovery

The evaluator uses **fail-fast** error handling:
- Stops execution on first error
- No attempt to continue after errors
- Simple but effective for teaching purposes

## Design Decisions

### Why Tree-Walking?

**Advantages**:
- **Simple to understand**: Direct correspondence to AST
- **Easy to implement**: No intermediate representations
- **Good for prototyping**: Fast development cycle
- **Excellent debugging**: Can inspect AST directly

**Disadvantages**:
- **Slower execution**: Overhead of tree traversal
- **Memory usage**: AST must stay in memory
- **No optimization**: Limited performance improvements possible

### Environment Chain vs Symbol Tables

**Chosen approach**: Environment chain
- **Dynamic scoping support**: Easy to add later
- **Simple implementation**: Just linked hash maps
- **Memory efficient**: Only active scopes in memory

**Alternative**: Global symbol table with scope indices
- **Faster lookups**: O(1) instead of O(depth)
- **More complex**: Harder to implement and understand

### Python-Like Variable Semantics

**Automatic variable creation**: `x = 5` creates `x` if it doesn't exist
- **Convenient**: No need for explicit declarations
- **Flexible**: Good for scripting
- **Potential confusion**: Typos create new variables instead of errors

### Separate Int/Double Types

**Benefits**:
- **Preserves intent**: Programmer's choice of type
- **Performance**: Integer operations can be faster
- **Type information**: Better error messages

**Trade-offs**:
- **Complexity**: More type checking code
- **Mixed arithmetic**: Need promotion rules

### Truthiness Rules

**Python-inspired**:
- **Empty values are falsy**: `0`, `0.0`, `""`, `nil`
- **Everything else is truthy**: Including negative numbers

**Advantages**:
- **Intuitive**: Matches many other languages
- **Flexible**: Works with any value type
- **Consistent**: Same rules everywhere

## Performance Characteristics

### Time Complexity

**Statement execution**: O(n) where n is the number of statements
**Expression evaluation**: O(d) where d is expression depth
**Variable lookup**: O(s) where s is scope depth

### Space Complexity

**Environment chain**: O(s) where s is maximum scope depth
**Expression evaluation**: O(d) call stack space
**Value storage**: O(v) where v is number of variables

### Memory Allocation

**Frequent allocations**:
- Cloning values for returns
- Boxing expressions in AST
- String operations

**Optimization opportunities**:
- Value reference counting
- Arena allocation for environments
- Copy-on-write for strings

## Future Enhancements

### Functions and Closures
Environment chain design naturally supports:
- Function local variables
- Closure capture
- Recursive functions

### Better Performance
Could optimize with:
- Bytecode compilation
- Register-based VM
- JIT compilation

### Advanced Features
Tree-walking foundation supports:
- Classes and objects
- Modules and imports
- Exception handling

The evaluator's tree-walking design prioritizes simplicity and educational value over performance, making it an excellent foundation for understanding interpreter implementation.