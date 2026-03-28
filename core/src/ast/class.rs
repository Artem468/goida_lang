use string_interner::{DefaultSymbol as Symbol};
use crate::ast::prelude::{ExprId, Parameter, Span, StmtId, TypeId};

#[derive(Debug, Clone, PartialEq)]
pub enum Visibility {
    Private,
    Public,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ClassField {
    pub name: Symbol,
    pub field_type: Option<TypeId>,
    pub visibility: Visibility,
    pub is_static: bool,
    pub default_value: Option<ExprId>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ClassMethod {
    pub name: Symbol,
    pub params: Vec<Parameter>,
    pub return_type: Option<TypeId>,
    pub body: StmtId,
    pub visibility: Visibility,
    pub is_static: bool,
    pub is_constructor: bool,
    pub span: Span,
}