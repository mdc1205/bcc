// BCC Language Interpreter Library
//
// This is the core library for the BCC language interpreter, a Lox-like
// language with Python-like syntax and excellent error diagnostics.

// Public modules
pub mod ast;
pub mod error;
pub mod evaluator;
pub mod lexer;
pub mod parser;
pub mod repl;
pub mod runner;
pub mod value;

// Re-export commonly used items
pub use ast::{Expr, Stmt, Program};
pub use error::{BccError, Span};
pub use evaluator::Evaluator;
pub use lexer::{Lexer, Token, TokenType};
pub use parser::Parser;
pub use value::Value;

// Re-export main functions
pub use repl::start as start_repl;
pub use runner::run;