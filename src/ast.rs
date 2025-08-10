use crate::error::Span;
use crate::value::Value;

/// Simplified AST using owned strings for better maintainability.
/// Prioritizes code clarity over memory efficiency.

#[derive(Debug, Clone)]
pub struct Program {
    pub statements: Vec<Stmt>,
}

#[derive(Debug, Clone)]
pub enum Stmt {
    Expression {
        expr: Expr,
        span: Span,
    },
    Block {
        statements: Vec<Stmt>,
        span: Span,
    },
    If {
        condition: Expr,
        then_branch: Box<Stmt>,
        else_branch: Option<Box<Stmt>>,
        span: Span,
    },
    While {
        condition: Expr,
        body: Box<Stmt>,
        span: Span,
    },
    For {
        initializer: Option<Box<Stmt>>,
        condition: Option<Expr>,
        increment: Option<Expr>,
        body: Box<Stmt>,
        span: Span,
    },
}

impl Stmt {
    pub fn span(&self) -> &Span {
        match self {
            Stmt::Expression { span, .. } => span,
            Stmt::Block { span, .. } => span,
            Stmt::If { span, .. } => span,
            Stmt::While { span, .. } => span,
            Stmt::For { span, .. } => span,
        }
    }
}

#[derive(Debug, Clone)]
pub enum Expr {
    Literal {
        value: Value,
        span: Span,
    },
    Variable {
        name: String,  // Simplified: owned string for better maintainability
        span: Span,
    },
    Assign {
        name: String,  // Simplified: owned string for better maintainability
        value: Box<Expr>,
        span: Span,
    },
    /// Multi-target assignment for destructuring: a, b, c = expr
    MultiAssign {
        targets: Vec<AssignTarget>,
        value: Box<Expr>,
        span: Span,
    },
    Binary {
        left: Box<Expr>,
        operator: BinaryOp,
        right: Box<Expr>,
        span: Span,
    },
    Unary {
        operator: UnaryOp,
        operand: Box<Expr>,
        span: Span,
    },
    Logical {
        left: Box<Expr>,
        operator: LogicalOp,
        right: Box<Expr>,
        span: Span,
    },
    Call {
        callee: Box<Expr>,
        args: Vec<Expr>,
        span: Span,
    },
    /// Enhanced function call with keyword arguments support
    CallWithKwargs {
        callee: Box<Expr>,
        positional_args: Vec<Expr>,
        keyword_args: Vec<KeywordArg>,
        span: Span,
    },
    /// Multi-return expression: return a, b, c
    MultiReturn {
        values: Vec<Expr>,
        span: Span,
    },
    Grouping {
        expr: Box<Expr>,
        span: Span,
    },
    List {
        elements: Vec<Expr>,
        span: Span,
    },
    Dict {
        pairs: Vec<(Expr, Expr)>,
        span: Span,
    },
    PropertyAccess {
        object: Box<Expr>,
        property: String,
        span: Span,
    },
    /// Tuple expression for multi-value contexts: (a, b, c)
    Tuple {
        elements: Vec<Expr>,
        span: Span,
    },
}

/// Represents a target in multi-assignment: variable name or underscore (ignore)
#[derive(Debug, Clone)]
pub enum AssignTarget {
    Variable { name: String, span: Span },
    Ignore { span: Span },  // For underscore targets: a, _, c = expr
}

/// Represents a keyword argument in function calls: name=value
#[derive(Debug, Clone)]
pub struct KeywordArg {
    pub name: String,
    pub value: Expr,
    pub span: Span,
}

impl Expr {
    pub fn span(&self) -> &Span {
        match self {
            Expr::Literal { span, .. } => span,
            Expr::Variable { span, .. } => span,
            Expr::Assign { span, .. } => span,
            Expr::MultiAssign { span, .. } => span,
            Expr::Binary { span, .. } => span,
            Expr::Unary { span, .. } => span,
            Expr::Logical { span, .. } => span,
            Expr::Call { span, .. } => span,
            Expr::CallWithKwargs { span, .. } => span,
            Expr::MultiReturn { span, .. } => span,
            Expr::Grouping { span, .. } => span,
            Expr::List { span, .. } => span,
            Expr::Dict { span, .. } => span,
            Expr::PropertyAccess { span, .. } => span,
            Expr::Tuple { span, .. } => span,
        }
    }
}

#[derive(Debug, Clone)]
pub enum BinaryOp {
    Add,
    Subtract,
    Multiply,
    Divide,
    Equal,
    NotEqual,
    Less,
    LessEqual,
    Greater,
    GreaterEqual,
    In,
}

#[derive(Debug, Clone)]
pub enum UnaryOp {
    Negate,
    Not,
}

#[derive(Debug, Clone)]
pub enum LogicalOp {
    And,
    Or,
}