use std::collections::HashMap;
use crate::ast::{Function, Program};

use std::rc::Rc;

use std::cell::RefCell;

#[derive(Debug, Clone)]
pub enum Value {
    Number(i64),
    Float(f64),
    Text(String),
    Boolean(bool),
    Object(Rc<RefCell<ClassInstance>>),
    Empty,
}

impl PartialEq for Value {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Value::Number(a), Value::Number(b)) => a == b,
            (Value::Float(a), Value::Float(b)) => a == b,
            (Value::Text(a), Value::Text(b)) => a == b,
            (Value::Boolean(a), Value::Boolean(b)) => a == b,
            (Value::Object(a), Value::Object(b)) => {
                
                Rc::ptr_eq(a, b)
            }
            (Value::Empty, Value::Empty) => true,
            _ => false,
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
    IOError(String),
    ParseError(String),
}

#[derive(Debug, Clone)]
pub struct Environment {
    pub(crate) variables: HashMap<String, Value>,
    pub(crate) parent: Option<Box<Environment>>,
}


#[derive(Debug, Clone, PartialEq)]
pub struct Class {
    pub name: String,
    pub fields: HashMap<String, (crate::ast::FieldVisibility, Option<Value>)>,
    pub methods: HashMap<String, (crate::ast::FieldVisibility, Function)>,
    pub constructor: Option<Function>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ClassInstance {
    pub class_name: String,
    pub fields: HashMap<String, Value>,
    pub class_ref: Rc<Class>,
}

#[derive(Debug)]
pub struct Interpreter {
    pub(crate) environment: Environment,
    pub(crate) functions: HashMap<String, Function>,
    pub(crate) classes: HashMap<String, Rc<Class>>,
    pub(crate) modules: HashMap<String, Module>,
    pub(crate) current_dir: std::path::PathBuf,
    pub(crate) current_module: Option<String>
}

#[derive(Debug, Clone)]
pub struct Module {
    pub(crate) functions: HashMap<String, Function>,
    pub(crate) classes: HashMap<String, Rc<Class>>,
    pub(crate) environment: Environment,
    pub(crate) program: Program,
}
