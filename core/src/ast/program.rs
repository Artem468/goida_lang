use crate::ast::class::Visibility;
use crate::ast::prelude::{ExprId, Span, StmtId, TypeId};
use crate::interpreter::prelude::{BuiltinFn, Value};
use crate::shared::SharedMut;
use std::collections::HashMap;
use std::sync::Arc;
use string_interner::DefaultSymbol as Symbol;

#[derive(Debug, Clone, PartialEq)]
/// User-defined function with parsed parameters, return type and body.
pub struct FunctionDefinition {
    pub name: Symbol,
    pub params: Vec<Parameter>,
    pub return_type: Option<TypeId>,
    pub body: StmtId,
    pub span: Span,
    pub module: Option<Symbol>,
}

#[derive(Debug, Clone, PartialEq)]
/// Function, method, or constructor parameter.
pub struct Parameter {
    pub name: Symbol,
    pub param_type: TypeId,
    pub default_value: Option<ExprId>,
    pub span: Span,
}

#[derive(Debug, Clone)]
/// Resolved import target and alias.
pub struct ImportItem {
    pub path: Symbol,
    pub alias: Symbol,
}

#[derive(Debug, Clone)]
/// Source-level import declaration.
pub struct Import {
    pub item: ImportItem,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
/// Native library function signature declared by `библиотека`.
pub struct NativeFunctionDefinition {
    pub name: Symbol,
    pub params: Vec<Parameter>,
    pub return_type: Option<TypeId>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
/// Native library global variable signature.
pub struct NativeGlobalDefinition {
    pub name: Symbol,
    pub value_type: TypeId,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
/// Native dynamic library declaration and its exported symbols.
pub struct NativeLibraryDefinition {
    pub path: Symbol,
    pub functions: Vec<NativeFunctionDefinition>,
    pub globals: Vec<NativeGlobalDefinition>,
    pub span: Span,
}

#[derive(Clone, Debug)]
/// Callable class member implementation.
pub enum MethodType {
    User(Arc<FunctionDefinition>),
    Native(Arc<BuiltinFn>),
}

#[derive(Clone, Debug)]
/// Runtime class metadata, including fields, methods, constructor and base class.
pub struct ClassDefinition {
    pub name: Symbol,
    pub base_class: Option<Symbol>,
    pub fields: HashMap<Symbol, (Visibility, bool, FieldData)>,
    pub methods: HashMap<Symbol, (Visibility, bool, MethodType)>,
    pub constructor: Option<MethodType>,
    pub span: Span,
}

#[derive(Clone, Debug)]
/// Stored field initializer or already computed static/native value.
pub enum FieldData {
    Expression(Option<ExprId>),
    Value(SharedMut<Value>),
}

#[derive(Clone, Debug)]
/// Runtime object instance with per-instance field state.
pub struct ClassInstance {
    pub class_name: Symbol,
    pub fields: HashMap<Symbol, Option<ExprId>>,
    pub field_values: HashMap<Symbol, Value>,
    pub class_ref: SharedMut<ClassDefinition>,
}

#[derive(Debug, Clone)]
/// Diagnostic payload shared by parse and runtime errors.
pub struct ErrorData {
    pub location: Span,
    pub message: String,
}

impl ErrorData {
    /// Creates a diagnostic payload at the given source span.
    pub fn new(location: Span, message: String) -> ErrorData {
        ErrorData { location, message }
    }
}
