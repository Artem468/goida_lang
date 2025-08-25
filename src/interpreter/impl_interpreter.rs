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

            StatementKind::IndexAssign { object: _, index: _, value: _, } => {
                Err(RuntimeError::InvalidOperation("Индексный доступ отключён".to_string()))
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

                let mut arguments = Vec::new();
                for &arg_id in args {
                    arguments.push(self.evaluate_expression(arg_id, program)?);
                }

                self.call_function_by_name(&func_name, arguments, program)
            }

            ExpressionKind::Index { object: _, index: _ } => {
                Err(RuntimeError::InvalidOperation("Индексный доступ отключён".to_string()))
            }

            ExpressionKind::List(_) => {
                Err(RuntimeError::InvalidOperation("Списки отключены".to_string()))
            }

            ExpressionKind::Dict(_) => {
                Err(RuntimeError::InvalidOperation("Словари отключены".to_string()))
            }

            ExpressionKind::Input(arg_id) => {
                let data = self.evaluate_expression(*arg_id, program)?;
                self.input_function(data)
            }
        }
    }

    fn call_function_by_name(&mut self, name: &str, arguments: Vec<Value>, program: &Program) -> Result<Value, RuntimeError> {
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