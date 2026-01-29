use std::collections::HashMap;
use std::rc::Rc;

use crate::ast::prelude::{AstArena, ClassDefinition, FunctionDefinition, Import, StmtId};
pub(crate) use crate::ast::program::ClassInstance;
use std::cell::RefCell;
use std::fmt::Debug;
use std::sync::{Arc, RwLock};
use string_interner::{DefaultSymbol as Symbol, StringInterner};

#[derive(Clone, Debug)]
pub enum Value {
    Number(i64),
    Float(f64),
    Text(String),
    Boolean(bool),
    Object(Rc<RefCell<ClassInstance>>),
    Function(Rc<FunctionDefinition>),
    Builtin(BuiltinFn),
    Module(Symbol),
    Empty,
}

#[derive(Clone)]
pub struct BuiltinFn(
    pub Arc<dyn Fn(&Interpreter, Vec<Value>) -> Result<Value, RuntimeError> + Send + Sync>,
);

#[derive(Debug)]
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

#[derive(Clone, Debug)]
pub struct Environment {
    pub(crate) variables: HashMap<Symbol, Value>,
    pub(crate) parent: Option<Box<Environment>>,
}

pub type SharedInterner = Arc<RwLock<StringInterner>>;

#[derive(Debug)]
pub struct Interpreter {
    pub(crate) builtins: HashMap<Symbol, BuiltinFn>,
    pub(crate) modules: HashMap<Symbol, Module>,
    pub(crate) interner: SharedInterner,
    pub(crate) environment: Environment,
}

#[derive(Clone, Debug)]
pub struct Module {
    pub name: Symbol,
    pub path: std::path::PathBuf,
    pub arena: AstArena,

    pub functions: HashMap<Symbol, FunctionDefinition>,
    pub classes: HashMap<Symbol, Rc<ClassDefinition>>,

    pub body: Vec<StmtId>,
    pub imports: Vec<Import>,

    pub globals: HashMap<Symbol, Value>,
}
