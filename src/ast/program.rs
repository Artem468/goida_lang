use crate::ast::class::Visibility;
use crate::ast::prelude::{ExprId, Span, StmtId, TypeId};
use crate::interpreter::prelude::Value;
use std::collections::HashMap;
use std::ops::Range;
use std::rc::Rc;
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

#[derive(Clone, PartialEq, Debug)]
pub struct ClassDefinition {
    pub name: Symbol,
    pub fields: HashMap<Symbol, (Visibility, Option<ExprId>)>,
    pub methods: HashMap<Symbol, (Visibility, FunctionDefinition)>,
    pub constructor: Option<FunctionDefinition>,
    pub span: Span,
}

#[derive(Clone, PartialEq, Debug)]
pub struct ClassInstance {
    pub class_name: Symbol,
    pub fields: HashMap<Symbol, Option<ExprId>>,
    pub field_values: HashMap<Symbol, Value>,
    pub class_ref: Rc<ClassDefinition>,
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
