use crate::evaluator::Evaluator;
use crate::lexer::Lexer;
use crate::parser::Parser;

/// Simplified runner using owned strings for better maintainability.
/// Prioritizes code clarity over memory efficiency.

pub fn run(source: &str, filename: Option<&str>) {
    // Lexical analysis
    let mut lexer = Lexer::new(source.to_string());
    let tokens = match lexer.scan_tokens() {
        Ok(tokens) => tokens,
        Err(error) => {
            error.report(source, filename);
            return;
        }
    };

    // Parsing
    let mut parser = Parser::new(tokens);
    let program = match parser.parse() {
        Ok(program) => program,
        Err(error) => {
            error.report(source, filename);
            return;
        }
    };

    // Evaluation
    let mut evaluator = Evaluator::new();
    if let Err(error) = evaluator.evaluate_program(&program) {
        error.report(source, filename);
    }
}