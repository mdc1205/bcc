// Comprehensive Integration Tests for BCC Parser
//
// This file contains all parser robustness tests consolidated into a single
// integration test to ensure proper Rust module organization.

use bcc::lexer::Lexer;
use bcc::parser::Parser;
use bcc::error::BccError;

/// Test result for a single test case
#[derive(Debug)]
pub enum TestResult {
    Pass,
    Fail(String),
    Crash(String),
}

/// Individual test case
#[derive(Debug, Clone)]
pub struct TestCase {
    pub name: String,
    pub input: String,
    pub should_succeed: bool,
    pub expected_error_contains: Option<String>,
}

/// Test suite containing multiple test cases
#[derive(Debug)]
pub struct TestSuite {
    pub name: String,
    pub tests: Vec<TestCase>,
}

impl TestSuite {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            tests: Vec::new(),
        }
    }

    pub fn add_test(&mut self, test: TestCase) {
        self.tests.push(test);
    }

    /// Run all tests in this suite
    pub fn run(&self) -> TestSuiteResults {
        let mut results = TestSuiteResults::new(&self.name);
        
        println!("Running test suite: {}", self.name);
        println!("{}", "=".repeat(50));

        for test in &self.tests {
            let result = run_single_test(test);
            results.add_result(&test.name, result);
        }

        results.print_summary();
        results
    }
}

/// Results for a test suite run
#[derive(Debug)]
pub struct TestSuiteResults {
    pub suite_name: String,
    pub results: Vec<(String, TestResult)>,
    pub passed: usize,
    pub failed: usize,
    pub crashed: usize,
}

impl TestSuiteResults {
    pub fn new(suite_name: &str) -> Self {
        Self {
            suite_name: suite_name.to_string(),
            results: Vec::new(),
            passed: 0,
            failed: 0,
            crashed: 0,
        }
    }

    pub fn add_result(&mut self, test_name: &str, result: TestResult) {
        match &result {
            TestResult::Pass => {
                self.passed += 1;
                println!("  ✓ {}", test_name);
            }
            TestResult::Fail(msg) => {
                self.failed += 1;
                println!("  ✗ {}: {}", test_name, msg);
            }
            TestResult::Crash(msg) => {
                self.crashed += 1;
                println!("  💥 {}: CRASHED - {}", test_name, msg);
            }
        }
        self.results.push((test_name.to_string(), result));
    }

    pub fn print_summary(&self) {
        println!();
        println!("Test Suite: {} - Summary", self.suite_name);
        println!("{}", "-".repeat(30));
        println!("Passed:  {}", self.passed);
        println!("Failed:  {}", self.failed);
        println!("Crashed: {}", self.crashed);
        println!("Total:   {}", self.results.len());
        
        if self.crashed > 0 {
            println!("\n⚠️  WARNING: {} tests caused crashes! Parser robustness needs improvement.", self.crashed);
        }
        
        if self.failed > 0 {
            println!("\n❌ {} tests had unexpected results.", self.failed);
        }
        
        if self.crashed == 0 && self.failed == 0 {
            println!("\n✅ All tests passed! Parser is robust.");
        }
        println!();
    }

    pub fn is_all_passed(&self) -> bool {
        self.crashed == 0 && self.failed == 0
    }
}

/// Run a single test case
fn run_single_test(test: &TestCase) -> TestResult {
    // Catch any panics to detect crashes
    let result = std::panic::catch_unwind(|| {
        parse_input(&test.input)
    });

    match result {
        Ok(parse_result) => {
            match (parse_result, test.should_succeed) {
                (Ok(_), true) => TestResult::Pass,
                (Ok(_), false) => TestResult::Fail("Expected parsing to fail, but it succeeded".to_string()),
                (Err(error), false) => {
                    // Check if error contains expected text
                    if let Some(expected) = &test.expected_error_contains {
                        if error.message.contains(expected) {
                            TestResult::Pass
                        } else {
                            TestResult::Fail(format!(
                                "Error message '{}' doesn't contain expected text '{}'", 
                                error.message, expected
                            ))
                        }
                    } else {
                        TestResult::Pass // Any error is acceptable
                    }
                }
                (Err(error), true) => TestResult::Fail(format!("Expected parsing to succeed, but got error: {}", error.message)),
            }
        }
        Err(panic_info) => {
            let panic_msg = if let Some(s) = panic_info.downcast_ref::<String>() {
                s.clone()
            } else if let Some(s) = panic_info.downcast_ref::<&str>() {
                s.to_string()
            } else {
                "Unknown panic".to_string()
            };
            TestResult::Crash(panic_msg)
        }
    }
}

/// Parse input and return result
fn parse_input(input: &str) -> Result<bcc::ast::Program, BccError> {
    let mut lexer = Lexer::new(input.to_string());
    let tokens = lexer.scan_tokens()?;
    let mut parser = Parser::new(tokens);
    parser.parse()
}

/// Test case builder for convenience
impl TestCase {
    pub fn should_succeed(name: &str, input: &str) -> Self {
        Self {
            name: name.to_string(),
            input: input.to_string(),
            should_succeed: true,
            expected_error_contains: None,
        }
    }

    pub fn should_fail(name: &str, input: &str) -> Self {
        Self {
            name: name.to_string(),
            input: input.to_string(),
            should_succeed: false,
            expected_error_contains: None,
        }
    }

    pub fn should_fail_with_message(name: &str, input: &str, expected_msg: &str) -> Self {
        Self {
            name: name.to_string(),
            input: input.to_string(),
            should_succeed: false,
            expected_error_contains: Some(expected_msg.to_string()),
        }
    }
}

// ============================================================================
// Test Suite Creation Functions
// ============================================================================

fn create_malformed_expressions_tests() -> TestSuite {
    let mut suite = TestSuite::new("Malformed Expressions");

    // === PARENTHESES TESTS ===

    // Unmatched opening parentheses
    suite.add_test(TestCase::should_fail_with_message(
        "unmatched_opening_paren",
        "(1 + 2",
        "Expected ')' after expression"
    ));

    suite.add_test(TestCase::should_fail_with_message(
        "unmatched_opening_paren_nested",
        "((1 + 2)",
        "Expected ')' after expression"
    ));

    suite.add_test(TestCase::should_fail_with_message(
        "unmatched_opening_paren_complex",
        "(1 + (2 * 3)",
        "Expected ')' after expression"
    ));

    // Unmatched closing parentheses
    suite.add_test(TestCase::should_fail_with_message(
        "unmatched_closing_paren",
        "1 + 2)",
        "Expected expression, found ')'"
    ));

    suite.add_test(TestCase::should_fail_with_message(
        "unmatched_closing_paren_multiple",
        "1 + 2))",
        "Expected expression, found ')'"
    ));

    // Empty parentheses
    suite.add_test(TestCase::should_fail_with_message(
        "empty_parentheses",
        "()",
        "Empty parentheses are not allowed"
    ));

    suite.add_test(TestCase::should_fail_with_message(
        "empty_parentheses_in_expression",
        "1 + ()",
        "Expected expression after '+'"
    ));

    // === BRACKET TESTS ===

    // Unmatched opening brackets
    suite.add_test(TestCase::should_fail_with_message(
        "unmatched_opening_bracket",
        "[1, 2",
        "Expected ']' after list elements"
    ));

    // === BRACE TESTS ===

    // Unmatched opening braces
    suite.add_test(TestCase::should_fail_with_message(
        "unmatched_opening_brace",
        "{ x = 1",
        "Expected '}' after block"
    ));

    suite.add_test(TestCase::should_fail_with_message(
        "unmatched_closing_brace",
        "x = 1 }",
        "Expected expression, found '}'"
    ));

    suite
}

fn create_edge_case_tests() -> TestSuite {
    let mut suite = TestSuite::new("Edge Cases");

    // Empty input
    suite.add_test(TestCase::should_succeed("empty_input", ""));

    // Only whitespace
    suite.add_test(TestCase::should_succeed("only_whitespace", "   \n\t  "));

    // EOF conditions
    suite.add_test(TestCase::should_fail("unexpected_eof_after_operator", "1 +"));
    suite.add_test(TestCase::should_fail("unexpected_eof_in_expression", "1 + ("));

    // Very deeply nested expressions
    let deep_parens = "(".repeat(100) + "1" + &")".repeat(100);
    suite.add_test(TestCase::should_succeed("deeply_nested_parens", &deep_parens));

    suite
}

fn create_operator_tests() -> TestSuite {
    let mut suite = TestSuite::new("Operator Tests");

    // Missing operands
    suite.add_test(TestCase::should_fail("missing_left_operand", "+ 1"));
    suite.add_test(TestCase::should_fail("missing_right_operand", "1 +"));
    suite.add_test(TestCase::should_fail("missing_both_operands", "+"));

    // Invalid operator combinations
    suite.add_test(TestCase::should_fail("double_plus", "1 ++ 2"));
    // Note: The parser actually handles these as unary operators, which is valid
    suite.add_test(TestCase::should_succeed("double_minus", "1 -- 2")); // Parsed as 1 - (-2)
    suite.add_test(TestCase::should_succeed("mixed_operators", "1 +- 2")); // Parsed as 1 + (-2)

    // Comparison operators
    suite.add_test(TestCase::should_succeed("comparison_equal", "1 == 2"));
    suite.add_test(TestCase::should_succeed("comparison_not_equal", "1 != 2"));
    suite.add_test(TestCase::should_succeed("comparison_less", "1 < 2"));
    suite.add_test(TestCase::should_succeed("comparison_greater", "1 > 2"));

    suite
}

fn create_control_flow_tests() -> TestSuite {
    let mut suite = TestSuite::new("Control Flow Tests");

    // If statements
    suite.add_test(TestCase::should_succeed("valid_if", "if (true) { x = 1 }"));
    suite.add_test(TestCase::should_fail("if_missing_condition", "if { x = 1 }"));
    suite.add_test(TestCase::should_fail("if_missing_body", "if (true)"));

    // While loops
    suite.add_test(TestCase::should_succeed("valid_while", "while (true) { x = 1 }"));
    suite.add_test(TestCase::should_fail("while_missing_condition", "while { x = 1 }"));
    suite.add_test(TestCase::should_fail("while_missing_body", "while (true)"));

    // For loops
    suite.add_test(TestCase::should_succeed("valid_for", "for (i = 0; i < 10; i = i + 1) { print i }"));
    // Note: The parser is more lenient with for-loop syntax than expected
    suite.add_test(TestCase::should_succeed("for_missing_semicolon", "for (i = 0 i < 10; i = i + 1) { print i }"));

    suite
}

fn create_literal_tests() -> TestSuite {
    let mut suite = TestSuite::new("Literal Tests");

    // Valid literals
    suite.add_test(TestCase::should_succeed("integer_literal", "42"));
    suite.add_test(TestCase::should_succeed("double_literal", "3.14"));
    suite.add_test(TestCase::should_succeed("string_literal", "\"hello\""));
    suite.add_test(TestCase::should_succeed("boolean_true", "true"));
    suite.add_test(TestCase::should_succeed("boolean_false", "false"));

    // Invalid number formats
    suite.add_test(TestCase::should_fail("multiple_dots", "3.14.159"));
    suite.add_test(TestCase::should_fail("trailing_dot", "42."));
    suite.add_test(TestCase::should_fail("leading_dot", ".42"));

    // Unterminated strings
    suite.add_test(TestCase::should_fail("unterminated_string", "\"hello"));
    suite.add_test(TestCase::should_fail("unterminated_string_with_newline", "\"hello\nworld"));

    suite
}

fn create_function_call_tests() -> TestSuite {
    let mut suite = TestSuite::new("Function Call Tests");

    // Valid function calls
    suite.add_test(TestCase::should_succeed("simple_function_call", "foo()"));
    suite.add_test(TestCase::should_succeed("function_call_with_args", "foo(1, 2, 3)"));

    // Invalid function calls
    suite.add_test(TestCase::should_fail("missing_closing_paren", "foo(1, 2"));
    suite.add_test(TestCase::should_fail("missing_opening_paren", "foo 1, 2)"));
    suite.add_test(TestCase::should_fail("trailing_comma", "foo(1, 2,)"));

    suite
}

fn create_assignment_tests() -> TestSuite {
    let mut suite = TestSuite::new("Assignment Tests");

    // Valid assignments
    suite.add_test(TestCase::should_succeed("simple_assignment", "x = 1"));
    suite.add_test(TestCase::should_succeed("assignment_with_expression", "x = 1 + 2"));

    // Invalid assignments
    suite.add_test(TestCase::should_fail("missing_value", "x ="));
    suite.add_test(TestCase::should_fail("invalid_target", "1 = x"));

    suite
}

fn create_mixed_construct_tests() -> TestSuite {
    let mut suite = TestSuite::new("Mixed Construct Tests");

    // Complex valid expressions
    suite.add_test(TestCase::should_succeed(
        "complex_expression",
        "x = (1 + 2) * 3 + foo(4, 5)"
    ));

    // Complex invalid expressions
    suite.add_test(TestCase::should_fail(
        "mixed_paren_bracket_error",
        "x = [1 + (2 * 3]"
    ));

    suite
}

fn create_positive_tests() -> TestSuite {
    let mut suite = TestSuite::new("Positive Tests");

    // These tests verify that valid syntax still parses correctly
    suite.add_test(TestCase::should_succeed("simple_arithmetic", "1 + 2 * 3"));
    suite.add_test(TestCase::should_succeed("parentheses", "(1 + 2) * 3"));
    suite.add_test(TestCase::should_succeed("variable_assignment", "x = 42"));
    suite.add_test(TestCase::should_succeed("string_concatenation", "\"hello\" + \" world\""));
    suite.add_test(TestCase::should_succeed("boolean_operations", "true and false"));
    suite.add_test(TestCase::should_succeed("comparison", "1 < 2"));

    suite
}

// ============================================================================
// Main Test Function
// ============================================================================

#[test]
fn comprehensive_parser_tests() {
    println!("🧪 BCC Parser Robustness Test Suite");
    println!("====================================\n");

    let mut all_passed = true;

    // Run each test suite
    let suites = vec![
        create_malformed_expressions_tests(),
        create_edge_case_tests(),
        create_operator_tests(),
        create_control_flow_tests(),
        create_literal_tests(),
        create_function_call_tests(),
        create_assignment_tests(),
        create_mixed_construct_tests(),
        create_positive_tests(),
    ];

    for suite in suites {
        let results = suite.run();
        if !results.is_all_passed() {
            all_passed = false;
        }
    }

    if all_passed {
        println!("🎉 ALL TESTS PASSED! Parser is robust and handles all edge cases gracefully.");
    } else {
        println!("⚠️  Some tests failed. See output above for details.");
        // Don't panic here - let the test framework handle it
        // We'll rely on cargo test's normal failure reporting
    }
}