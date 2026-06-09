use crate::ast::class::Visibility;
use crate::ast::prelude::{ExprId, Span, StmtId, TypeId};
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
pub enum MethodType<NativeMethod = ()> {
    User(Arc<FunctionDefinition>),
    Native(Arc<NativeMethod>),
}

#[derive(Clone, Debug)]
/// Runtime class metadata, including fields, methods, constructor and base class.
pub struct ClassDefinition<NativeMethod = (), RuntimeValue = ()> {
    pub name: Symbol,
    pub base_class: Option<Symbol>,
    pub fields: HashMap<Symbol, (Visibility, bool, FieldData<RuntimeValue>)>,
    pub methods: HashMap<Symbol, (Visibility, bool, MethodType<NativeMethod>)>,
    pub constructor: Option<MethodType<NativeMethod>>,
    pub span: Span,
}

#[derive(Clone, Debug)]
/// Stored field initializer or already computed static/native value.
pub enum FieldData<RuntimeValue = ()> {
    Expression(Option<ExprId>),
    Value(RuntimeValue),
}

#[derive(Debug, Clone)]
/// Diagnostic payload shared by parse and runtime errors.
pub struct ErrorData {
    pub location: Span,
    pub message: String,
    pub stack_trace: Vec<StackFrame>,
}

impl From<Option<ExprId>> for FieldData {
    fn from(expression: Option<ExprId>) -> Self {
        Self::Expression(expression)
    }
}

#[derive(Debug, Clone)]
/// One source-level call frame attached to a runtime error.
pub struct StackFrame {
    pub name: String,
    pub location: Span,
}

impl ErrorData {
    /// Creates a diagnostic payload at the given source span.
    pub fn new(location: Span, message: String) -> ErrorData {
        ErrorData {
            location,
            message,
            stack_trace: Vec::new(),
        }
    }

    pub fn push_frame(&mut self, name: impl Into<String>, location: Span) {
        self.stack_trace.push(StackFrame {
            name: name.into(),
            location,
        });
    }
}
