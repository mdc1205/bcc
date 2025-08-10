use crate::ast::{BinaryOp, Expr, LogicalOp, Program, Stmt, UnaryOp};
use crate::error::{BccError, Span};
use crate::value::{Value, CaseResult};
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct Environment {
    values: HashMap<String, Value>,
    enclosing: Option<Box<Environment>>,
}

impl Environment {
    pub fn new() -> Self {
        Self {
            values: HashMap::new(),
            enclosing: None,
        }
    }

    pub fn with_enclosing(enclosing: Environment) -> Self {
        Self {
            values: HashMap::new(),
            enclosing: Some(Box::new(enclosing)),
        }
    }

    pub fn get(&self, name: &str) -> Option<Value> {
        if let Some(value) = self.values.get(name) {
            Some(value.clone())
        } else if let Some(ref enclosing) = self.enclosing {
            enclosing.get(name)
        } else {
            None
        }
    }

    pub fn assign(&mut self, name: &str, value: Value) -> Result<(), BccError> {
        if self.values.contains_key(name) {
            self.values.insert(name.to_string(), value);
            Ok(())
        } else if let Some(ref mut enclosing) = self.enclosing {
            enclosing.assign(name, value)
        } else {
            // For Python-like behavior, create the variable if it doesn't exist
            self.values.insert(name.to_string(), value);
            Ok(())
        }
    }
}

pub struct Evaluator {
    environment: Environment,
}

impl Evaluator {
    pub fn new() -> Self {
        let mut evaluator = Self {
            environment: Environment::new(),
        };
        
        // Add built-in functions
        evaluator.environment.assign("print", Value::String("__builtin_print__".to_string())).unwrap();
        evaluator.environment.assign("len", Value::String("__builtin_len__".to_string())).unwrap();
        evaluator.environment.assign("type", Value::String("__builtin_type__".to_string())).unwrap();
        evaluator.environment.assign("case", Value::String("__builtin_case__".to_string())).unwrap();
        evaluator.environment.assign("divmod", Value::String("__builtin_divmod__".to_string())).unwrap();
        
        evaluator
    }

    pub fn evaluate_program(&mut self, program: &Program) -> Result<(), BccError> {
        for statement in &program.statements {
            self.execute_statement(statement)?;
        }
        Ok(())
    }

    fn execute_statement(&mut self, stmt: &Stmt) -> Result<(), BccError> {
        match stmt {
            Stmt::Expression { expr, .. } => {
                self.evaluate_expression(expr)?;
                Ok(())
            }
            Stmt::Block { statements, .. } => {
                self.execute_block(statements)
            }
            Stmt::If {
                condition,
                then_branch,
                else_branch,
                ..
            } => {
                let condition_value = self.evaluate_expression(condition)?;
                if condition_value.is_truthy() {
                    self.execute_statement(then_branch)?;
                } else if let Some(else_stmt) = else_branch {
                    self.execute_statement(else_stmt)?;
                }
                Ok(())
            }
            Stmt::While { condition, body, .. } => {
                while self.evaluate_expression(condition)?.is_truthy() {
                    self.execute_statement(body)?;
                }
                Ok(())
            }
            Stmt::For {
                initializer,
                condition,
                increment,
                body,
                ..
            } => {
                // Execute initializer if present
                if let Some(init) = initializer {
                    self.execute_statement(init)?;
                }

                // Execute loop
                loop {
                    // Check condition
                    if let Some(cond) = condition {
                        if !self.evaluate_expression(cond)?.is_truthy() {
                            break;
                        }
                    }

                    // Execute body
                    self.execute_statement(body)?;

                    // Execute increment
                    if let Some(inc) = increment {
                        self.evaluate_expression(inc)?;
                    }
                }
                Ok(())
            }
        }
    }

    fn execute_block(&mut self, statements: &[Stmt]) -> Result<(), BccError> {
        let previous_env = self.environment.clone();
        self.environment = Environment::with_enclosing(previous_env);

        let result = (|| {
            for statement in statements {
                self.execute_statement(statement)?;
            }
            Ok(())
        })();

        // Extract the enclosing environment which may have been modified
        if let Some(enclosing) = self.environment.enclosing.take() {
            self.environment = *enclosing;
        }
        
        result
    }

    pub fn evaluate_expression(&mut self, expr: &Expr) -> Result<Value, BccError> {
        match expr {
            Expr::Literal { value, .. } => Ok(value.clone()),
            Expr::Variable { name, span } => {
                self.environment.get(name).ok_or_else(|| {
                    BccError::runtime_error(
                        span.clone(),
                        format!("Undefined variable '{}'", name),
                    )
                })
            }
            Expr::Assign { name, value, span } => {
                let val = self.evaluate_expression(value)?;
                self.environment.assign(name, val.clone()).map_err(|_| {
                    BccError::runtime_error(
                        span.clone(),
                        format!("Undefined variable '{}'", name),
                    )
                })?;
                Ok(val)
            }
            Expr::Binary {
                left,
                operator,
                right,
                span,
            } => {
                let left_val = self.evaluate_expression(left)?;
                let right_val = self.evaluate_expression(right)?;
                self.evaluate_binary_op(operator, left_val, right_val, span)
            }
            Expr::Unary {
                operator, operand, span
            } => {
                let operand_val = self.evaluate_expression(operand)?;
                self.evaluate_unary_op(operator, operand_val, span)
            }
            Expr::Logical {
                left,
                operator,
                right,
                ..
            } => {
                let left_val = self.evaluate_expression(left)?;

                match operator {
                    LogicalOp::Or => {
                        if left_val.is_truthy() {
                            Ok(left_val)
                        } else {
                            self.evaluate_expression(right)
                        }
                    }
                    LogicalOp::And => {
                        if !left_val.is_truthy() {
                            Ok(left_val)
                        } else {
                            self.evaluate_expression(right)
                        }
                    }
                }
            }
            Expr::Call { callee, args, span } => {
                let function_value = self.evaluate_expression(callee)?;
                
                // Check if it's a built-in function
                if let Value::String(name) = function_value {
                    match name.as_str() {
                        "__builtin_print__" => {
                            // Handle print built-in
                            for arg in args {
                                let value = self.evaluate_expression(arg)?;
                                println!("{}", value);
                            }
                            return Ok(Value::Nil);
                        },
                        "__builtin_len__" => {
                            // Handle len built-in
                            if args.len() != 1 {
                                return Err(BccError::runtime_error_with_help(
                                    span.clone(),
                                    format!("len() takes exactly 1 argument, got {}", args.len()),
                                    "Usage: len(value) where value is a string, list, or dictionary.".to_string(),
                                ));
                            }
                            
                            let arg_value = self.evaluate_expression(&args[0])?;
                            match arg_value {
                                Value::String(s) => return Ok(Value::Int(s.chars().count() as i64)),
                                Value::List(l) => return Ok(Value::Int(l.len() as i64)),
                                Value::Dict(d) => return Ok(Value::Int(d.len() as i64)),
                                _ => return Err(BccError::runtime_error_with_help(
                                    span.clone(),
                                    format!("len() not supported for type {}", arg_value.type_name()),
                                    "len() only works with strings, lists, and dictionaries.".to_string(),
                                )),
                            }
                        },
                        "__builtin_type__" => {
                            // Handle type built-in
                            if args.len() != 1 {
                                return Err(BccError::runtime_error_with_help(
                                    span.clone(),
                                    format!("type() takes exactly 1 argument, got {}", args.len()),
                                    "Usage: type(value) returns the type name as a string.".to_string(),
                                ));
                            }
                            
                            let arg_value = self.evaluate_expression(&args[0])?;
                            return Ok(Value::String(arg_value.type_name().to_string()));
                        },
                        "__builtin_case__" => {
                            // Handle case built-in
                            if args.len() < 2 || args.len() % 2 != 0 {
                                return Err(BccError::runtime_error_with_help(
                                    span.clone(),
                                    format!("case() requires an even number of arguments (at least 2), got {}", args.len()),
                                    "Usage: case(condition1, result1, condition2, result2, ...). Each condition is paired with its result.".to_string(),
                                ));
                            }
                            
                            // Evaluate condition-result pairs in order
                            for i in (0..args.len()).step_by(2) {
                                let condition_value = self.evaluate_expression(&args[i])?;
                                if condition_value.is_truthy() {
                                    let result_value = self.evaluate_expression(&args[i + 1])?;
                                    return Ok(Value::CaseResult(CaseResult {
                                        result: Box::new(result_value),
                                    }));
                                }
                            }
                            
                            // If no condition matches, return nil wrapped in CaseResult
                            return Ok(Value::CaseResult(CaseResult {
                                result: Box::new(Value::Nil),
                            }));
                        },
                        "__builtin_divmod__" => {
                            // Handle divmod built-in without kwargs (default rounding mode)
                            if args.len() != 2 {
                                return Err(BccError::runtime_error_with_help(
                                    span.clone(),
                                    format!("divmod() takes exactly 2 arguments, got {}", args.len()),
                                    "Usage: divmod(dividend, divisor) - use divmod(a, b, round_mode=\"up\") for keyword arguments.".to_string(),
                                ));
                            }
                            
                            let dividend = self.evaluate_expression(&args[0])?;
                            let divisor = self.evaluate_expression(&args[1])?;
                            
                            return self.builtin_divmod(dividend, divisor, "down".to_string(), span);
                        },
                        _ => {}
                    }
                }
                
                // For other function calls, return error since we haven't implemented user-defined functions
                Err(BccError::runtime_error_with_help(
                    span.clone(),
                    "User-defined functions not yet implemented".to_string(),
                    "Only built-in functions like print(), len(), and type() are currently supported.".to_string(),
                ))
            }
            Expr::Grouping { expr, .. } => self.evaluate_expression(expr),
            Expr::List { elements, .. } => {
                let mut list_values = Vec::new();
                for element in elements {
                    list_values.push(self.evaluate_expression(element)?);
                }
                Ok(Value::List(list_values))
            }
            Expr::Dict { pairs, span } => {
                let mut dict_values = HashMap::new();
                for (key_expr, value_expr) in pairs {
                    let key_value = self.evaluate_expression(key_expr)?;
                    let value_value = self.evaluate_expression(value_expr)?;
                    
                    // Dictionary keys must be strings
                    let key_string = match key_value {
                        Value::String(s) => s,
                        _ => return Err(BccError::runtime_error(
                            span.clone(),
                            format!("Dictionary keys must be strings, got {}", key_value.type_name()),
                        )),
                    };
                    
                    dict_values.insert(key_string, value_value);
                }
                Ok(Value::Dict(dict_values))
            }
            Expr::PropertyAccess { object, property, span } => {
                let object_value = self.evaluate_expression(object)?;
                
                match object_value {
                    Value::CaseResult(case_result) => {
                        match property.as_str() {
                            "result" => Ok(*case_result.result),
                            _ => Err(BccError::runtime_error_with_help(
                                span.clone(),
                                format!("Unknown property '{}' on case_result", property),
                                "case_result objects only have a 'result' property.".to_string(),
                            )),
                        }
                    },
                    _ => Err(BccError::runtime_error_with_help(
                        span.clone(),
                        format!("Property access not supported for type {}", object_value.type_name()),
                        "Property access is currently only supported for case_result objects.".to_string(),
                    )),
                }
            }
            Expr::MultiAssign { targets, value, span } => {
                let values = self.evaluate_expression(value)?;
                
                // Handle multi-assignment unpacking
                let unpacked_values = match &values {
                    Value::Tuple(tuple_values) => tuple_values.clone(),
                    Value::List(list_values) => list_values.clone(),
                    single_value => vec![single_value.clone()], // Single value gets wrapped in a list
                };
                
                // Check if we have the right number of values
                let non_ignore_targets = targets.iter().filter(|t| matches!(t, crate::ast::AssignTarget::Variable { .. })).count();
                
                if unpacked_values.len() < non_ignore_targets {
                    return Err(BccError::runtime_error_with_help(
                        span.clone(),
                        format!("Not enough values to unpack (expected {}, got {})", targets.len(), unpacked_values.len()),
                        "Multi-assignment requires the same number of values as targets. Use _ to ignore extra values.".to_string(),
                    ));
                }
                
                // Assign values to targets
                for (i, target) in targets.iter().enumerate() {
                    match target {
                        crate::ast::AssignTarget::Variable { name, .. } => {
                            if i < unpacked_values.len() {
                                self.environment.assign(name, unpacked_values[i].clone())?;
                            }
                        }
                        crate::ast::AssignTarget::Ignore { .. } => {
                            // Ignore this value
                        }
                    }
                }
                
                // Return the original tuple/list for chaining
                Ok(values)
            }
            Expr::CallWithKwargs { callee, positional_args, keyword_args, span } => {
                let function_value = self.evaluate_expression(callee)?;
                
                // Check if it's a built-in function
                if let Value::String(name) = function_value {
                    match name.as_str() {
                        "__builtin_divmod__" => {
                            // Example builtin showcasing kwargs: divmod(a, b, round_mode="down") 
                            if positional_args.len() != 2 {
                                return Err(BccError::runtime_error_with_help(
                                    span.clone(),
                                    format!("divmod() takes exactly 2 positional arguments, got {}", positional_args.len()),
                                    "Usage: divmod(dividend, divisor, round_mode=\"down\")".to_string(),
                                ));
                            }
                            
                            let dividend = self.evaluate_expression(&positional_args[0])?;
                            let divisor = self.evaluate_expression(&positional_args[1])?;
                            
                            // Parse keyword arguments for rounding mode
                            let mut round_mode = "down".to_string();
                            for kwarg in keyword_args {
                                match kwarg.name.as_str() {
                                    "round_mode" => {
                                        let mode_value = self.evaluate_expression(&kwarg.value)?;
                                        if let Value::String(mode) = mode_value {
                                            round_mode = mode;
                                        } else {
                                            return Err(BccError::runtime_error_with_help(
                                                span.clone(),
                                                "round_mode must be a string".to_string(),
                                                "Valid round modes: \"down\", \"up\", \"nearest\"".to_string(),
                                            ));
                                        }
                                    }
                                    _ => {
                                        return Err(BccError::runtime_error_with_help(
                                            span.clone(),
                                            format!("Unknown keyword argument '{}' for divmod()", kwarg.name),
                                            "Valid keyword arguments: round_mode".to_string(),
                                        ));
                                    }
                                }
                            }
                            
                            // Perform division with the specified rounding mode
                            return self.builtin_divmod(dividend, divisor, round_mode, span);
                        }
                        _ => {
                            // For now, fall back to regular call handling for other builtins
                            // This allows kwargs calls to regular functions to work
                            let all_args = positional_args.clone();
                            
                            // For now, we'll ignore keyword args for non-kwargs builtins
                            // In a real implementation, you'd validate the kwargs
                            if !keyword_args.is_empty() {
                                return Err(BccError::runtime_error_with_help(
                                    span.clone(),
                                    format!("Function '{}' does not accept keyword arguments", name.replace("__builtin_", "").replace("__", "")),
                                    "Only some built-in functions support keyword arguments.".to_string(),
                                ));
                            }
                            
                            // Reuse the regular call logic
                            return self.evaluate_expression(&Expr::Call {
                                callee: Box::new(Expr::Variable { 
                                    name: name.clone(), 
                                    span: callee.span().clone() 
                                }),
                                args: all_args,
                                span: span.clone(),
                            });
                        }
                    }
                }
                
                Err(BccError::runtime_error_with_help(
                    span.clone(),
                    "User-defined functions with kwargs not yet implemented".to_string(),
                    "Only built-in functions support keyword arguments currently.".to_string(),
                ))
            }
            Expr::MultiReturn { values, .. } => {
                let mut result_values = Vec::new();
                for value_expr in values {
                    result_values.push(self.evaluate_expression(value_expr)?);
                }
                Ok(Value::Tuple(result_values))
            }
            Expr::Tuple { elements, .. } => {
                let mut tuple_values = Vec::new();
                for element in elements {
                    tuple_values.push(self.evaluate_expression(element)?);
                }
                Ok(Value::Tuple(tuple_values))
            }
        }
    }

    fn builtin_divmod(&self, dividend: Value, divisor: Value, round_mode: String, span: &Span) -> Result<Value, BccError> {
        let dividend_type = dividend.type_name();
        let divisor_type = divisor.type_name();
        match (dividend, divisor) {
            (Value::Int(a), Value::Int(b)) => {
                if b == 0 {
                    return Err(BccError::runtime_error(
                        span.clone(),
                        "Division by zero".to_string(),
                    ));
                }
                
                let (quotient, remainder) = match round_mode.as_str() {
                    "down" => (a / b, a % b),
                    "up" => {
                        let q = if a * b >= 0 { (a + b - 1) / b } else { a / b };
                        (q, a - q * b)
                    }
                    "nearest" => {
                        let q = ((a as f64) / (b as f64)).round() as i64;
                        (q, a - q * b)
                    }
                    _ => return Err(BccError::runtime_error_with_help(
                        span.clone(),
                        format!("Unknown rounding mode '{}'", round_mode),
                        "Valid rounding modes: \"down\", \"up\", \"nearest\"".to_string(),
                    )),
                };
                
                Ok(Value::Tuple(vec![Value::Int(quotient), Value::Int(remainder)]))
            }
            (Value::Double(a), Value::Double(b)) => {
                if b == 0.0 {
                    return Err(BccError::runtime_error(
                        span.clone(),
                        "Division by zero".to_string(),
                    ));
                }
                
                let quotient = match round_mode.as_str() {
                    "down" => (a / b).floor(),
                    "up" => (a / b).ceil(),
                    "nearest" => (a / b).round(),
                    _ => return Err(BccError::runtime_error_with_help(
                        span.clone(),
                        format!("Unknown rounding mode '{}'", round_mode),
                        "Valid rounding modes: \"down\", \"up\", \"nearest\"".to_string(),
                    )),
                };
                let remainder = a - quotient * b;
                
                Ok(Value::Tuple(vec![Value::Double(quotient), Value::Double(remainder)]))
            }
            (Value::Int(a), Value::Double(b)) => {
                self.builtin_divmod(Value::Double(a as f64), Value::Double(b), round_mode, span)
            }
            (Value::Double(a), Value::Int(b)) => {
                self.builtin_divmod(Value::Double(a), Value::Double(b as f64), round_mode, span)
            }
            _ => Err(BccError::runtime_error_with_help(
                span.clone(),
                format!("divmod() not supported for types {} and {}", dividend_type, divisor_type),
                "divmod() only works with numbers (int and double).".to_string(),
            )),
        }
    }

    fn evaluate_binary_op(
        &self,
        operator: &BinaryOp,
        left: Value,
        right: Value,
        span: &Span,
    ) -> Result<Value, BccError> {
        match operator {
            BinaryOp::Add => match (left, right) {
                (Value::Int(l), Value::Int(r)) => Ok(Value::Int(l + r)),
                (Value::Double(l), Value::Double(r)) => Ok(Value::Double(l + r)),
                (Value::Int(l), Value::Double(r)) => Ok(Value::Double(l as f64 + r)),
                (Value::Double(l), Value::Int(r)) => Ok(Value::Double(l + r as f64)),
                (Value::String(l), Value::String(r)) => Ok(Value::String(l + &r)),
                (l, r) => Err(BccError::runtime_error(
                    span.clone(),
                    format!(
                        "Cannot add {} and {}",
                        l.type_name(),
                        r.type_name()
                    ),
                )),
            },
            BinaryOp::Subtract => match (left, right) {
                (Value::Int(l), Value::Int(r)) => Ok(Value::Int(l - r)),
                (Value::Double(l), Value::Double(r)) => Ok(Value::Double(l - r)),
                (Value::Int(l), Value::Double(r)) => Ok(Value::Double(l as f64 - r)),
                (Value::Double(l), Value::Int(r)) => Ok(Value::Double(l - r as f64)),
                (l, r) => Err(BccError::runtime_error(
                    span.clone(),
                    format!(
                        "Cannot subtract {} and {}",
                        l.type_name(),
                        r.type_name()
                    ),
                )),
            },
            BinaryOp::Multiply => match (left, right) {
                (Value::Int(l), Value::Int(r)) => Ok(Value::Int(l * r)),
                (Value::Double(l), Value::Double(r)) => Ok(Value::Double(l * r)),
                (Value::Int(l), Value::Double(r)) => Ok(Value::Double(l as f64 * r)),
                (Value::Double(l), Value::Int(r)) => Ok(Value::Double(l * r as f64)),
                (l, r) => Err(BccError::runtime_error(
                    span.clone(),
                    format!(
                        "Cannot multiply {} and {}",
                        l.type_name(),
                        r.type_name()
                    ),
                )),
            },
            BinaryOp::Divide => match (left, right) {
                (Value::Int(l), Value::Int(r)) => {
                    if r == 0 {
                        Err(BccError::runtime_error(
                            span.clone(),
                            "Division by zero".to_string(),
                        ))
                    } else {
                        Ok(Value::Double(l as f64 / r as f64))
                    }
                }
                (Value::Double(l), Value::Double(r)) => {
                    if r == 0.0 {
                        Err(BccError::runtime_error(
                            span.clone(),
                            "Division by zero".to_string(),
                        ))
                    } else {
                        Ok(Value::Double(l / r))
                    }
                }
                (Value::Int(l), Value::Double(r)) => {
                    if r == 0.0 {
                        Err(BccError::runtime_error(
                            span.clone(),
                            "Division by zero".to_string(),
                        ))
                    } else {
                        Ok(Value::Double(l as f64 / r))
                    }
                }
                (Value::Double(l), Value::Int(r)) => {
                    if r == 0 {
                        Err(BccError::runtime_error(
                            span.clone(),
                            "Division by zero".to_string(),
                        ))
                    } else {
                        Ok(Value::Double(l / r as f64))
                    }
                }
                (l, r) => Err(BccError::runtime_error(
                    span.clone(),
                    format!(
                        "Cannot divide {} and {}",
                        l.type_name(),
                        r.type_name()
                    ),
                )),
            },
            BinaryOp::Equal => Ok(Value::Bool(self.is_equal(&left, &right))),
            BinaryOp::NotEqual => Ok(Value::Bool(!self.is_equal(&left, &right))),
            BinaryOp::Greater => match (left, right) {
                (Value::Int(l), Value::Int(r)) => Ok(Value::Bool(l > r)),
                (Value::Double(l), Value::Double(r)) => Ok(Value::Bool(l > r)),
                (Value::Int(l), Value::Double(r)) => Ok(Value::Bool((l as f64) > r)),
                (Value::Double(l), Value::Int(r)) => Ok(Value::Bool(l > (r as f64))),
                (l, r) => Err(BccError::runtime_error(
                    span.clone(),
                    format!(
                        "Cannot compare {} and {}",
                        l.type_name(),
                        r.type_name()
                    ),
                )),
            },
            BinaryOp::GreaterEqual => match (left, right) {
                (Value::Int(l), Value::Int(r)) => Ok(Value::Bool(l >= r)),
                (Value::Double(l), Value::Double(r)) => Ok(Value::Bool(l >= r)),
                (Value::Int(l), Value::Double(r)) => Ok(Value::Bool((l as f64) >= r)),
                (Value::Double(l), Value::Int(r)) => Ok(Value::Bool(l >= (r as f64))),
                (l, r) => Err(BccError::runtime_error(
                    span.clone(),
                    format!(
                        "Cannot compare {} and {}",
                        l.type_name(),
                        r.type_name()
                    ),
                )),
            },
            BinaryOp::Less => match (left, right) {
                (Value::Int(l), Value::Int(r)) => Ok(Value::Bool(l < r)),
                (Value::Double(l), Value::Double(r)) => Ok(Value::Bool(l < r)),
                (Value::Int(l), Value::Double(r)) => Ok(Value::Bool((l as f64) < r)),
                (Value::Double(l), Value::Int(r)) => Ok(Value::Bool(l < (r as f64))),
                (l, r) => Err(BccError::runtime_error(
                    span.clone(),
                    format!(
                        "Cannot compare {} and {}",
                        l.type_name(),
                        r.type_name()
                    ),
                )),
            },
            BinaryOp::LessEqual => match (left, right) {
                (Value::Int(l), Value::Int(r)) => Ok(Value::Bool(l <= r)),
                (Value::Double(l), Value::Double(r)) => Ok(Value::Bool(l <= r)),
                (Value::Int(l), Value::Double(r)) => Ok(Value::Bool((l as f64) <= r)),
                (Value::Double(l), Value::Int(r)) => Ok(Value::Bool(l <= (r as f64))),
                (l, r) => Err(BccError::runtime_error(
                    span.clone(),
                    format!(
                        "Cannot compare {} and {}",
                        l.type_name(),
                        r.type_name()
                    ),
                )),
            },
            BinaryOp::In => {
                self.evaluate_in_operation(left, right, span)
            },
        }
    }

    fn evaluate_unary_op(
        &self,
        operator: &UnaryOp,
        operand: Value,
        span: &Span,
    ) -> Result<Value, BccError> {
        match operator {
            UnaryOp::Negate => match operand {
                Value::Int(n) => Ok(Value::Int(-n)),
                Value::Double(n) => Ok(Value::Double(-n)),
                _ => Err(BccError::runtime_error(
                    span.clone(),
                    format!("Cannot negate {}", operand.type_name()),
                )),
            },
            UnaryOp::Not => Ok(Value::Bool(!operand.is_truthy())),
        }
    }

    fn evaluate_in_operation(
        &self,
        left: Value,
        right: Value,
        span: &Span,
    ) -> Result<Value, BccError> {
        match right {
            Value::List(list) => {
                // Check if left value is in the list
                for item in &list {
                    if self.is_equal(&left, item) {
                        return Ok(Value::Bool(true));
                    }
                }
                Ok(Value::Bool(false))
            },
            Value::Dict(dict) => {
                // Check if left value is a key in the dictionary
                match left {
                    Value::String(key) => {
                        Ok(Value::Bool(dict.contains_key(&key)))
                    },
                    _ => Err(BccError::runtime_error_with_help(
                        span.clone(),
                        format!("Dictionary key lookup requires a string, got {}", left.type_name()),
                        "Use 'in' with dictionaries like: \"key\" in {\"key\": \"value\"}. Only string keys are supported.".to_string(),
                    ))
                }
            },
            Value::String(string) => {
                // Check if left value is a substring of the string
                match left {
                    Value::String(substring) => {
                        Ok(Value::Bool(string.contains(&substring)))
                    },
                    _ => Err(BccError::runtime_error_with_help(
                        span.clone(),
                        format!("String containment check requires a string, got {}", left.type_name()),
                        "Use 'in' with strings like: \"sub\" in \"substring\". Both values must be strings.".to_string(),
                    ))
                }
            },
            Value::Tuple(tuple) => {
                // Check if left value is in the tuple
                for item in &tuple {
                    if self.is_equal(&left, item) {
                        return Ok(Value::Bool(true));
                    }
                }
                Ok(Value::Bool(false))
            },
            _ => Err(BccError::runtime_error_with_help(
                span.clone(),
                format!("'in' operator not supported for type {}", right.type_name()),
                "The 'in' operator works with lists, tuples, dictionaries, and strings. Examples: item in [1, 2, 3], item in (1, 2, 3), \"key\" in {\"key\": \"value\"}, \"sub\" in \"substring\".".to_string(),
            ))
        }
    }

    fn is_equal(&self, left: &Value, right: &Value) -> bool {
        match (left, right) {
            (Value::Nil, Value::Nil) => true,
            (Value::Bool(l), Value::Bool(r)) => l == r,
            (Value::Int(l), Value::Int(r)) => l == r,
            (Value::Double(l), Value::Double(r)) => l == r,
            (Value::Int(l), Value::Double(r)) => (*l as f64) == *r,
            (Value::Double(l), Value::Int(r)) => *l == (*r as f64),
            (Value::String(l), Value::String(r)) => l == r,
            (Value::List(l), Value::List(r)) => {
                if l.len() != r.len() {
                    false
                } else {
                    l.iter().zip(r.iter()).all(|(a, b)| self.is_equal(a, b))
                }
            },
            (Value::Tuple(l), Value::Tuple(r)) => {
                if l.len() != r.len() {
                    false
                } else {
                    l.iter().zip(r.iter()).all(|(a, b)| self.is_equal(a, b))
                }
            },
            // Note: We don't implement Dict equality here as it's complex and not essential for basic functionality
            _ => false,
        }
    }
}