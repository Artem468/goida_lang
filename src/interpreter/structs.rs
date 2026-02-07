use std::any::Any;
use std::collections::HashMap;

use crate::ast::prelude::{AstArena, ClassDefinition, ErrorData, FunctionDefinition, Import, Span, StmtId};
pub(crate) use crate::ast::program::ClassInstance;
use crate::parser::structs::ParseError;
use crate::shared::SharedMut;
use std::fmt::Debug;
use std::path::PathBuf;
use std::sync::Arc;
use string_interner::backend::StringBackend;
use string_interner::{DefaultSymbol as Symbol, StringInterner};
use crate::ast::source::SourceManager;

#[derive(Clone, Debug)]
pub enum Value {
    Number(i64),
    Float(f64),
    Text(String),
    Boolean(bool),
    Object(SharedMut<ClassInstance>),
    Class(SharedMut<ClassDefinition>),
    Function(Arc<FunctionDefinition>),
    Builtin(BuiltinFn),
    Module(Symbol),
    List(SharedMut<Vec<Value>>),
    Array(Arc<Vec<Value>>),
    Dict(SharedMut<HashMap<String, Value>>),
    NativeResource(SharedMut<Box<dyn Any + Send + Sync>>),
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
    Panic(ErrorData),
}

#[derive(Debug)]
pub struct Environment {
    pub(crate) variables: HashMap<Symbol, Value>,
    pub(crate) parent: Option<SharedMut<Environment>>,
}

pub type SharedInterner = SharedMut<StringInterner<StringBackend>>;

#[derive(Debug)]
pub struct Interpreter {
    pub(crate) std_classes: HashMap<Symbol, SharedMut<ClassDefinition>>,
    pub(crate) builtins: HashMap<Symbol, BuiltinFn>,
    pub(crate) modules: HashMap<Symbol, Module>,
    pub(crate) interner: SharedInterner,
    pub(crate) environment: SharedMut<Environment>,
    pub(crate) source_manager: SourceManager,
}

#[derive(Clone, Debug)]
pub struct Module {
    pub name: Symbol,
    pub path: PathBuf,
    pub arena: AstArena,

    pub functions: HashMap<Symbol, FunctionDefinition>,
    pub classes: HashMap<Symbol, SharedMut<ClassDefinition>>,

    pub body: Vec<StmtId>,
    pub imports: Vec<Import>,

    pub globals: HashMap<Symbol, Value>,
}
