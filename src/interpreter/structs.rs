use std::collections::HashMap;
use std::rc::Rc;

use crate::ast::prelude::{FunctionDefinition, Program, Visibility};
use std::cell::RefCell;
use std::sync::Arc;
use string_interner::DefaultSymbol as Symbol;

#[derive(Clone)]
pub enum Value {
    Number(i64),
    Float(f64),
    Text(String),
    Boolean(bool),
    Object(Rc<RefCell<ClassInstance>>),
    Function(Rc<FunctionDefinition>),
    Builtin(BuiltinFn),
    Empty,
}

impl PartialEq for Value {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Value::Number(a), Value::Number(b)) => a == b,
            (Value::Float(a), Value::Float(b)) => a == b,
            (Value::Text(a), Value::Text(b)) => a == b,
            (Value::Boolean(a), Value::Boolean(b)) => a == b,
            (Value::Object(a), Value::Object(b)) => Rc::ptr_eq(a, b),
            (Value::Function(a), Value::Function(b)) => Rc::ptr_eq(a, b),
            (Value::Empty, Value::Empty) => true,
            _ => false,
        }
    }
}

pub type BuiltinFn =
    Arc<dyn Fn(&Interpreter, Vec<Value>) -> Result<Value, RuntimeError> + Send + Sync>;


pub enum RuntimeError {
    UndefinedVariable(String),
    UndefinedFunction(String),
    UndefinedMethod(String),
    TypeMismatch(String),
    DivisionByZero,
    InvalidOperation(String),
    Return(Value),
    TypeError(String),
    IOError(String),
}

#[derive(Clone)]
pub struct Environment {
    pub(crate) variables: HashMap<String, Value>,
    pub(crate) parent: Option<Box<Environment>>,
}

#[derive(Clone, PartialEq)]
pub struct Class {
    pub name: Symbol,
    pub fields: HashMap<String, (Visibility, Option<Value>)>,
    pub methods: HashMap<String, (Visibility, FunctionDefinition)>,
    pub constructor: Option<FunctionDefinition>,
}

#[derive(Clone, PartialEq)]
pub struct ClassInstance {
    pub class_name: Symbol,
    pub fields: HashMap<String, Value>,
    pub class_ref: Rc<Class>,
}

pub struct Interpreter {
    pub(crate) environment: Environment,
    pub(crate) program: Program,
    pub(crate) functions: HashMap<String, FunctionDefinition>,
    pub(crate) builtins: HashMap<String, BuiltinFn>,
    pub(crate) classes: HashMap<String, Rc<Class>>,
    pub(crate) modules: HashMap<String, Module>,
    pub(crate) current_dir: std::path::PathBuf,
    pub(crate) current_module: Option<String>,
}

#[derive(Clone)]
pub struct Module {
    pub(crate) functions: HashMap<String, FunctionDefinition>,
    pub(crate) classes: HashMap<String, Rc<Class>>,
    pub(crate) environment: Environment,
    pub(crate) program: Program,
}
