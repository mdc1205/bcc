use ariadne::{Color, Fmt, Label, Report, ReportKind, Source};
use std::fmt;

#[derive(Debug, Clone)]
pub struct Span {
    pub start: usize,
    pub end: usize,
}

impl Span {
    pub fn new(start: usize, end: usize) -> Self {
        Self { start, end }
    }

    pub fn single(pos: usize) -> Self {
        Self {
            start: pos,
            end: pos + 1,
        }
    }
}

#[derive(Debug, Clone)]
pub enum ErrorKind {
    LexError,
    ParseError,
    RuntimeError,
}

#[derive(Debug, Clone)]
pub struct BccError {
    pub kind: ErrorKind,
    pub span: Span,
    pub message: String,
    pub help: Option<String>,
}

impl BccError {
    pub fn new(kind: ErrorKind, span: Span, message: String) -> Self {
        Self {
            kind,
            span,
            message,
            help: None,
        }
    }

    pub fn new_with_help(kind: ErrorKind, span: Span, message: String, help: String) -> Self {
        Self {
            kind,
            span,
            message,
            help: Some(help),
        }
    }

    pub fn lex_error(span: Span, message: String) -> Self {
        Self::new(ErrorKind::LexError, span, message)
    }

    pub fn parse_error(span: Span, message: String) -> Self {
        Self::new(ErrorKind::ParseError, span, message)
    }

    pub fn parse_error_with_help(span: Span, message: String, help: String) -> Self {
        Self::new_with_help(ErrorKind::ParseError, span, message, help)
    }

    pub fn runtime_error(span: Span, message: String) -> Self {
        Self::new(ErrorKind::RuntimeError, span, message)
    }

    pub fn runtime_error_with_help(span: Span, message: String, help: String) -> Self {
        Self::new_with_help(ErrorKind::RuntimeError, span, message, help)
    }

    pub fn report(&self, source: &str, filename: Option<&str>) {
        let filename = filename.unwrap_or("<repl>");
        
        let color = match self.kind {
            ErrorKind::LexError => Color::Red,
            ErrorKind::ParseError => Color::Yellow,
            ErrorKind::RuntimeError => Color::Magenta,
        };

        let kind_str = match self.kind {
            ErrorKind::LexError => "Lexical Error",
            ErrorKind::ParseError => "Parse Error", 
            ErrorKind::RuntimeError => "Runtime Error",
        };

        let mut report_builder = Report::build(ReportKind::Error, filename, self.span.start)
            .with_message(format!("{}: {}", kind_str.fg(color), self.message))
            .with_label(
                Label::new((filename, self.span.start..self.span.end))
                    .with_message(&self.message)
                    .with_color(color),
            );

        // Add help note if available
        if let Some(ref help_text) = self.help {
            report_builder = report_builder.with_note(format!("{}: {}", "help".fg(Color::Cyan), help_text));
        }

        report_builder
            .finish()
            .print((filename, Source::from(source)))
            .unwrap();
    }
}

impl fmt::Display for BccError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for BccError {}