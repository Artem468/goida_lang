use crate::ast::*;
use std::collections::HashMap;
use std::fmt;
use std::io::{self, Write};

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Number(i64),
    Text(String),
    Boolean(bool),
    Empty,
}

impl Value {
    fn to_string(&self) -> String {
        match self {
            Value::Number(n) => n.to_string(),
            Value::Text(s) => s.clone(),
            Value::Boolean(b) => {
                if *b {
                    "истина".to_string()
                } else {
                    "ложь".to_string()
                }
            }
            Value::Empty => "пустота".to_string(),
        }
    }

    fn is_truthy(&self) -> bool {
        match self {
            Value::Boolean(b) => *b,
            Value::Number(n) => *n != 0,
            Value::Text(s) => !s.is_empty(),
            Value::Empty => false,
        }
    }
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Value::Number(data) => {write!(f, "{data}")}
            Value::Text(data) => {write!(f, "{data}")}
            Value::Boolean(data) => {write!(f, "{data}")}
            Value::Empty => {write!(f, "")}
        }
    }
}

#[derive(Debug)]
pub enum RuntimeError {
    UndefinedVariable(String),
    UndefinedFunction(String),
    TypeMismatch(String),
    DivisionByZero,
    InvalidOperation(String),
    Return(Value),
}

#[derive(Clone)]
pub struct Environment {
    variables: HashMap<String, Value>,
    parent: Option<Box<Environment>>,
}

impl Environment {
    fn new() -> Self {
        Environment {
            variables: HashMap::new(),
            parent: None,
        }
    }

    fn with_parent(parent: Environment) -> Self {
        Environment {
            variables: HashMap::new(),
            parent: Some(Box::new(parent)),
        }
    }

    fn define(&mut self, name: String, value: Value) {
        self.variables.insert(name, value);
    }

    fn get(&self, name: &str) -> Option<Value> {
        if let Some(value) = self.variables.get(name) {
            Some(value.clone())
        } else if let Some(parent) = &self.parent {
            parent.get(name)
        } else {
            None
        }
    }

    fn set(&mut self, name: &str, value: Value) -> Result<(), RuntimeError> {
        if self.variables.contains_key(name) {
            self.variables.insert(name.to_string(), value);
            Ok(())
        } else if let Some(parent) = &mut self.parent {
            parent.set(name, value)
        } else {
            // Если переменная не найдена, создаем ее в текущей области
            self.variables.insert(name.to_string(), value);
            Ok(())
        }
    }
}

pub struct Interpreter {
    environment: Environment,
    functions: HashMap<String, Function>,
}

impl Interpreter {
    pub fn new() -> Self {
        Interpreter {
            environment: Environment::new(),
            functions: HashMap::new(),
        }
    }

    pub fn interpret(&mut self, program: Program) -> Result<(), RuntimeError> {
        for function in program.functions {
            self.functions.insert(function.name.clone(), function);
        }

        for statement in program.operators {
            match self.execute_statement(&statement) {
                Err(RuntimeError::Return(_)) => {}
                Err(e) => return Err(e),
                Ok(()) => {}
            }
        }

        Ok(())
    }

    fn execute_statement(&mut self, statement: &Statement) -> Result<(), RuntimeError> {
        match statement {
            Statement::Declaration {
                name,
                type_of: _,
                value,
            } => {
                let value = if let Some(expr) = value {
                    self.evaluate_expression(expr)?
                } else {
                    Value::Empty
                };
                self.environment.define(name.clone(), value);
                Ok(())
            }

            Statement::Assignment { name, value } => {
                let _value = self.evaluate_expression(value)?;
                self.environment.set(name, _value)?;
                Ok(())
            }

            Statement::If {
                condition,
                body,
                another,
            } => {
                let condition_value = self.evaluate_expression(condition)?;
                if condition_value.is_truthy() {
                    for stmt in body {
                        self.execute_statement(stmt)?;
                    }
                } else if let Some(else_body) = another {
                    for stmt in else_body {
                        self.execute_statement(stmt)?;
                    }
                }
                Ok(())
            }

            Statement::While { condition, body } => {
                while self.evaluate_expression(condition)?.is_truthy() {
                    for stmt in body {
                        self.execute_statement(stmt)?;
                    }
                }
                Ok(())
            }

            Statement::For {
                variable,
                start,
                end,
                body,
            } => {
                let start_val = self.evaluate_expression(start)?;
                let end_val = self.evaluate_expression(end)?;

                let (start, end) = match (start_val, end_val) {
                    (Value::Number(s), Value::Number(e)) => (s, e),
                    _ => {
                        return Err(RuntimeError::TypeMismatch(
                            "Цикл 'для' требует числовые значения".to_string(),
                        ))
                    }
                };

                let parent_env = self.environment.clone();
                self.environment = Environment::with_parent(parent_env);

                for i in start..=end {
                    self.environment.define(variable.clone(), Value::Number(i));
                    for stmt in body {
                        self.execute_statement(stmt)?;
                    }
                }

                if let Some(parent) = self.environment.parent.take() {
                    self.environment = *parent;
                }
                Ok(())
            }

            Statement::Return(expr) => {
                let value = if let Some(e) = expr {
                    self.evaluate_expression(e)?
                } else {
                    Value::Empty
                };
                Err(RuntimeError::Return(value))
            }

            Statement::Expression(expr) => {
                self.evaluate_expression(expr)?;
                Ok(())
            }

            Statement::Print(expr) => {
                let value = self.evaluate_expression(expr)?;
                println!("{}", value.to_string());
                Ok(())
            }

            Statement::Block(statements) => {
                let parent_env = self.environment.clone();
                self.environment = Environment::with_parent(parent_env);

                for stmt in statements {
                    self.execute_statement(stmt)?;
                }

                if let Some(parent) = self.environment.parent.take() {
                    self.environment = *parent;
                }
                Ok(())
            }
        }
    }

    fn evaluate_expression(&mut self, expression: &Expression) -> Result<Value, RuntimeError> {
        match expression {
            Expression::Number(n) => Ok(Value::Number(*n)),
            Expression::Text(s) => Ok(Value::Text(s.clone())),
            Expression::Boolean(b) => Ok(Value::Boolean(*b)),

            Expression::Identifier(name) => self
                .environment
                .get(name)
                .ok_or_else(|| RuntimeError::UndefinedVariable(name.clone())),

            Expression::BinaryOperation {
                left,
                operator,
                right,
            } => {
                let left_val = self.evaluate_expression(left)?;
                let right_val = self.evaluate_expression(right)?;

                match operator {
                    BinaryOperator::Plus => self.add_values(left_val, right_val),
                    BinaryOperator::Minus => self.subtract_values(left_val, right_val),
                    BinaryOperator::Multiply => self.multiply_values(left_val, right_val),
                    BinaryOperator::Divide => self.divide_values(left_val, right_val),
                    BinaryOperator::Remainder => self.modulo_values(left_val, right_val),
                    BinaryOperator::Equal => Ok(Value::Boolean(left_val == right_val)),
                    BinaryOperator::Unequal => Ok(Value::Boolean(left_val != right_val)),
                    BinaryOperator::More => self.compare_greater(left_val, right_val),
                    BinaryOperator::Less => self.compare_less(left_val, right_val),
                    BinaryOperator::MoreEqual => self.compare_greater_equal(left_val, right_val),
                    BinaryOperator::LessEqual => self.compare_less_equal(left_val, right_val),
                    BinaryOperator::And => Ok(Value::Boolean(
                        left_val.is_truthy() && right_val.is_truthy(),
                    )),
                    BinaryOperator::Or => Ok(Value::Boolean(
                        left_val.is_truthy() || right_val.is_truthy(),
                    )),
                }
            }

            Expression::UnaryOperation { operator, operand } => {
                let value = self.evaluate_expression(operand)?;

                match operator {
                    UnaryOperator::Negative => match value {
                        Value::Number(n) => Ok(Value::Number(-n)),
                        _ => Err(RuntimeError::TypeMismatch(
                            "Унарный минус применим только к числам".to_string(),
                        )),
                    },
                    UnaryOperator::Not => Ok(Value::Boolean(!value.is_truthy())),
                }
            }

            Expression::CallingFunction { name, arguments } => {
                if name == "ввод" {
                    if arguments.len() != 1 {
                        return Err(RuntimeError::InvalidOperation(format!(
                            "Функция {} ожидает 1 аргумент, получено {}",
                            name,
                            arguments.len()
                        )));
                    }
                    let arg_value = self.evaluate_expression(&arguments[0])?;
                    return self.input_function(arg_value);
                }

                if let Some(function) = self.functions.get(name).cloned() {
                    if arguments.len() != function.parameters.len() {
                        return Err(RuntimeError::InvalidOperation(format!(
                            "Функция {} ожидает {} аргументов, получено {}",
                            name,
                            function.parameters.len(),
                            arguments.len()
                        )));
                    }

                    let parent_env = self.environment.clone();
                    self.environment = Environment::with_parent(parent_env);

                    for (param, arg_expr) in function.parameters.iter().zip(arguments.iter()) {
                        let arg_value = self.evaluate_expression(arg_expr)?;
                        self.environment.define(param.name.clone(), arg_value);
                    }

                    let mut result = Value::Empty;
                    for stmt in &function.body {
                        match self.execute_statement(stmt) {
                            Ok(()) => {}
                            Err(RuntimeError::Return(value)) => {
                                result = value;
                                break;
                            }
                            Err(e) => {
                                if let Some(parent) = self.environment.parent.take() {
                                    self.environment = *parent;
                                }
                                return Err(e);
                            }
                        }
                    }

                    if let Some(parent) = self.environment.parent.take() {
                        self.environment = *parent;
                    }
                    Ok(result)
                } else {
                    Err(RuntimeError::UndefinedFunction(name.clone()))
                }
            }

            Expression::AccessIndex {
                object: _,
                index: _,
            } => Err(RuntimeError::InvalidOperation(
                "Индексный доступ еще не реализован".to_string(),
            )),
        }
    }

    fn input_function(&self, argument: Value) -> Result<Value, RuntimeError> {
        print!("{}", argument);
        io::stdout().flush().unwrap();

        let mut input = String::new();
        io::stdin().read_line(&mut input).unwrap();
        let input = input.trim();

        if let Ok(num) = input.parse::<i64>() {
            Ok(Value::Number(num))
        } else {
            Ok(Value::Text(input.to_string()))
        }
    }

    fn add_values(&self, left: Value, right: Value) -> Result<Value, RuntimeError> {
        match (left, right) {
            (Value::Number(a), Value::Number(b)) => Ok(Value::Number(a + b)),
            (Value::Text(a), Value::Text(b)) => Ok(Value::Text(format!("{}{}", a, b))),
            (Value::Text(a), Value::Number(b)) => Ok(Value::Text(format!("{}{}", a, b))),
            (Value::Number(a), Value::Text(b)) => Ok(Value::Text(format!("{}{}", a, b))),
            (Value::Text(a), Value::Boolean(b)) => Ok(Value::Text(format!(
                "{}{}",
                a,
                if b { "истина" } else { "ложь" }
            ))),
            (Value::Boolean(a), Value::Text(b)) => Ok(Value::Text(format!(
                "{}{}",
                if a { "истина" } else { "ложь" },
                b
            ))),
            _ => Err(RuntimeError::TypeMismatch(
                "Неподдерживаемые типы для операции сложения".to_string(),
            )),
        }
    }

    fn subtract_values(&self, left: Value, right: Value) -> Result<Value, RuntimeError> {
        match (left, right) {
            (Value::Number(a), Value::Number(b)) => Ok(Value::Number(a - b)),
            _ => Err(RuntimeError::TypeMismatch(
                "Вычитание применимо только к числам".to_string(),
            )),
        }
    }

    fn multiply_values(&self, left: Value, right: Value) -> Result<Value, RuntimeError> {
        match (left, right) {
            (Value::Number(a), Value::Number(b)) => Ok(Value::Number(a * b)),
            _ => Err(RuntimeError::TypeMismatch(
                "Умножение применимо только к числам".to_string(),
            )),
        }
    }

    fn divide_values(&self, left: Value, right: Value) -> Result<Value, RuntimeError> {
        match (left, right) {
            (Value::Number(a), Value::Number(b)) => {
                if b == 0 {
                    Err(RuntimeError::DivisionByZero)
                } else {
                    Ok(Value::Number(a / b))
                }
            }
            _ => Err(RuntimeError::TypeMismatch(
                "Деление применимо только к числам".to_string(),
            )),
        }
    }

    fn modulo_values(&self, left: Value, right: Value) -> Result<Value, RuntimeError> {
        match (left, right) {
            (Value::Number(a), Value::Number(b)) => {
                if b == 0 {
                    Err(RuntimeError::DivisionByZero)
                } else {
                    Ok(Value::Number(a % b))
                }
            }
            _ => Err(RuntimeError::TypeMismatch(
                "Остаток от деления применим только к числам".to_string(),
            )),
        }
    }

    fn compare_greater(&self, left: Value, right: Value) -> Result<Value, RuntimeError> {
        match (left, right) {
            (Value::Number(a), Value::Number(b)) => Ok(Value::Boolean(a > b)),
            _ => Err(RuntimeError::TypeMismatch(
                "Сравнение применимо только к числам".to_string(),
            )),
        }
    }

    fn compare_less(&self, left: Value, right: Value) -> Result<Value, RuntimeError> {
        match (left, right) {
            (Value::Number(a), Value::Number(b)) => Ok(Value::Boolean(a < b)),
            _ => Err(RuntimeError::TypeMismatch(
                "Сравнение применимо только к числам".to_string(),
            )),
        }
    }

    fn compare_greater_equal(&self, left: Value, right: Value) -> Result<Value, RuntimeError> {
        match (left, right) {
            (Value::Number(a), Value::Number(b)) => Ok(Value::Boolean(a >= b)),
            _ => Err(RuntimeError::TypeMismatch(
                "Сравнение применимо только к числам".to_string(),
            )),
        }
    }

    fn compare_less_equal(&self, left: Value, right: Value) -> Result<Value, RuntimeError> {
        match (left, right) {
            (Value::Number(a), Value::Number(b)) => Ok(Value::Boolean(a <= b)),
            _ => Err(RuntimeError::TypeMismatch(
                "Сравнение применимо только к числам".to_string(),
            )),
        }
    }
}
