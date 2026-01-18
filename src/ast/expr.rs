use string_interner::{DefaultSymbol as Symbol};
use crate::ast::prelude::{BinaryOperator, Program, Span, TypeId, UnaryOperator};

pub type ExprId = u32;

#[derive(Debug, Clone)]
pub enum ExpressionKind {
    Literal(LiteralValue),
    Identifier(Symbol),
    Binary {
        op: BinaryOperator,
        left: ExprId,
        right: ExprId,
    },
    Unary {
        op: UnaryOperator,
        operand: ExprId,
    },
    FunctionCall {
        function: ExprId,
        args: Vec<ExprId>,
    },
    Index {
        object: ExprId,
        index: ExprId,
    },
    Input(ExprId),

    PropertyAccess {
        object: ExprId,
        property: Symbol,
    },
    MethodCall {
        object: ExprId,
        method: Symbol,
        args: Vec<ExprId>,
    },
    ObjectCreation {
        class_name: Symbol,
        args: Vec<ExprId>,
    },
    This,
}

#[derive(Debug, Clone)]
pub enum LiteralValue {
    Number(i64),
    Float(f64),
    Text(Symbol),
    Boolean(bool),
    Unit,
}

#[derive(Debug, Clone)]
pub struct ExpressionNode {
    pub kind: ExpressionKind,
    pub span: Span,
    pub type_hint: Option<TypeId>,
}
