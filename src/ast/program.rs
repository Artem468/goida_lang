use crate::ast::class::Visibility;
use crate::ast::prelude::{ExprId, Span, StmtId, TypeId};
use crate::interpreter::prelude::{BuiltinFn, Value};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use string_interner::DefaultSymbol as Symbol;

#[derive(Debug, Clone, PartialEq)]
pub struct FunctionDefinition {
    pub name: Symbol,
    pub params: Vec<Parameter>,
    pub return_type: Option<TypeId>,
    pub body: StmtId,
    pub span: Span,
    pub module: Option<Symbol>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Parameter {
    pub name: Symbol,
    pub param_type: TypeId,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct Import {
    pub files: Vec<Symbol>,
    pub span: Span,
}

#[derive(Clone, Debug)]
pub enum MethodType {
    User(Arc<FunctionDefinition>),
    Native(Arc<BuiltinFn>),
}

#[derive(Clone, Debug)]
pub struct ClassDefinition {
    pub name: Symbol,
    pub fields: HashMap<Symbol, (Visibility, bool, FieldData)>,
    pub methods: HashMap<Symbol, (Visibility, bool, MethodType)>,
    pub constructor: Option<MethodType>,
    pub span: Span,
}

#[derive(Clone, Debug)]
pub enum FieldData {
    Expression(Option<ExprId>),
    Value(Arc<RwLock<Value>>),
}

#[derive(Clone, Debug)]
pub struct ClassInstance {
    pub class_name: Symbol,
    pub fields: HashMap<Symbol, Option<ExprId>>,
    pub field_values: HashMap<Symbol, Value>,
    pub class_ref: Arc<RwLock<ClassDefinition>>,
}

#[derive(Debug, Clone)]
pub struct ErrorData {
    pub location: Span,
    pub message: String,
}

impl ErrorData {
    pub fn new(location: Span, message: String) -> ErrorData {
        ErrorData { location, message }
    }
}
