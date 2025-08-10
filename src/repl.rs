use crate::evaluator::Evaluator;
use crate::lexer::Lexer;
use crate::parser::Parser;
use std::io::{self, Write};

/// Simplified REPL using owned strings for better maintainability.
/// Prioritizes code clarity and maintains persistent state between commands.

pub fn start() {
    println!("BCC Interpreter v0.1.0");
    println!("Type 'exit' or press Ctrl+C to quit");
    println!();

    // Create a persistent evaluator to maintain state between commands
    let mut evaluator = Evaluator::new();

    loop {
        print!("> ");
        io::stdout().flush().unwrap();

        let mut line = String::new();
        match io::stdin().read_line(&mut line) {
            Ok(0) => {
                // EOF reached (Ctrl+D or piped input ended)
                println!(); // Add newline for clean exit
                break;
            }
            Ok(_) => {
                let line = line.trim();
                if line.is_empty() {
                    continue;
                }
                if line == "exit" || line == "quit" {
                    println!("Goodbye!");
                    break;
                }

                // Parse and evaluate the input with persistent state
                run_repl_command(line, &mut evaluator);
            }
            Err(error) => {
                eprintln!("Error reading input: {}", error);
                break;
            }
        }
    }
}

fn run_repl_command(source: &str, evaluator: &mut Evaluator) {
    // Lexical analysis
    let mut lexer = Lexer::new(source.to_string());
    let tokens = match lexer.scan_tokens() {
        Ok(tokens) => tokens,
        Err(error) => {
            error.report(source, None);
            return;
        }
    };

    // Parsing
    let mut parser = Parser::new(tokens);
    let program = match parser.parse() {
        Ok(program) => program,
        Err(error) => {
            error.report(source, None);
            return;
        }
    };

    // Check if it's a single expression statement and display its value (but not assignments)
    if program.statements.len() == 1 {
        if let crate::ast::Stmt::Expression { expr, .. } = &program.statements[0] {
            // Don't display assignment values
            if !matches!(expr, crate::ast::Expr::Assign { .. }) {
                match evaluator.evaluate_expression(expr) {
                    Ok(value) => {
                        println!("{}", value);
                        return;
                    }
                    Err(error) => {
                        error.report(source, None);
                        return;
                    }
                }
            }
        }
    }

    // Otherwise, evaluate the program normally (for assignments, print statements, etc.)
    if let Err(error) = evaluator.evaluate_program(&program) {
        error.report(source, None);
    }
}