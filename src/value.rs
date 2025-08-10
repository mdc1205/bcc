use std::fmt;
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Nil,
    Bool(bool),
    Int(i64),
    Double(f64),
    String(String),
    List(Vec<Value>),
    Dict(HashMap<String, Value>),
    CaseResult(CaseResult),
    /// Tuple type for multi-return values and grouped expressions
    Tuple(Vec<Value>),
}

#[derive(Debug, Clone, PartialEq)]
pub struct CaseResult {
    pub result: Box<Value>,
}

impl Value {
    pub fn is_truthy(&self) -> bool {
        match self {
            Value::Nil => false,
            Value::Bool(b) => *b,
            Value::Int(n) => *n != 0,
            Value::Double(n) => *n != 0.0,
            Value::String(s) => !s.is_empty(),
            Value::List(l) => !l.is_empty(),
            Value::Dict(d) => !d.is_empty(),
            Value::CaseResult(case_result) => case_result.result.is_truthy(),
            Value::Tuple(t) => !t.is_empty(),
        }
    }

    pub fn type_name(&self) -> &'static str {
        match self {
            Value::Nil => "nil",
            Value::Bool(_) => "bool",
            Value::Int(_) => "int",
            Value::Double(_) => "double", 
            Value::String(_) => "string",
            Value::List(_) => "list",
            Value::Dict(_) => "dict",
            Value::CaseResult(_) => "case_result",
            Value::Tuple(_) => "tuple",
        }
    }
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Value::Nil => write!(f, "nil"),
            Value::Bool(b) => write!(f, "{}", b),
            Value::Int(n) => write!(f, "{}", n),
            Value::Double(n) => {
                // Always show at least one decimal place for doubles
                if n.fract() == 0.0 {
                    write!(f, "{:.1}", n)
                } else {
                    write!(f, "{}", n)
                }
            },
            Value::String(s) => write!(f, "{}", s),
            Value::List(l) => {
                write!(f, "[")?;
                for (i, item) in l.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", item)?;
                }
                write!(f, "]")
            },
            Value::Dict(d) => {
                write!(f, "{{")?;
                let mut first = true;
                for (key, value) in d.iter() {
                    if !first {
                        write!(f, ", ")?;
                    }
                    write!(f, "\"{}\": {}", key, value)?;
                    first = false;
                }
                write!(f, "}}")
            },
            Value::CaseResult(case_result) => write!(f, "<case_result: {}>", case_result.result),
            Value::Tuple(t) => {
                write!(f, "(")?;
                for (i, item) in t.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", item)?;
                }
                // Always show trailing comma for single-element tuples to distinguish from grouping
                if t.len() == 1 {
                    write!(f, ",")?;
                }
                write!(f, ")")
            },
        }
    }
}