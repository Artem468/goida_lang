use std::collections::HashMap;
use crate::ast::{Function, Program};

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Number(i64),
    Float(f64),
    Text(String),
    Boolean(bool),
    Empty,
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

#[derive(Debug)]
pub struct Interpreter {
    pub(crate) environment: Environment,
    pub(crate) functions: HashMap<String, Function>,
    pub(crate) modules: HashMap<String, Module>,
    pub(crate) current_dir: std::path::PathBuf,
    pub(crate) current_module: Option<String>
}

#[derive(Debug, Clone)]
pub struct Module {
    pub(crate) functions: HashMap<String, Function>,
    pub(crate) environment: Environment,
    pub(crate) program: Program,
}
