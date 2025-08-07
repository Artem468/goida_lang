use crate::ast::{BinaryOperator, ExprId, ExpressionKind, Function, Program, StatementKind, StmtId, UnaryOperator, LiteralValue};
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

    fn into_module(self, program: Program) -> Module {
        Module {
            functions: self.functions,
            environment: self.environment,
            program,
        }
    }

    pub fn interpret(&mut self, program: Program) -> Result<(), RuntimeError> {
        let module_name = program.arena.resolve_symbol(program.name).unwrap().to_string();
        self.current_module = Some(module_name.clone());

        for import in &program.imports {
            for path_symbol in &import.files {
                let path = program.arena.resolve_symbol(*path_symbol).unwrap();
                let relative_path = std::path::Path::new(path);
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
                sub_interpreter.interpret(program.clone())?;
                self.modules.insert(file_stem.to_string(), sub_interpreter.into_module(program));
            }
        }

        for function in &program.functions {
            let func_name = program.arena.resolve_symbol(function.name).unwrap().to_string();
            self.functions.insert(func_name, function.clone());
        }

        for &stmt_id in &program.statements {
            match self.execute_statement(stmt_id, &program) {
                Err(RuntimeError::Return(_)) => {}
                Err(e) => return Err(e),
                Ok(()) => {}
            }
        }

        Ok(())
    }

    fn execute_statement(&mut self, stmt_id: StmtId, program: &Program) -> Result<(), RuntimeError> {
        let stmt = program.arena.get_statement(stmt_id).unwrap();
        match &stmt.kind {
            StatementKind::Expression(expr_id) => {
                self.evaluate_expression(*expr_id, program)?;
                Ok(())
            }

            StatementKind::Let { name, type_hint: _, value } => {
                let name_str = program.arena.resolve_symbol(*name).unwrap().to_string();
                let val = if let Some(expr_id) = value {
                    self.evaluate_expression(*expr_id, program)?
                } else {
                    Value::Empty
                };
                self.environment.define(name_str, val);
                Ok(())
            }

            StatementKind::Assign { name, value } => {
                let name_str = program.arena.resolve_symbol(*name).unwrap().to_string();
                let val = self.evaluate_expression(*value, program)?;
                self.environment.set(&name_str, val)?;
                Ok(())
            }

            StatementKind::IndexAssign { object, index, value } => {
                self.execute_index_assignment(*object, *index, *value, program)
            }

            StatementKind::If { condition, then_body, else_body } => {
                let condition_value = self.evaluate_expression(*condition, program)?;
                if condition_value.is_truthy() {
                    self.execute_statement(*then_body, program)?;
                } else if let Some(else_stmt_id) = else_body {
                    self.execute_statement(*else_stmt_id, program)?;
                }
                Ok(())
            }

            StatementKind::While { condition, body } => {
                while self.evaluate_expression(*condition, program)?.is_truthy() {
                    self.execute_statement(*body, program)?;
                }
                Ok(())
            }

            StatementKind::For { variable, start, end, body } => {
                let variable_str = program.arena.resolve_symbol(*variable).unwrap().to_string();
                let start_val = self.evaluate_expression(*start, program)?;
                let end_val = self.evaluate_expression(*end, program)?;

                let (start_num, end_num) = match (start_val, end_val) {
                    (Value::Number(s), Value::Number(e)) => (s, e),
                    _ => {
                        return Err(RuntimeError::TypeMismatch(
                            "Цикл 'для' требует числовые значения".to_string(),
                        ))
                    }
                };

                let parent_env = self.environment.clone();
                self.environment = Environment::with_parent(parent_env);

                for i in start_num..=end_num {
                    self.environment.define(variable_str.clone(), Value::Number(i));
                    self.execute_statement(*body, program)?;
                }

                if let Some(parent) = self.environment.parent.take() {
                    self.environment = *parent;
                }
                Ok(())
            }

            StatementKind::Block(statements) => {
                let parent_env = self.environment.clone();
                self.environment = Environment::with_parent(parent_env);

                for &stmt_id in statements {
                    self.execute_statement(stmt_id, program)?;
                }

                if let Some(parent) = self.environment.parent.take() {
                    self.environment = *parent;
                }
                Ok(())
            }

            StatementKind::Return(expr) => {
                let value = if let Some(e) = expr {
                    self.evaluate_expression(*e, program)?
                } else {
                    Value::Empty
                };
                Err(RuntimeError::Return(value))
            }

            StatementKind::Print(expr_id) => {
                let value = self.evaluate_expression(*expr_id, program)?;
                println!("{}", value.to_string());
                Ok(())
            }

            StatementKind::Input(expr_id) => {
                let value = self.evaluate_expression(*expr_id, program)?;
                self.input_function(value)?;
                Ok(())
            }
        }
    }

    fn evaluate_expression(&mut self, expr_id: ExprId, program: &Program) -> Result<Value, RuntimeError> {
        let expr = program.arena.get_expression(expr_id).unwrap();
        match &expr.kind {
            ExpressionKind::Literal(literal) => match literal {
                LiteralValue::Number(n) => Ok(Value::Number(*n)),
                LiteralValue::Float(f) => Ok(Value::Float(*f)),
                LiteralValue::Text(symbol) => {
                    let text = program.arena.resolve_symbol(*symbol).unwrap().to_string();
                    Ok(Value::Text(text))
                }
                LiteralValue::Boolean(b) => Ok(Value::Boolean(*b)),
                LiteralValue::Unit => Ok(Value::Empty),
            },

            ExpressionKind::Identifier(symbol) => {
                let name = program.arena.resolve_symbol(*symbol).unwrap().to_string();
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
                        .ok_or_else(|| RuntimeError::UndefinedVariable(name))
                }
            }

            ExpressionKind::Binary { op, left, right } => {
                let left_val = self.evaluate_expression(*left, program)?;
                let right_val = self.evaluate_expression(*right, program)?;

                match op {
                    BinaryOperator::Add => self.add_values(left_val, right_val),
                    BinaryOperator::Sub => self.subtract_values(left_val, right_val),
                    BinaryOperator::Mul => self.multiply_values(left_val, right_val),
                    BinaryOperator::Div => self.divide_values(left_val, right_val),
                    BinaryOperator::Mod => self.modulo_values(left_val, right_val),
                    BinaryOperator::Eq => Ok(Value::Boolean(left_val == right_val)),
                    BinaryOperator::Ne => Ok(Value::Boolean(left_val != right_val)),
                    BinaryOperator::Gt => self.compare_greater(left_val, right_val),
                    BinaryOperator::Lt => self.compare_less(left_val, right_val),
                    BinaryOperator::Ge => self.compare_greater_equal(left_val, right_val),
                    BinaryOperator::Le => self.compare_less_equal(left_val, right_val),
                    BinaryOperator::And => Ok(Value::Boolean(
                        left_val.is_truthy() && right_val.is_truthy(),
                    )),
                    BinaryOperator::Or => Ok(Value::Boolean(
                        left_val.is_truthy() || right_val.is_truthy(),
                    )),
                    BinaryOperator::Assign => Err(RuntimeError::InvalidOperation(
                        "Оператор присваивания не поддерживается в выражениях".to_string(),
                    )),
                }
            }

            ExpressionKind::Unary { op, operand } => {
                let value = self.evaluate_expression(*operand, program)?;

                match op {
                    UnaryOperator::Negative => match value {
                        Value::Number(n) => Ok(Value::Number(-n)),
                        _ => Err(RuntimeError::TypeMismatch(
                            "Унарный минус применим только к числам".to_string(),
                        )),
                    },
                    UnaryOperator::Not => Ok(Value::Boolean(!value.is_truthy())),
                }
            }

            ExpressionKind::Call { function, args } => {
                let func_expr = program.arena.get_expression(*function).unwrap();
                let func_name = match &func_expr.kind {
                    ExpressionKind::Identifier(symbol) => {
                        program.arena.resolve_symbol(*symbol).unwrap().to_string()
                    }
                    _ => return Err(RuntimeError::InvalidOperation(
                        "Вызов функции возможен только по имени".to_string()
                    )),
                };

                if self.is_mutable_builtin(&func_name) {
                    return self.call_mutable_builtin(&func_name, args, program);
                }

                let mut arguments = Vec::new();
                for &arg_id in args {
                    arguments.push(self.evaluate_expression(arg_id, program)?);
                }

                self.call_function_by_name(&func_name, arguments, program)
            }

            ExpressionKind::Index { object, index } => {
                let obj_value = self.evaluate_expression(*object, program)?;
                let idx_value = self.evaluate_expression(*index, program)?;

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
                            }
                            _ => Err(RuntimeError::TypeMismatch(
                                "Индекс списка должен быть числом".to_string(),
                            ))
                        }
                    }
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
                    }
                    _ => Err(RuntimeError::TypeMismatch(
                        "Индексный доступ поддерживается только для списков и словарей".to_string(),
                    ))
                }
            }

            ExpressionKind::List(elements) => {
                let mut list_values = Vec::new();
                for &elem_id in elements {
                    list_values.push(self.evaluate_expression(elem_id, program)?);
                }
                Ok(Value::List(list_values))
            }

            ExpressionKind::Dict(pairs) => {
                let mut dict_map = HashMap::new();
                for &(key_id, value_id) in pairs {
                    let key = self.evaluate_expression(key_id, program)?;
                    let value = self.evaluate_expression(value_id, program)?;
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

            ExpressionKind::Input(arg_id) => {
                let data = self.evaluate_expression(*arg_id, program)?;
                self.input_function(data)
            }
        }
    }

    fn call_function_by_name(&mut self, name: &str, arguments: Vec<Value>, program: &Program) -> Result<Value, RuntimeError> {
        if let Ok(result) = self.call_builtin_function(name, &arguments) {
            return Ok(result);
        }

        if let Some(dot_index) = name.find('.') {
            let module_name = &name[..dot_index];
            let func_name = &name[dot_index + 1..];
            if let Some(module) = self.modules.get(module_name).cloned() {
                if let Some(function) = module.functions.get(func_name) {
                    return self.call_function(function.clone(), arguments, &module.program);
                }
            }
            Err(RuntimeError::UndefinedFunction(name.to_string()))
        } else {
            if let Some(function) = self.functions.get(name).cloned() {
                self.call_function(function, arguments, program)
            } else if let Some(module_name) = &self.current_module {
                if let Some(module) = self.modules.get(module_name).cloned() {
                    if let Some(function) = module.functions.get(name) {
                        return self.call_function(function.clone(), arguments, &module.program);
                    }
                }
                Err(RuntimeError::UndefinedFunction(name.to_string()))
            } else {
                Err(RuntimeError::UndefinedFunction(name.to_string()))
            }
        }
    }

    fn call_function(&mut self, function: Function, arguments: Vec<Value>, program: &Program) -> Result<Value, RuntimeError> {
        let prev_module = self.current_module.clone();
        let module_name = if let Some(module_symbol) = function.module {
            Some(program.arena.resolve_symbol(module_symbol).unwrap().to_string())
        } else {
            None
        };
        self.current_module = module_name;

        let parent_env = self.environment.clone();
        self.environment = Environment::with_parent(parent_env);

        if arguments.len() != function.params.len() {
            return Err(RuntimeError::InvalidOperation(format!(
                "Функция {} ожидает {} аргументов, получено {}",
                program.arena.resolve_symbol(function.name).unwrap(),
                function.params.len(),
                arguments.len()
            )));
        }

        for (param, arg_value) in function.params.iter().zip(arguments.iter()) {
            let param_name = program.arena.resolve_symbol(param.name).unwrap().to_string();
            self.environment.define(param_name, arg_value.clone());
        }

        let mut result = Value::Empty;
        match self.execute_statement(function.body, program) {
            Ok(()) => {}
            Err(RuntimeError::Return(val)) => {
                result = val;
            }
            Err(e) => {
                self.environment = self.environment.clone().pop();
                self.current_module = prev_module;
                return Err(e);
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

    fn call_builtin_function(&mut self, name: &str, arguments: &[Value]) -> Result<Value, RuntimeError> {
        match name {
            "добавить" => self.builtin_push(arguments),
            "извлечь" => self.builtin_pop(arguments),
            "удалить" => self.builtin_remove(arguments),
            "длина" => self.builtin_size(arguments),
            "содержит" => self.builtin_contains(arguments),
            _ => Err(RuntimeError::UndefinedFunction(name.to_string())),
        }
    }

    fn builtin_push(&mut self, arguments: &[Value]) -> Result<Value, RuntimeError> {
        match arguments.len() {
            2 => {
                match &arguments[0] {
                    Value::List(items) => {
                        let mut new_list = Vec::with_capacity(items.len() + 1);
                        new_list.extend_from_slice(items);
                        new_list.push(arguments[1].clone());
                        Ok(Value::List(new_list))
                    }
                    _ => Err(RuntimeError::TypeMismatch(
                        "Для добавления элемента первый аргумент должен быть списком".to_string(),
                    )),
                }
            }
            3 => {
                match &arguments[0] {
                    Value::Dict(map) => {
                        let key_str = match &arguments[1] {
                            Value::Text(s) => s.clone(),
                            Value::Number(n) => n.to_string(),
                            Value::Boolean(b) => (if *b { "истина" } else { "ложь" }).to_string(),
                            _ => return Err(RuntimeError::TypeMismatch(
                                "Ключ словаря должен быть текстом, числом или логическим значением".to_string(),
                            )),
                        };

                        let mut new_map = map.clone();
                        new_map.insert(key_str, arguments[2].clone());
                        Ok(Value::Dict(new_map))
                    }
                    _ => Err(RuntimeError::TypeMismatch(
                        "Для добавления пары ключ-значение первый аргумент должен быть словарём".to_string(),
                    )),
                }
            }
            _ => Err(RuntimeError::InvalidOperation(
                "Функция 'добавить' ожидает 2 аргумента для списков (список, элемент) или 3 для словарей (словарь, ключ, значение)".to_string(),
            )),
        }
    }

    fn builtin_pop(&mut self, arguments: &[Value]) -> Result<Value, RuntimeError> {
        if arguments.len() != 1 {
            return Err(RuntimeError::InvalidOperation(
                "Функция 'извлечь' ожидает 1 аргумент: список".to_string(),
            ));
        }

        match &arguments[0] {
            Value::List(items) => {
                if items.is_empty() {
                    Err(RuntimeError::InvalidOperation(
                        "Нельзя извлечь элемент из пустого списка".to_string(),
                    ))
                } else {
                    let mut new_list = items.clone();
                    let popped = new_list.pop().unwrap();
                    Ok(popped)
                }
            }
            _ => Err(RuntimeError::TypeMismatch(
                "Функция 'извлечь' применима только к спискам".to_string(),
            )),
        }
    }

    fn builtin_remove(&mut self, arguments: &[Value]) -> Result<Value, RuntimeError> {
        if arguments.len() != 2 {
            return Err(RuntimeError::InvalidOperation(
                "Функция 'удалить' ожидает 2 аргумента: (список/словарь, индекс/ключ)".to_string(),
            ));
        }

        match (&arguments[0], &arguments[1]) {
            (Value::List(items), Value::Number(index)) => {
                let idx = *index as usize;
                if idx >= items.len() {
                    return Err(RuntimeError::InvalidOperation(format!(
                        "Индекс {} выходит за границы списка длины {}",
                        idx, items.len()
                    )));
                }
                let mut new_list = items.clone();
                let removed = new_list.remove(idx);
                Ok(removed)
            }
            (Value::Dict(map), key) => {
                let key_str = match key {
                    Value::Text(s) => s.clone(),
                    Value::Number(n) => n.to_string(),
                    Value::Boolean(b) => (if *b { "истина" } else { "ложь" }).to_string(),
                    _ => return Err(RuntimeError::TypeMismatch(
                        "Ключ словаря должен быть текстом, числом или логическим значением".to_string(),
                    )),
                };

                let mut new_map = map.clone();
                match new_map.remove(&key_str) {
                    Some(removed) => Ok(removed),
                    None => Err(RuntimeError::InvalidOperation(format!(
                        "Ключ '{}' не найден в словаре",
                        key_str
                    ))),
                }
            }
            _ => Err(RuntimeError::TypeMismatch(
                "Функция 'удалить' применима к спискам (с числовым индексом) и словарям (с ключом)".to_string(),
            )),
        }
    }

    fn builtin_size(&mut self, arguments: &[Value]) -> Result<Value, RuntimeError> {
        if arguments.len() != 1 {
            return Err(RuntimeError::InvalidOperation(
                "Функция 'длина' ожидает 1 аргумент: список или словарь".to_string(),
            ));
        }

        match &arguments[0] {
            Value::List(items) => Ok(Value::Number(items.len() as i64)),
            Value::Dict(map) => Ok(Value::Number(map.len() as i64)),
            Value::Text(s) => Ok(Value::Number(s.len() as i64)),
            _ => Err(RuntimeError::TypeMismatch(
                "Функция 'длина' применима к спискам, словарям и строкам".to_string(),
            )),
        }
    }

    fn builtin_contains(&mut self, arguments: &[Value]) -> Result<Value, RuntimeError> {
        if arguments.len() != 2 {
            return Err(RuntimeError::InvalidOperation(
                "Функция 'содержит' ожидает 2 аргумента: (список/словарь, элемент/ключ)".to_string(),
            ));
        }

        match (&arguments[0], &arguments[1]) {
            (Value::List(items), element) => Ok(Value::Boolean(items.contains(element))),
            (Value::Dict(map), element) => {
                let key_str = match element {
                    Value::Text(s) => s.clone(),
                    Value::Number(n) => n.to_string(),
                    Value::Boolean(b) => (if *b { "истина" } else { "ложь" }).to_string(),
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

    fn is_mutable_builtin(&self, name: &str) -> bool {
        matches!(name, "добавить" | "извлечь" | "удалить")
    }

    fn call_mutable_builtin(&mut self, name: &str, args: &[ExprId], program: &Program) -> Result<Value, RuntimeError> {
        match name {
            "добавить" => {
                match args.len() {
                    2 => {
                        let var_name = self.get_variable_name_from_expr(args[0], program)?;
                        let element_value = self.evaluate_expression(args[1], program)?;

                        let mut current_value = self.environment.get(&var_name)
                            .ok_or_else(|| RuntimeError::UndefinedVariable(var_name.clone()))?;

                        match &mut current_value {
                            Value::List(items) => {
                                items.push(element_value);
                                self.environment.set(&var_name, current_value)?;
                                Ok(Value::Empty)
                            }
                            _ => Err(RuntimeError::TypeMismatch(
                                "Первый аргумент 'добавить' должен быть списком".to_string(),
                            )),
                        }
                    }
                    3 => {
                        let var_name = self.get_variable_name_from_expr(args[0], program)?;
                        let key_value = self.evaluate_expression(args[1], program)?;
                        let value_to_add = self.evaluate_expression(args[2], program)?;

                        let mut current_value = self.environment.get(&var_name)
                            .ok_or_else(|| RuntimeError::UndefinedVariable(var_name.clone()))?;

                        match &mut current_value {
                            Value::Dict(map) => {
                                let key_str = match key_value {
                                    Value::Text(s) => s,
                                    Value::Number(n) => n.to_string(),
                                    Value::Boolean(b) => (if b { "истина" } else { "ложь" }).to_string(),
                                    _ => return Err(RuntimeError::TypeMismatch(
                                        "Ключ словаря должен быть текстом, числом или логическим значением".to_string(),
                                    )),
                                };

                                map.insert(key_str, value_to_add);
                                self.environment.set(&var_name, current_value)?;
                                Ok(Value::Empty)
                            }
                            _ => Err(RuntimeError::TypeMismatch(
                                "Первый аргумент 'добавить' должен быть словарём для добавления пары ключ-значение".to_string(),
                            )),
                        }
                    }
                    _ => Err(RuntimeError::InvalidOperation(
                        "Функция 'добавить' ожидает 2 аргумента для списков или 3 для словарей".to_string(),
                    )),
                }
            }
            "извлечь" => {
                if args.len() != 1 {
                    return Err(RuntimeError::InvalidOperation(
                        "Функция 'извлечь' ожидает 1 аргумент".to_string(),
                    ));
                }

                let var_name = self.get_variable_name_from_expr(args[0], program)?;
                let mut current_value = self.environment.get(&var_name)
                    .ok_or_else(|| RuntimeError::UndefinedVariable(var_name.clone()))?;

                match &mut current_value {
                    Value::List(items) => {
                        if items.is_empty() {
                            Err(RuntimeError::InvalidOperation(
                                "Нельзя извлечь элемент из пустого списка".to_string(),
                            ))
                        } else {
                            let popped = items.pop().unwrap();
                            self.environment.set(&var_name, current_value)?;
                            Ok(popped)
                        }
                    }
                    _ => Err(RuntimeError::TypeMismatch(
                        "Функция 'извлечь' применима только к спискам".to_string(),
                    )),
                }
            }
            "удалить" => {
                if args.len() != 2 {
                    return Err(RuntimeError::InvalidOperation(
                        "Функция 'удалить' ожидает 2 аргумента".to_string(),
                    ));
                }

                let var_name = self.get_variable_name_from_expr(args[0], program)?;
                let index_or_key = self.evaluate_expression(args[1], program)?;

                let mut current_value = self.environment.get(&var_name)
                    .ok_or_else(|| RuntimeError::UndefinedVariable(var_name.clone()))?;

                match (&mut current_value, &index_or_key) {
                    (Value::List(items), Value::Number(index)) => {
                        let idx = *index as usize;
                        if idx >= items.len() {
                            return Err(RuntimeError::InvalidOperation(format!(
                                "Индекс {} выходит за границы списка длины {}",
                                idx, items.len()
                            )));
                        }
                        let removed = items.remove(idx);
                        self.environment.set(&var_name, current_value)?;
                        Ok(removed)
                    }
                    (Value::Dict(map), key) => {
                        let key_str = match key {
                            Value::Text(s) => s.clone(),
                            Value::Number(n) => n.to_string(),
                            Value::Boolean(b) => (if *b { "истина" } else { "ложь" }).to_string(),
                            _ => return Err(RuntimeError::TypeMismatch(
                                "Ключ словаря должен быть текстом, числом или логическим значением".to_string(),
                            )),
                        };

                        match map.remove(&key_str) {
                            Some(removed) => {
                                self.environment.set(&var_name, current_value)?;
                                Ok(removed)
                            }
                            None => Err(RuntimeError::InvalidOperation(format!(
                                "Ключ '{}' не найден в словаре",
                                key_str
                            ))),
                        }
                    }
                    _ => Err(RuntimeError::TypeMismatch(
                        "Функция 'удалить' применима к спискам (с числовым индексом) и словарям (с ключом)".to_string(),
                    )),
                }
            }
            _ => Err(RuntimeError::UndefinedFunction(name.to_string())),
        }
    }

    fn get_variable_name_from_expr(&self, expr_id: ExprId, program: &Program) -> Result<String, RuntimeError> {
        let expr = program.arena.get_expression(expr_id).unwrap();
        match &expr.kind {
            ExpressionKind::Identifier(symbol) => {
                Ok(program.arena.resolve_symbol(*symbol).unwrap().to_string())
            }
            _ => Err(RuntimeError::InvalidOperation(
                "Ожидается имя переменной".to_string(),
            )),
        }
    }

    fn execute_index_assignment(&mut self, object_id: ExprId, index_id: ExprId, value_id: ExprId, program: &Program) -> Result<(), RuntimeError> {
        let new_value = self.evaluate_expression(value_id, program)?;
        let index_value = self.evaluate_expression(index_id, program)?;

        let object_expr = program.arena.get_expression(object_id).unwrap();
        let variable_name = match &object_expr.kind {
            ExpressionKind::Identifier(symbol) => {
                program.arena.resolve_symbol(*symbol).unwrap().to_string()
            }
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
