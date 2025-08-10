mod lexer;
mod parser;
mod evaluator;
mod error;
mod repl;
mod runner;
mod value;
mod ast;

use clap::{Arg, Command};
use std::fs;
use std::path::Path;

fn main() {
    let matches = Command::new("bcc")
        .about("A Lox-like interpreter with excellent error diagnostics")
        .arg(
            Arg::new("file")
                .help("The script file to execute")
                .value_name("FILE")
                .index(1),
        )
        .arg(
            Arg::new("interactive")
                .short('i')
                .long("interactive")
                .help("Start in interactive REPL mode")
                .action(clap::ArgAction::SetTrue),
        )
        .get_matches();

    if let Some(file_path) = matches.get_one::<String>("file") {
        run_file(file_path);
    } else if matches.get_flag("interactive") || matches.get_one::<String>("file").is_none() {
        repl::start();
    }
}

fn run_file(path: &str) {
    let path = Path::new(path);
    
    if !path.exists() {
        eprintln!("Error: File '{}' not found", path.display());
        std::process::exit(1);
    }

    match fs::read_to_string(path) {
        Ok(source) => {
            runner::run(&source, Some(path.to_str().unwrap()));
        }
        Err(e) => {
            eprintln!("Error reading file '{}': {}", path.display(), e);
            std::process::exit(1);
        }
    }
}