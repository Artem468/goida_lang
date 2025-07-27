use crate::ast::{BinaryOperator, Expression, Function, Program, Statement, UnaryOperator};
use crate::interpreter::prelude::InterpreterStructs::{
    Environment, Interpreter, Module, RuntimeError, Value,
};
use crate::lexer::structs::Lexer;
use crate::parser::structs::Parser;
use std::collections::HashMap;
use std::io;
use std::io::Write;

impl Interpreter {
    pub fn new(dir: std::path::PathBuf) -> Self {
        Interpreter {
            environment: Environment::new(),
            functions: HashMap::new(),
            modules: HashMap::new(),
            current_dir: dir,
        }
    }

    pub fn interpret(&mut self, program: Program) -> Result<(), RuntimeError> {
        for import in program.imports {
            for path in import.files {
                let relative_path = std::path::Path::new(&path);
                let full_path = self.current_dir.join(relative_path).with_extension("goida");
                let file_stem =
                    full_path
                        .file_stem()
                        .and_then(|s| s.to_str())
                        .ok_or_else(|| {
                            RuntimeError::InvalidOperation(format!(
                                "Невозможно получить имя модуля из пути: {}",
                                full_path.display()
                            ))
                        })?;
                let code = std::fs::read_to_string(&full_path).map_err(|err| {
                    RuntimeError::IOError(format!("{} | {err}", full_path.display()))
                })?;
                let tokens = Lexer::new(code).tokenize();
                let program = Parser::new(tokens).parse().map_err(|err| {
                    RuntimeError::ParseError(format!(
                        "Ошибка парсинга модуля {}: {err:?}",
                        file_stem
                    ))
                })?;

                let mut sub_interpreter = Interpreter::new(
                    full_path
                        .parent()
                        .unwrap_or(&self.current_dir)
                        .to_path_buf(),
                );

                sub_interpreter.interpret(program)?;

                let mut namespaced_functions = HashMap::new();
                for (name, mut func) in sub_interpreter.functions {
                    let qualified = format!("{}", name);
                    func.module = Some(name);
                    namespaced_functions.insert(qualified, func);
                }

                let mut namespaced_variables = HashMap::new();
                for (name, value) in sub_interpreter.environment.variables {
                    let qualified = format!("{}", name);
                    namespaced_variables.insert(qualified, value);
                }

                let module = Module {
                    functions: namespaced_functions.clone(),
                    environment: Environment {
                        variables: namespaced_variables.clone(),
                        parent: None,
                    },
                };

                self.modules.insert(file_stem.to_string(), module);
            }
        }

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

            Statement::Input(expr) => {
                let value = self.evaluate_expression(expr)?;
                self.input_function(value)?;
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

            Expression::Identifier(name) => {
                if let Some(dot_pos) = name.find('.') {
                    let module_name = &name[..dot_pos];
                    let var_name = &name[dot_pos + 1..];

                    if let Some(module_env) = self.modules.get(module_name) {
                        module_env.environment.get(var_name).ok_or_else(|| {
                            RuntimeError::UndefinedVariable(format!("{}.{}", module_name, var_name))
                        })
                    } else {
                        Err(RuntimeError::InvalidOperation(format!(
                            "Модуль '{}' не найден",
                            module_name
                        )))
                    }
                } else {
                    self.environment
                        .get(&name)
                        .ok_or_else(|| RuntimeError::UndefinedVariable(name.clone()))
                }
            }

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
                if let Some(dot_index) = name.find('.') {
                    let module_name = &name[..dot_index];
                    let func_name = &name[dot_index + 1..];

                    return if let Some(module) = self.modules.get(module_name) {
                        if let Some(function) = module.functions.get(func_name) {
                            self.call_function(function.clone(), arguments.clone())
                        } else {
                            Err(RuntimeError::UndefinedFunction(format!(
                                "{}.{}",
                                module_name, func_name
                            )))
                        }
                    } else {
                        Err(RuntimeError::InvalidOperation(format!(
                            "Модуль {} не найден",
                            module_name
                        )))
                    };
                }

                if let Some(function) = self.functions.get(name).cloned() {
                    return self.call_function(function, arguments.clone());
                }
                for (_module_name, module) in &self.modules {
                    if let Some(function) = module.functions.get(name) {
                        return self.call_function(function.clone(), arguments.clone());
                    }
                }
                Err(RuntimeError::UndefinedFunction(name.clone()))
            }

            Expression::Input { argument } => {
                let data = self.evaluate_expression(argument)?;
                self.input_function(data)
            }

            Expression::AccessIndex {
                object: _,
                index: _,
            } => Err(RuntimeError::InvalidOperation(
                "Индексный доступ еще не реализован".to_string(),
            )),
        }
    }

    fn call_function(
        &mut self,
        function: Function,
        arguments: Vec<Expression>,
    ) -> Result<Value, RuntimeError> {
        if arguments.len() != function.parameters.len() {
            return Err(RuntimeError::InvalidOperation(format!(
                "Функция {} ожидает {} аргументов, получено {}",
                function.name,
                function.parameters.len(),
                arguments.len()
            )));
        }

        // Новый scope
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
                Err(RuntimeError::Return(val)) => {
                    result = val;
                    break;
                }
                Err(e) => {
                    self.environment = self.environment.clone().pop();
                    return Err(e);
                }
            }
        }

        self.environment = self.environment.clone().pop();
        Ok(result)
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
