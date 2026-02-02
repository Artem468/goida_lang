use std::collections::HashMap;
use std::rc::Rc;

use crate::ast::prelude::{AstArena, ClassDefinition, ErrorData, FunctionDefinition, Import, Span, StmtId};
pub(crate) use crate::ast::program::ClassInstance;
use std::cell::RefCell;
use std::fmt::Debug;
use std::sync::{Arc, RwLock};
use string_interner::{DefaultSymbol as Symbol, StringInterner};
use string_interner::backend::StringBackend;
use crate::parser::structs::ParseError;

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
    List(Rc<RefCell<Vec<Value>>>),
    Array(Rc<Vec<Value>>),
    Dict(Rc<RefCell<HashMap<String, Value>>>),
    Empty,
}

#[derive(Clone)]
pub struct BuiltinFn(
    pub Arc<dyn Fn(&Interpreter, Vec<Value>, Span) -> Result<Value, RuntimeError> + Send + Sync>,
);

#[derive(Debug)]
pub enum RuntimeError {
    UndefinedVariable(ErrorData),
    UndefinedFunction(ErrorData),
    UndefinedMethod(ErrorData),
    TypeMismatch(ErrorData),
    DivisionByZero(ErrorData),
    InvalidOperation(ErrorData),
    Return(ErrorData, Value),
    TypeError(ErrorData),
    IOError(ErrorData),
    ImportError(ParseError),
}

#[derive(Clone, Debug)]
pub struct Environment {
    pub(crate) variables: HashMap<Symbol, Value>,
    pub(crate) parent: Option<Box<Environment>>,
}

pub type SharedInterner = Arc<RwLock<StringInterner<StringBackend>>>;

#[derive(Debug)]
pub struct Interpreter {
    pub(crate) std_classes: HashMap<Symbol, Rc<ClassDefinition>>,
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
