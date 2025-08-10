# Architecture Overview

This document provides a high-level understanding of the BCC interpreter's architecture, design philosophy, and how all the components work together to create a complete programming language implementation.

## Table of Contents
1. [System Overview](#system-overview)
2. [Design Philosophy](#design-philosophy)
3. [Component Architecture](#component-architecture)
4. [Data Flow](#data-flow)
5. [Module Organization](#module-organization)
6. [Error Handling Strategy](#error-handling-strategy)
7. [Language Design Decisions](#language-design-decisions)
8. [Extension Points](#extension-points)
9. [Performance Characteristics](#performance-characteristics)
10. [Future Architecture Evolution](#future-architecture-evolution)

## System Overview

BCC (Basic Compiler Collection) is a **simple tree-walking interpreter** for a **Lox-like dynamically-typed scripting language**:

```
Source Code → Simplified Lexer → Parser → Evaluator → Output
    ↓              ↓                ↓         ↓
  String       String Tokens    String AST  Values/Effects
```

**Key characteristics:**
- **Code simplicity**: Owned strings prioritize maintainability over memory optimization
- **Modular design**: Clean separation between phases
- **Rich error diagnostics**: Beautiful error reporting with Ariadne
- **Python-like syntax**: Familiar and approachable
- **Educational focus**: Clear, well-documented code for learning

## Design Philosophy

### Educational First

The codebase is designed as a **teaching tool** for understanding interpreter implementation:

- **Clear code organization**: Each phase in its own module
- **Comprehensive documentation**: Every major component explained
- **Simple algorithms**: Straightforward implementations over clever optimizations
- **Readable naming**: Function and variable names clearly indicate purpose
- **Small files**: No massive files that are hard to understand

### Simplicity and Maintainability Focus

- **Clear ownership model**: Fully converted from zero-copy design to owned `String`s to eliminate all lifetime complexity
- **Tree-walking interpreter**: Direct AST execution instead of bytecode
- **Recursive algorithms**: Natural recursion for parsing and evaluation
- **Simple data structures**: Prioritize code clarity over micro-optimizations
- **Educational clarity**: Code prioritizes understanding over performance
- **Minimal state**: Each component maintains only necessary state

### Rich Error Experience

- **Ariadne integration**: Beautiful colored error reports with source context
- **Span tracking**: Every AST node knows its source location
- **Descriptive messages**: Clear, helpful error descriptions
- **Context-aware errors**: Different error messages for different situations

## Component Architecture

### Core Pipeline Components

```
┌─────────────┐    ┌─────────────┐    ┌─────────────┐    ┌─────────────┐
│   Source    │ -> │    Lexer    │ -> │   Parser    │ -> │  Evaluator  │
│   String    │    │   Tokens    │    │     AST     │    │   Values    │
└─────────────┘    └─────────────┘    └─────────────┘    └─────────────┘
```

#### 1. Lexer (Tokenizer)
- **Input**: Raw source code string
- **Output**: Sequence of tokens with position information
- **Responsibility**: Convert characters into meaningful symbols
- **Algorithm**: Character-by-character scanning with keyword recognition

#### 2. Parser
- **Input**: Token stream from lexer
- **Output**: Abstract Syntax Tree (AST)
- **Responsibility**: Structure tokens into meaningful program representation
- **Algorithm**: Recursive descent parser with precedence climbing

#### 3. Evaluator
- **Input**: AST from parser
- **Output**: Program execution with side effects
- **Responsibility**: Execute the program logic
- **Algorithm**: Tree-walking interpretation with environment chain

### Supporting Components

#### Error System
- **Centralized error types**: `BccError` with different error kinds
- **Rich diagnostics**: Span information for precise error location
- **Ariadne integration**: Beautiful terminal error display

#### Value System
- **Dynamic typing**: Runtime type checking
- **Rich type set**: Integers, doubles, strings, booleans, nil
- **Truthiness rules**: Python-like semantics for conditions

#### Environment Chain
- **Lexical scoping**: Variables resolved through scope chain
- **Block scoping**: Each `{}` block creates new scope
- **Dynamic variable creation**: Python-like assignment behavior

## Data Flow

### From Source to Execution

```
"x = 42; print x"
        ↓
┌─────────────────────┐
│       LEXER         │
│  String → Tokens    │
└─────────────────────┘
        ↓
[Identifier("x"), Equal, Integer("42"), Semicolon, 
 Print, Identifier("x"), EOF]
        ↓
┌─────────────────────┐
│       PARSER        │
│   Tokens → AST      │
└─────────────────────┘
        ↓
Program([
  Expression(Assign("x", Literal(Int(42)))),
  Print(Variable("x"))
])
        ↓
┌─────────────────────┐
│      EVALUATOR      │
│     AST → Effects   │
└─────────────────────┘
        ↓
Environment: { x: 42 }
Output: 42
```

### Error Propagation

Each phase can produce errors:

```
Source Code → Lexer Error (unexpected character)
                ↓
            Tokens → Parser Error (syntax error)
                      ↓  
                  AST → Runtime Error (undefined variable)
```

All errors are:
1. **Enriched with context** (source location, descriptive message)
2. **Reported immediately** with beautiful Ariadne formatting
3. **Stop execution** (fail-fast behavior)

## Module Organization

### File Structure

```
src/
├── main.rs         # Entry point, command-line interface
├── lexer.rs        # Memory-efficient tokenization using string slices
├── parser.rs       # Recursive descent parser with lifetime-parameterized AST
├── ast.rs          # Abstract syntax tree definitions with `&str` references
├── evaluator.rs    # Tree-walking interpreter with environments
├── runner.rs       # File execution orchestration
├── repl.rs         # Interactive shell with persistent state
├── value.rs        # Runtime value type system
└── error.rs        # Error types and ariadne-powered diagnostics
```

### Dependency Graph

```
main.rs
├── repl.rs → runner.rs
├── runner.rs
│   ├── lexer.rs → error.rs
│   ├── parser.rs → lexer.rs, ast.rs, error.rs, value.rs
│   └── evaluator.rs → ast.rs, value.rs, error.rs
├── ast.rs → value.rs, error.rs
├── value.rs
└── error.rs → ariadne
```

**Key principles**:
- **Minimal dependencies**: Each module imports only what it needs
- **Clear layering**: Higher-level modules use lower-level ones
- **No circular dependencies**: Clean dependency graph

### Interface Design

Each module exposes a clean public interface:

```rust
// Lexer
impl Lexer {
    pub fn new(source: &str) -> Self
    pub fn scan_tokens(&mut self) -> Result<Vec<Token>, BccError>
}

// Parser  
impl Parser {
    pub fn new(tokens: Vec<Token>) -> Self
    pub fn parse(&mut self) -> Result<Program, BccError>
}

// Evaluator
impl Evaluator {
    pub fn new() -> Self
    pub fn evaluate_program(&mut self, program: &Program) -> Result<(), BccError>
}
```

## Error Handling Strategy

### Three-Layer Error System

#### 1. Error Types
```rust
pub enum ErrorKind {
    LexError,      // Tokenization problems
    ParseError,    // Syntax problems  
    RuntimeError,  // Execution problems
}
```

#### 2. Rich Context
```rust
pub struct BccError {
    pub kind: ErrorKind,
    pub span: Span,        // Source location
    pub message: String,   // Human-readable description
}
```

#### 3. Beautiful Reporting
- **Ariadne integration**: Colored, formatted error display
- **Source context**: Shows problematic code with highlighting
- **Helpful messages**: Guides user toward solutions

### Error Philosophy

- **Fail-fast**: Stop on first error to avoid confusion
- **Clear messages**: Explain what went wrong and where
- **Precise location**: Highlight exact problem location
- **Context-appropriate**: Different messages for different phases

## Language Design Decisions

### Syntax Choices

#### Python-Inspired Elements
- **No `var` keyword**: Direct assignment creates variables
- **Optional semicolons**: More natural for scripting
- **Dynamic typing**: No type declarations needed
- **Truthiness rules**: Empty/zero values are falsy

#### C-Style Elements  
- **Block syntax**: Braces for grouping statements
- **Control flow**: Traditional if/while/for structures
- **Operators**: Familiar arithmetic and comparison operators
- **Comments**: C++ style `//` line comments

### Type System

#### Value Types
```rust
pub enum Value {
    Nil,           // Absence of value
    Bool(bool),    // True/false
    Int(i64),      # 64-bit integers
    Double(f64),   # IEEE 754 doubles  
    String(String), # UTF-8 strings
}
```

#### Type Design Decisions

- **Separate int/double**: Preserves programmer intent
- **No implicit string conversion**: Type safety for operations
- **Rich equality**: Cross-type numeric comparisons allowed
- **Dynamic but safe**: Runtime type checking with clear errors

### Scoping Model

#### Lexical Scoping
- **Block-based**: Each `{}` creates new scope
- **Environment chain**: Parent scopes accessible
- **Variable shadowing**: Inner scopes can hide outer variables

#### Variable Semantics
- **Dynamic creation**: Assignment creates variables
- **No hoisting**: Variables available after assignment
- **Scope-based lifetime**: Variables die when scope exits

## Extension Points

### Language Features

The architecture naturally supports adding:

#### Functions
```rust
// Parser extensions
Expr::Call { callee, args }  // Already exists
Stmt::Function { name, params, body }  // Could add

// Evaluator extensions  
Value::Function(Function)  // Could add closure type
```

#### Classes (Future)
```rust
Stmt::Class { name, methods }
Expr::Get { object, name }
Expr::Set { object, name, value }
```

#### Modules (Future)
```rust
Stmt::Import { module, items }
Stmt::Export { items }
```

### Backend Variations

The tree-walking architecture makes it easy to:

#### Add Bytecode VM
1. **Keep existing frontend** (lexer, parser)
2. **Add compiler phase**: AST → Bytecode
3. **Add VM**: Bytecode → Execution

#### Add Static Analysis
1. **Symbol table builder**: Track variable declarations
2. **Type checker**: Optional static typing
3. **Optimization passes**: Dead code elimination, etc.

### Tool Integration

The rich AST and error system support:

#### Language Server
- **Hover information**: Use spans for location mapping
- **Error reporting**: Use existing error system
- **Completion**: Analyze AST for available symbols

#### Debugger
- **Breakpoints**: Use spans for location mapping
- **Variable inspection**: Access evaluator environment
- **Stack traces**: Use call chain information

## Performance Characteristics

### Time Complexity

| Phase | Complexity | Notes |
|-------|-----------|-------|
| Lexing | O(n) | Linear scan of source |
| Parsing | O(n) | Each token processed once |
| Evaluation | O(n×d) | n statements, d scope depth |

### Space Complexity

| Component | Complexity | Notes |
|-----------|-----------|-------|
| Tokens | O(n) | One token per language element |
| AST | O(n) | One node per language construct |
| Environment | O(v×d) | v variables, d scope depth |

### Performance Trade-offs

**Prioritized**:
- Code clarity and maintainability
- Rich error reporting
- Educational value

**Sacrificed**:
- Execution speed (tree-walking overhead)
- Memory efficiency (lots of allocations)
- Startup time (no precompilation)

## Future Architecture Evolution

### Short-term Extensions

#### Language Features
- **Functions and closures**: Natural fit for current architecture
- **Built-in functions**: Extension to evaluator
- **Better REPL**: Multi-line input, history
- **Standard library**: File I/O, string manipulation

#### Development Experience
- **Improved error recovery**: Continue parsing after errors
- **Better error messages**: More context-specific hints
- **Performance profiling**: Built-in timing and memory tracking

### Long-term Evolution

#### Performance Path
1. **Bytecode compiler**: AST → bytecode transformation
2. **Register-based VM**: Efficient instruction execution
3. **JIT compilation**: Hot code compilation to native

#### Language Path  
1. **Static typing**: Optional type annotations
2. **Module system**: Import/export mechanisms
3. **Object system**: Classes, inheritance, polymorphism
4. **Concurrency**: Async/await, actors, or threads

#### Tool Ecosystem
1. **Package manager**: Dependency management
2. **Language server**: IDE integration
3. **Debugger**: Interactive debugging
4. **Formatter**: Code style enforcement

### Architectural Flexibility

The current design supports both paths:

**Performance evolution**: Can add backend phases without changing frontend
**Language evolution**: Can extend AST and evaluator incrementally
**Tool evolution**: Rich spans and error system support advanced tooling

The modular architecture ensures that complexity can be added gradually while maintaining the educational clarity of the core system.

## Conclusion

The BCC interpreter architecture strikes a balance between simplicity and extensibility. The clean separation between phases, rich error handling, and educational focus create a system that's both understandable and practical.

The tree-walking approach provides an excellent foundation for learning interpreter implementation, while the modular design ensures the system can evolve toward more sophisticated implementations as needed.

This architecture demonstrates that educational software doesn't have to be toy software - with proper design, a teaching-focused implementation can serve as the foundation for a serious language implementation.