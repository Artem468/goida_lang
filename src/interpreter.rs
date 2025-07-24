use crate::ast::*;
use std::collections::HashMap;
use std::io::{self, Write};

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Число(i64),
    Текст(String),
    Логический(bool),
    Пустота,
}

impl Value {
    fn to_string(&self) -> String {
        match self {
            Value::Число(n) => n.to_string(),
            Value::Текст(s) => s.clone(),
            Value::Логический(b) => if *b { "истина".to_string() } else { "ложь".to_string() },
            Value::Пустота => "пустота".to_string(),
        }
    }
    
    fn is_truthy(&self) -> bool {
        match self {
            Value::Логический(b) => *b,
            Value::Число(n) => *n != 0,
            Value::Текст(s) => !s.is_empty(),
            Value::Пустота => false,
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
        // Регистрируем функции
        for function in program.функции {
            self.functions.insert(function.имя.clone(), function);
        }
        
        // Выполняем операторы
        for statement in program.операторы {
            match self.execute_statement(&statement) {
                Err(RuntimeError::Return(_)) => {
                    // Игнорируем return на верхнем уровне
                }
                Err(e) => return Err(e),
                Ok(()) => {}
            }
        }
        
        Ok(())
    }
    
    fn execute_statement(&mut self, statement: &Statement) -> Result<(), RuntimeError> {
        match statement {
            Statement::Объявление { имя, тип: _, значение } => {
                let value = if let Some(expr) = значение {
                    self.evaluate_expression(expr)?
                } else {
                    Value::Пустота
                };
                self.environment.define(имя.clone(), value);
                Ok(())
            }
            
            Statement::Присваивание { имя, значение } => {
                let value = self.evaluate_expression(значение)?;
                self.environment.set(имя, value)?;
                Ok(())
            }
            
            Statement::Если { условие, тело, иначе } => {
                let condition_value = self.evaluate_expression(условие)?;
                if condition_value.is_truthy() {
                    for stmt in тело {
                        self.execute_statement(stmt)?;
                    }
                } else if let Some(else_body) = иначе {
                    for stmt in else_body {
                        self.execute_statement(stmt)?;
                    }
                }
                Ok(())
            }
            
            Statement::Пока { условие, тело } => {
                while self.evaluate_expression(условие)?.is_truthy() {
                    for stmt in тело {
                        self.execute_statement(stmt)?;
                    }
                }
                Ok(())
            }
            
            Statement::Для { переменная, начало, конец, тело } => {
                let start_val = self.evaluate_expression(начало)?;
                let end_val = self.evaluate_expression(конец)?;
                
                let (start, end) = match (start_val, end_val) {
                    (Value::Число(s), Value::Число(e)) => (s, e),
                    _ => return Err(RuntimeError::TypeMismatch("Цикл 'для' требует числовые значения".to_string())),
                };
                
                let parent_env = self.environment.clone();
                self.environment = Environment::with_parent(parent_env);
                
                for i in start..=end {
                    self.environment.define(переменная.clone(), Value::Число(i));
                    for stmt in тело {
                        self.execute_statement(stmt)?;
                    }
                }
                
                // Восстанавливаем родительскую среду
                if let Some(parent) = self.environment.parent.take() {
                    self.environment = *parent;
                }
                Ok(())
            }
            
            Statement::Возврат(expr) => {
                let value = if let Some(e) = expr {
                    self.evaluate_expression(e)?
                } else {
                    Value::Пустота
                };
                Err(RuntimeError::Return(value))
            }
            
            Statement::Выражение(expr) => {
                self.evaluate_expression(expr)?;
                Ok(())
            }
            
            Statement::Печать(expr) => {
                let value = self.evaluate_expression(expr)?;
                println!("{}", value.to_string());
                Ok(())
            }
            
            Statement::Блок(statements) => {
                let parent_env = self.environment.clone();
                self.environment = Environment::with_parent(parent_env);
                
                for stmt in statements {
                    self.execute_statement(stmt)?;
                }
                
                // Восстанавливаем родительскую среду
                if let Some(parent) = self.environment.parent.take() {
                    self.environment = *parent;
                }
                Ok(())
            }
        }
    }
    
    fn evaluate_expression(&mut self, expression: &Expression) -> Result<Value, RuntimeError> {
        match expression {
            Expression::Число(n) => Ok(Value::Число(*n)),
            Expression::Текст(s) => Ok(Value::Текст(s.clone())),
            Expression::Логический(b) => Ok(Value::Логический(*b)),
            
            Expression::Идентификатор(name) => {
                self.environment.get(name)
                    .ok_or_else(|| RuntimeError::UndefinedVariable(name.clone()))
            }
            
            Expression::БинарнаяОперация { левый, оператор, правый } => {
                let left_val = self.evaluate_expression(левый)?;
                let right_val = self.evaluate_expression(правый)?;
                
                match оператор {
                    BinaryOperator::Плюс => self.add_values(left_val, right_val),
                    BinaryOperator::Минус => self.subtract_values(left_val, right_val),
                    BinaryOperator::Умножить => self.multiply_values(left_val, right_val),
                    BinaryOperator::Разделить => self.divide_values(left_val, right_val),
                    BinaryOperator::Остаток => self.modulo_values(left_val, right_val),
                    BinaryOperator::Равно => Ok(Value::Логический(left_val == right_val)),
                    BinaryOperator::НеРавно => Ok(Value::Логический(left_val != right_val)),
                    BinaryOperator::Больше => self.compare_greater(left_val, right_val),
                    BinaryOperator::Меньше => self.compare_less(left_val, right_val),
                    BinaryOperator::БольшеРавно => self.compare_greater_equal(left_val, right_val),
                    BinaryOperator::МеньшеРавно => self.compare_less_equal(left_val, right_val),
                    BinaryOperator::И => Ok(Value::Логический(left_val.is_truthy() && right_val.is_truthy())),
                    BinaryOperator::Или => Ok(Value::Логический(left_val.is_truthy() || right_val.is_truthy())),
                }
            }
            
            Expression::УнарнаяОперация { оператор, операнд } => {
                let value = self.evaluate_expression(операнд)?;
                
                match оператор {
                    UnaryOperator::Минус => {
                        match value {
                            Value::Число(n) => Ok(Value::Число(-n)),
                            _ => Err(RuntimeError::TypeMismatch("Унарный минус применим только к числам".to_string())),
                        }
                    }
                    UnaryOperator::Не => Ok(Value::Логический(!value.is_truthy())),
                }
            }
            
            Expression::ВызовФункции { имя, аргументы } => {
                if имя == "ввод" {
                    return self.input_function();
                }
                
                if let Some(function) = self.functions.get(имя).cloned() {
                    if аргументы.len() != function.параметры.len() {
                        return Err(RuntimeError::InvalidOperation(
                            format!("Функция {} ожидает {} аргументов, получено {}", 
                                   имя, function.параметры.len(), аргументы.len())
                        ));
                    }
                    
                    let parent_env = self.environment.clone();
                    self.environment = Environment::with_parent(parent_env);
                    
                    // Связываем параметры с аргументами
                    for (param, arg_expr) in function.параметры.iter().zip(аргументы.iter()) {
                        let arg_value = self.evaluate_expression(arg_expr)?;
                        self.environment.define(param.имя.clone(), arg_value);
                    }
                    
                    // Выполняем тело функции
                    let mut result = Value::Пустота;
                    for stmt in &function.тело {
                        match self.execute_statement(stmt) {
                            Ok(()) => {}
                            Err(RuntimeError::Return(value)) => {
                                result = value;
                                break;
                            }
                            Err(e) => {
                                // Восстанавливаем родительскую среду
                                if let Some(parent) = self.environment.parent.take() {
                                    self.environment = *parent;
                                }
                                return Err(e);
                            }
                        }
                    }
                    
                    // Восстанавливаем родительскую среду
                    if let Some(parent) = self.environment.parent.take() {
                        self.environment = *parent;
                    }
                    Ok(result)
                } else {
                    Err(RuntimeError::UndefinedFunction(имя.clone()))
                }
            }
            
            Expression::ИндексДоступ { объект: _, индекс: _ } => {
                Err(RuntimeError::InvalidOperation("Индексный доступ еще не реализован".to_string()))
            }
        }
    }
    
    fn input_function(&self) -> Result<Value, RuntimeError> {
        print!("Введите значение: ");
        io::stdout().flush().unwrap();
        
        let mut input = String::new();
        io::stdin().read_line(&mut input).unwrap();
        let input = input.trim();
        
        // Пытаемся парсить как число
        if let Ok(num) = input.parse::<i64>() {
            Ok(Value::Число(num))
        } else {
            Ok(Value::Текст(input.to_string()))
        }
    }
    
    fn add_values(&self, left: Value, right: Value) -> Result<Value, RuntimeError> {
        match (left, right) {
            (Value::Число(a), Value::Число(b)) => Ok(Value::Число(a + b)),
            (Value::Текст(a), Value::Текст(b)) => Ok(Value::Текст(format!("{}{}", a, b))),
            (Value::Текст(a), Value::Число(b)) => Ok(Value::Текст(format!("{}{}", a, b))),
            (Value::Число(a), Value::Текст(b)) => Ok(Value::Текст(format!("{}{}", a, b))),
            (Value::Текст(a), Value::Логический(b)) => Ok(Value::Текст(format!("{}{}", a, if b { "истина" } else { "ложь" }))),
            (Value::Логический(a), Value::Текст(b)) => Ok(Value::Текст(format!("{}{}", if a { "истина" } else { "ложь" }, b))),
            _ => Err(RuntimeError::TypeMismatch("Неподдерживаемые типы для операции сложения".to_string())),
        }
    }
    
    fn subtract_values(&self, left: Value, right: Value) -> Result<Value, RuntimeError> {
        match (left, right) {
            (Value::Число(a), Value::Число(b)) => Ok(Value::Число(a - b)),
            _ => Err(RuntimeError::TypeMismatch("Вычитание применимо только к числам".to_string())),
        }
    }
    
    fn multiply_values(&self, left: Value, right: Value) -> Result<Value, RuntimeError> {
        match (left, right) {
            (Value::Число(a), Value::Число(b)) => Ok(Value::Число(a * b)),
            _ => Err(RuntimeError::TypeMismatch("Умножение применимо только к числам".to_string())),
        }
    }
    
    fn divide_values(&self, left: Value, right: Value) -> Result<Value, RuntimeError> {
        match (left, right) {
            (Value::Число(a), Value::Число(b)) => {
                if b == 0 {
                    Err(RuntimeError::DivisionByZero)
                } else {
                    Ok(Value::Число(a / b))
                }
            }
            _ => Err(RuntimeError::TypeMismatch("Деление применимо только к числам".to_string())),
        }
    }
    
    fn modulo_values(&self, left: Value, right: Value) -> Result<Value, RuntimeError> {
        match (left, right) {
            (Value::Число(a), Value::Число(b)) => {
                if b == 0 {
                    Err(RuntimeError::DivisionByZero)
                } else {
                    Ok(Value::Число(a % b))
                }
            }
            _ => Err(RuntimeError::TypeMismatch("Остаток от деления применим только к числам".to_string())),
        }
    }
    
    fn compare_greater(&self, left: Value, right: Value) -> Result<Value, RuntimeError> {
        match (left, right) {
            (Value::Число(a), Value::Число(b)) => Ok(Value::Логический(a > b)),
            _ => Err(RuntimeError::TypeMismatch("Сравнение применимо только к числам".to_string())),
        }
    }
    
    fn compare_less(&self, left: Value, right: Value) -> Result<Value, RuntimeError> {
        match (left, right) {
            (Value::Число(a), Value::Число(b)) => Ok(Value::Логический(a < b)),
            _ => Err(RuntimeError::TypeMismatch("Сравнение применимо только к числам".to_string())),
        }
    }
    
    fn compare_greater_equal(&self, left: Value, right: Value) -> Result<Value, RuntimeError> {
        match (left, right) {
            (Value::Число(a), Value::Число(b)) => Ok(Value::Логический(a >= b)),
            _ => Err(RuntimeError::TypeMismatch("Сравнение применимо только к числам".to_string())),
        }
    }
    
    fn compare_less_equal(&self, left: Value, right: Value) -> Result<Value, RuntimeError> {
        match (left, right) {
            (Value::Число(a), Value::Число(b)) => Ok(Value::Логический(a <= b)),
            _ => Err(RuntimeError::TypeMismatch("Сравнение применимо только к числам".to_string())),
        }
    }
}
