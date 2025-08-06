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
            current_module: None,
        }
    }

    fn into_module(self) -> Module {
        Module {
            functions: self.functions,
            environment: self.environment,
        }
    }

    pub fn interpret(&mut self, program: Program) -> Result<(), RuntimeError> {
        self.current_module = Some(program.name.clone());

        for import in program.imports {
            for path in import.files {
                let relative_path = std::path::Path::new(&path);
                let full_path = self.current_dir.join(relative_path).with_extension("goida");
                let file_stem = full_path.file_stem().and_then(|s| s.to_str()).ok_or_else(|| {
                    RuntimeError::InvalidOperation(format!(
                        "Невозможно получить имя модуля из пути: {}",
                        full_path.display()
                    ))
                })?;
                let code = std::fs::read_to_string(&full_path).map_err(|err| {
                    RuntimeError::IOError(format!("{} | {err}", full_path.display()))
                })?;

                let tokens = Lexer::new(code).tokenize();
                let program = Parser::new(tokens, file_stem.to_string()).parse().map_err(|err| {
                    RuntimeError::ParseError(format!(
                        "Ошибка парсинга модуля {}: {err:?}",
                        file_stem
                    ))
                })?;

                let mut sub_interpreter = Interpreter::new(
                    full_path.parent().unwrap_or(&self.current_dir).to_path_buf(),
                );
                sub_interpreter.interpret(program)?;
                self.modules.insert(file_stem.to_string(), sub_interpreter.into_module());
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

            Statement::IndexAssignment { object, index, value } => {
                self.execute_index_assignment(object, index, value)
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
            Expression::List(elements) => {
                let mut list_values = Vec::new();
                for element in elements {
                    list_values.push(self.evaluate_expression(element)?);
                }
                Ok(Value::List(list_values))
            }
            Expression::Dict(pairs) => {
                let mut dict_map = std::collections::HashMap::new();
                for (key_expr, value_expr) in pairs {
                    let key = self.evaluate_expression(key_expr)?;
                    let value = self.evaluate_expression(value_expr)?;
                    let key_str = match key {
                        Value::Text(s) => s,
                        Value::Number(n) => n.to_string(),
                        Value::Boolean(b) => (if b { "истина" } else { "ложь" }).to_string(),
                        _ => return Err(RuntimeError::TypeMismatch(
                            "Ключи словаря должны быть текстом, числом или логическим значением".to_string(),
                        )),
                    };
                    dict_map.insert(key_str, value);
                }
                Ok(Value::Dict(dict_map))
            }

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
                if let Ok(result) = self.call_builtin_function(name, arguments) {
                    return Ok(result);
                }
                
                if let Some(dot_index) = name.find('.') {
                    let module_name = &name[..dot_index];
                    let func_name = &name[dot_index + 1..];
                    if let Some(module) = self.modules.get(module_name) {
                        if let Some(function) = module.functions.get(func_name) {
                            return self.call_function(function.clone(), arguments.clone());
                        }
                    }
                    Err(RuntimeError::UndefinedFunction(name.clone()))
                } else {
                    if let Some(function) = self.functions.get(name).cloned() {
                        self.call_function(function, arguments.clone())
                    } else if let Some(module_name) = &self.current_module {
                        if let Some(module) = self.modules.get(module_name) {
                            if let Some(function) = module.functions.get(name) {
                                return self.call_function(function.clone(), arguments.clone());
                            }
                        }
                        Err(RuntimeError::UndefinedFunction(name.clone()))
                    } else {
                        Err(RuntimeError::UndefinedFunction(name.clone()))
                    }
                }
            }

            Expression::Input { argument } => {
                let data = self.evaluate_expression(argument)?;
                self.input_function(data)
            }

            Expression::AccessIndex { object, index } => {
                let obj_value = self.evaluate_expression(object)?;
                let idx_value = self.evaluate_expression(index)?;
                
                match obj_value {
                    Value::List(items) => {
                        match idx_value {
                            Value::Number(n) => {
                                let index = n as usize;
                                if index < items.len() {
                                    Ok(items[index].clone())
                                } else {
                                    Err(RuntimeError::InvalidOperation(format!(
                                        "Индекс {} выходит за границы списка длины {}",
                                        index, items.len()
                                    )))
                                }
                            },
                            _ => Err(RuntimeError::TypeMismatch(
                                "Индекс списка должен быть числом".to_string(),
                            ))
                        }
                    },
                    Value::Dict(map) => {
                        let key_str = match idx_value {
                            Value::Text(s) => s,
                            Value::Number(n) => n.to_string(),
                            Value::Boolean(b) => (if b { "истина" } else { "ложь" }).to_string(),
                            _ => return Err(RuntimeError::TypeMismatch(
                                "Ключ словаря должен быть текстом, числом или логическим значением".to_string(),
                            )),
                        };
                        
                        map.get(&key_str).cloned().ok_or_else(|| {
                            RuntimeError::InvalidOperation(format!(
                                "Ключ '{}' не найден в словаре",
                                key_str
                            ))
                        })
                    },
                    _ => Err(RuntimeError::TypeMismatch(
                        "Индексный доступ поддерживается только для списков и словарей".to_string(),
                    ))
                }
            }
        }
    }

    fn call_function(
        &mut self,
        function: Function,
        arguments: Vec<Expression>,
    ) -> Result<Value, RuntimeError> {
        let prev_module = self.current_module.clone();
        self.current_module = function.module.clone();
        let parent_env = self.environment.clone();
        self.environment = Environment::with_parent(parent_env);
        
        
        if arguments.len() != function.parameters.len() {
            return Err(RuntimeError::InvalidOperation(format!(
                "Функция {} ожидает {} аргументов, получено {}",
                function.name,
                function.parameters.len(),
                arguments.len()
            )));
        }
        
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
        self.current_module = prev_module;
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
        match (&left, &right) {
            (Value::Number(a), Value::Number(b)) => Ok(Value::Number(a + b)),
            (Value::Text(a), Value::Text(b)) => Ok(Value::Text(format!("{}{}", a, b))),
            (Value::Text(a), Value::Number(b)) => Ok(Value::Text(format!("{}{}", a, b))),
            (Value::Number(a), Value::Text(b)) => Ok(Value::Text(format!("{}{}", a, b))),
            (Value::Text(a), Value::Boolean(b)) => Ok(Value::Text(format!(
                "{}{}",
                a,
                if *b { "истина" } else { "ложь" }
            ))),
            (Value::Boolean(a), Value::Text(b)) => Ok(Value::Text(format!(
                "{}{}",
                if *a { "истина" } else { "ложь" },
                b
            ))),
            (Value::Text(a), Value::List(_)) => Ok(Value::Text(format!("{}{}", a, right))),
            (Value::List(_), Value::Text(b)) => Ok(Value::Text(format!("{}{}", left, b))),
            (Value::Text(a), Value::Dict(_)) => Ok(Value::Text(format!("{}{}", a, right))),
            (Value::Dict(_), Value::Text(d)) => Ok(Value::Text(format!("{}{}", left, d))),
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

    fn call_builtin_function(
        &mut self,
        name: &str,
        arguments: &[Expression],
    ) -> Result<Value, RuntimeError> {
        match name {
            "добавить" => self.builtin_push(arguments),
            "извлечь" => self.builtin_pop(arguments),
            "удалить" => self.builtin_remove(arguments),
            "размер" => self.builtin_size(arguments),
            "содержит" => self.builtin_contains(arguments),
            _ => Err(RuntimeError::UndefinedFunction(name.to_string())),
        }
    }

    fn builtin_push(&mut self, arguments: &[Expression]) -> Result<Value, RuntimeError> {
        if arguments.len() != 2 && arguments.len() != 3 {
            return Err(RuntimeError::InvalidOperation(
                "Функция 'добавить' ожидает 2 аргумента для списков (список, элемент) или 3 для словарей (словарь, ключ, значение)".to_string(),
            ));
        }

        let mut container = self.evaluate_expression(&arguments[0])?;

        match &mut container {
            Value::List(ref mut items) => {
                if arguments.len() != 2 {
                    return Err(RuntimeError::InvalidOperation(
                        "Для списка функция 'добавить' ожидает 2 аргумента: (список, элемент)".to_string(),
                    ));
                }
                let element = self.evaluate_expression(&arguments[1])?;
                items.push(element);
                
                if let Expression::Identifier(name) = &arguments[0] {
                    let _ = self.environment.set(name.as_str(), container.clone());
                }
                
                Ok(Value::Empty)
            }
            Value::Dict(ref mut map) => {
                if arguments.len() != 3 {
                    return Err(RuntimeError::InvalidOperation(
                        "Для словаря функция 'добавить' ожидает 3 аргумента: (словарь, ключ, значение)".to_string(),
                    ));
                }
                let key = self.evaluate_expression(&arguments[1])?;
                let value = self.evaluate_expression(&arguments[2])?;
                
                let key_str = match key {
                    Value::Text(s) => s,
                    Value::Number(n) => n.to_string(),
                    Value::Boolean(b) => (if b { "истина" } else { "ложь" }).to_string(),
                    _ => return Err(RuntimeError::TypeMismatch(
                        "Ключ словаря должен быть текстом, числом или логическим значением".to_string(),
                    )),
                };
                
                map.insert(key_str, value);
                
                if let Expression::Identifier(name) = &arguments[0] {
                    let _ = self.environment.set(name.as_str(), container.clone());
                }
                
                Ok(Value::Empty)
            }
            _ => Err(RuntimeError::TypeMismatch(
                "Функция 'добавить' применима только к спискам и словарям".to_string(),
            )),
        }
    }

    fn builtin_pop(&mut self, arguments: &[Expression]) -> Result<Value, RuntimeError> {
        if arguments.len() != 1 {
            return Err(RuntimeError::InvalidOperation(
                "Функция 'извлечь' ожидает 1 аргумент: список".to_string(),
            ));
        }

        let mut container = self.evaluate_expression(&arguments[0])?;

        match &mut container {
            Value::List(ref mut items) => {
                if items.is_empty() {
                    Err(RuntimeError::InvalidOperation(
                        "Нельзя извлечь элемент из пустого списка".to_string(),
                    ))
                } else {
                    Ok(items.pop().unwrap())
                }
            }
            _ => Err(RuntimeError::TypeMismatch(
                "Функция 'извлечь' применима только к спискам".to_string(),
            )),
        }
    }

    fn builtin_remove(&mut self, arguments: &[Expression]) -> Result<Value, RuntimeError> {
        if arguments.len() != 2 {
            return Err(RuntimeError::InvalidOperation(
                "Функция 'удалить' ожидает 2 аргумента: (список/словарь, индекс/ключ)".to_string(),
            ));
        }

        let mut container = self.evaluate_expression(&arguments[0])?;
        let key_or_index = self.evaluate_expression(&arguments[1])?;

        match &mut container {
            Value::List(ref mut items) => {
                match key_or_index {
                    Value::Number(n) => {
                        let index = n as usize;
                        if index < items.len() {
                            Ok(items.remove(index))
                        } else {
                            Err(RuntimeError::InvalidOperation(format!(
                                "Индекс {} выходит за границы списка длины {}",
                                index, items.len()
                            )))
                        }
                    }
                    _ => Err(RuntimeError::TypeMismatch(
                        "Индекс списка должен быть числом".to_string(),
                    ))
                }
            }
            Value::Dict(ref mut map) => {
                let key_str = match key_or_index {
                    Value::Text(s) => s,
                    Value::Number(n) => n.to_string(),
                    Value::Boolean(b) => (if b { "истина" } else { "ложь" }).to_string(),
                    _ => return Err(RuntimeError::TypeMismatch(
                        "Ключ словаря должен быть текстом, числом или логическим значением".to_string(),
                    )),
                };

                map.remove(&key_str).ok_or_else(|| {
                    RuntimeError::InvalidOperation(format!(
                        "Ключ '{}' не найден в словаре",
                        key_str
                    ))
                })
            }
            _ => Err(RuntimeError::TypeMismatch(
                "Функция 'удалить' применима только к спискам и словарям".to_string(),
            )),
        }
    }

    fn builtin_size(&mut self, arguments: &[Expression]) -> Result<Value, RuntimeError> {
        if arguments.len() != 1 {
            return Err(RuntimeError::InvalidOperation(
                "Функция 'размер' ожидает 1 аргумент: список или словарь".to_string(),
            ));
        }

        let container = self.evaluate_expression(&arguments[0])?;

        match container {
            Value::List(items) => Ok(Value::Number(items.len() as i64)),
            Value::Dict(map) => Ok(Value::Number(map.len() as i64)),
            Value::Text(s) => Ok(Value::Number(s.len() as i64)),
            _ => Err(RuntimeError::TypeMismatch(
                "Функция 'размер' применима к спискам, словарям и строкам".to_string(),
            )),
        }
    }

    fn builtin_contains(&mut self, arguments: &[Expression]) -> Result<Value, RuntimeError> {
        if arguments.len() != 2 {
            return Err(RuntimeError::InvalidOperation(
                "Функция 'содержит' ожидает 2 аргумента: (список/словарь, элемент/ключ)".to_string(),
            ));
        }

        let container = self.evaluate_expression(&arguments[0])?;
        let element = self.evaluate_expression(&arguments[1])?;

        match container {
            Value::List(items) => Ok(Value::Boolean(items.contains(&element))),
            Value::Dict(map) => {
                let key_str = match element {
                    Value::Text(s) => s,
                    Value::Number(n) => n.to_string(),
                    Value::Boolean(b) => (if b { "истина" } else { "ложь" }).to_string(),
                    _ => return Err(RuntimeError::TypeMismatch(
                        "Ключ словаря должен быть текстом, числом или логическим значением".to_string(),
                    )),
                };
                Ok(Value::Boolean(map.contains_key(&key_str)))
            }
            _ => Err(RuntimeError::TypeMismatch(
                "Функция 'содержит' применима только к спискам и словарям".to_string(),
            )),
        }
    }

    fn execute_index_assignment(
        &mut self,
        object_expr: &Expression,
        index_expr: &Expression,
        value_expr: &Expression,
    ) -> Result<(), RuntimeError> {
        let new_value = self.evaluate_expression(value_expr)?;
        let index_value = self.evaluate_expression(index_expr)?;
        
        let variable_name = match object_expr {
            Expression::Identifier(name) => name.clone(),
            _ => return Err(RuntimeError::InvalidOperation(
                "Присваивание по индексу возможно только для переменных".to_string(),
            )),
        };
        
        let mut container = self.environment.get(&variable_name)
            .ok_or_else(|| RuntimeError::UndefinedVariable(variable_name.clone()))?;
        
        match &mut container {
            Value::List(ref mut items) => {
                match index_value {
                    Value::Number(n) => {
                        let index = n as usize;
                        if index < items.len() {
                            items[index] = new_value;
                        } else {
                            return Err(RuntimeError::InvalidOperation(format!(
                                "Индекс {} выходит за границы списка длины {}",
                                index, items.len()
                            )));
                        }
                    }
                    _ => return Err(RuntimeError::TypeMismatch(
                        "Индекс списка должен быть числом".to_string(),
                    )),
                }
            }
            Value::Dict(ref mut map) => {
                let key_str = match index_value {
                    Value::Text(s) => s,
                    Value::Number(n) => n.to_string(),
                    Value::Boolean(b) => (if b { "истина" } else { "ложь" }).to_string(),
                    _ => return Err(RuntimeError::TypeMismatch(
                        "Ключ словаря должен быть текстом, числом или логическим значением".to_string(),
                    )),
                };
                
                map.insert(key_str, new_value);
            }
            _ => return Err(RuntimeError::TypeMismatch(
                "Присваивание по индексу поддерживается только для списков и словарей".to_string(),
            )),
        }
        
        self.environment.set(&variable_name, container)?;
        Ok(())
    }
}
