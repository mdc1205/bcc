I'm looking to build an interpreter for a language very similar to lox. The things I care less about are class creation.

## ✅ COMPLETED FEATURES:
 - ✅ Python-like variable assignment (no var keyword needed: `x = 10`)
 - ✅ Separate int and double types with proper arithmetic type promotion
 - ✅ No classes or super (simplified from Lox)
 - ✅ Excellent error diagnostics using ariadne with colored output
 - ✅ REPL with persistent memory between commands
 - ✅ File execution mode
 - ✅ Python-like REPL behavior (expressions return values directly)
 - ✅ Optional semicolons (Python-like syntax)
 - ✅ Control flow: if/else, while loops, for loops
 - ✅ Boolean operations: and, or, not (both `!` and `not` keyword supported)
 - ✅ **Membership Testing**: `in` keyword for checking containment in lists, dictionaries, and strings
 - ✅ String concatenation and manipulation
 - ✅ Block scoping with proper environment handling
 - ✅ **Lists and Dictionaries**: Full support for collection literals `[1, 2, 3]` and `{"key": "value"}` with smart grammar disambiguation
 - ✅ **Built-in Functions**: `print()`, `len()`, `type()`, and `case()` functions with comprehensive type support
 - ✅ **Unicode Support**: Full Unicode identifier support (Greek, Chinese, Arabic, etc.)
 - ✅ Clean modular architecture with small focused files
 - ✅ **Comprehensive Documentation**: Complete markdown documentation covering lexer, parser, evaluator, architecture, built-ins, and compiler fundamentals  
 - ✅ **Simplified Architecture**: Fully converted from zero-copy design to owned strings (`String`) for better code maintainability over memory optimization
 - ✅ **Robust Parser**: Never crashes on any malformed input, always provides helpful error messages
 - ✅ **Comprehensive Test Suite**: 300+ test cases covering all edge cases and malformed input scenarios
 - ✅ **Case Function & Property Access**: `case()` built-in for conditional selection with `.result` property access

## 🔨 CURRENT STATUS:
The interpreter is fully functional with core language features working correctly. You can:
- Assign variables without var: `x = 10`
- Use expressions directly in REPL: `x + 5` (returns 15)
- Use control flow: `if (x > 5) { print("yes") }`
- Use loops: `while (i < 3) { i = i + 1 }`
- Mix int/double arithmetic seamlessly with proper type promotion
- Create and manipulate lists: `items = [1, 2.5, "hello"]`
- Create and manipulate dictionaries: `person = {"name": "John", "age": 30}`
- Use both `!` and `not` for logical negation: `not false` or `!false`
- Call built-in functions: `print("Hello")`, `len([1,2,3])`, `type(42)`, `case(true, "yes", false, "no")`
- Use Unicode identifiers: `α = 3.14`, `中文 = "Chinese"`, `العربية = true`
- Test membership: `2 in [1, 2, 3]`, `"key" in {"key": "value"}`, `"sub" in "string"`
- Use case selection: `result = case(x > 10, "big", x > 5, "medium", true, "small")` then `result.result`

## 📋 REMAINING TASKS:
 - User-defined functions and function calls (built-ins work, but user-defined functions show "not yet implemented")
 - I want the ability to later add on a bytecode VM that would be easy to tie into the existing code (maybe separate the tree walk)
 - Simple design with a focus on smaller files for better understanding (I don't want 2k line files I would rather split things up)

## 🚀 NEXT IMMEDIATE STEPS:
1. **Functions Implementation**: Add user-defined function definitions and calls to complete the language
2. **Bytecode VM Prep**: Ensure architecture supports easy VM integration later
3. **Performance Optimization**: Add back selective performance optimizations where beneficial

## 🏗️ CURRENT ARCHITECTURE:
```
src/
├── main.rs         - CLI entry point (REPL vs file mode)
├── lib.rs          - Library interface for external use
├── lexer.rs        - Simplified tokenization using owned strings
├── parser.rs       - Recursive descent parser with clean AST generation
├── ast.rs          - Abstract syntax tree with owned string identifiers
├── evaluator.rs    - Tree-walking interpreter with environments
├── value.rs        - Runtime value representation
├── error.rs        - Error types with ariadne diagnostics
├── runner.rs       - Orchestrates lexing → parsing → evaluation
└── repl.rs         - Interactive REPL with persistent state

tests/
├── main.rs                  - Test runner executable
├── lib.rs                   - Test library interface
├── test_runner.rs           - Core test framework
├── malformed_expressions.rs - Parentheses, brackets, braces tests
├── edge_cases.rs           - EOF, empty inputs, deeply nested tests  
├── operator_tests.rs       - Missing operands, invalid combinations
├── control_flow_tests.rs   - Malformed if/while/for statements
├── literal_tests.rs        - Invalid numbers, strings, booleans
├── function_call_tests.rs  - Malformed calls, missing arguments
├── assignment_tests.rs     - Invalid targets, incomplete expressions
├── mixed_construct_tests.rs- Complex combination errors
└── positive_tests.rs       - Verify correct parsing still works
```

**Key Design Principles:**
- **Code Maintainability**: Fully converted to owned strings (`String`) - eliminated all zero-copy complexity and lifetime parameters for maximum clarity
- **Clean Separation**: Each phase has its own focused module
- **Excellent Error Reporting**: Ariadne provides beautiful diagnostic messages
- **Extensible Design**: Architecture supports easy addition of new features
- **Robust Error Handling**: Parser never crashes, always recovers gracefully with helpful diagnostics
- **Comprehensive Testing**: 300+ test cases ensure parser robustness across all edge cases

## 🧪 PARSER ROBUSTNESS TESTING:

The interpreter includes a comprehensive test suite that stress-tests the parser with every conceivable type of malformed input:

### Test Categories:
1. **Malformed Expressions** (50+ tests)
   - Unmatched parentheses, brackets, braces
   - Empty parentheses and other invalid constructs
   - Mixed-up delimiter types

2. **Edge Cases** (40+ tests)  
   - EOF conditions after operators, in expressions
   - Empty inputs, whitespace-only inputs
   - Deeply nested structures (50+ levels)
   - Boundary conditions and single-character inputs

3. **Operator Errors** (45+ tests)
   - Missing left/right operands for all binary operators
   - Invalid operator combinations (++, +=, etc.)
   - Unary operators with missing operands

4. **Control Flow Errors** (35+ tests)
   - Missing parentheses in if/while/for conditions
   - Malformed loop clauses and nested control flow
   - Invalid block structures

5. **Literal Errors** (30+ tests)
   - Invalid number formats, overflow conditions  
   - Unclosed strings, invalid string delimiters
   - Wrong case boolean values, invalid nil variations

6. **Function Call Errors** (25+ tests)
   - Unmatched parentheses in function calls
   - Missing arguments, trailing commas
   - Nested call errors with complex expressions

7. **Assignment Errors** (35+ tests)
   - Invalid assignment targets (literals, expressions)
   - Incomplete right-hand sides  
   - Chained assignment problems

8. **Mixed Construct Errors** (30+ tests)
   - Complex combinations of multiple error types
   - Pathological cases with everything wrong
   - Deep nesting with various delimiter mismatches

9. **Positive Tests** (50+ tests)
   - Verify all correct syntax still works
   - Complex valid programs and edge cases
   - Unicode support, stress tests

### Running the Tests:
```bash
# Run the comprehensive parser robustness test suite
cargo run --bin test-parser-robustness

# Expected output: "✅ SUCCESS: All tests passed! Parser is robust."
```

### Error Quality Guarantees:
- **No Crashes**: Parser never panics on any input
- **Helpful Messages**: All errors include clear descriptions and suggestions
- **Precise Locations**: Error spans point to exact problematic code locations  
- **Recovery**: Parser continues after errors when possible
- **Context-Aware**: Error messages understand the parsing context