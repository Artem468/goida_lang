use crate::ast::prelude::{BinaryOperator, Parameter, Span, StmtId, TypeId, UnaryOperator};
use string_interner::DefaultSymbol as Symbol;

/// Stable index of an expression inside [`AstArena`](crate::ast::arena::AstArena).
pub type ExprId = u32;

#[derive(Debug, Clone)]
/// Function or method call argument.
pub struct CallArg {
    /// Argument name for `имя=значение`; `None` for positional arguments.
    pub name: Option<Symbol>,
    /// Expression that produces the argument value.
    pub value: ExprId,
}

#[derive(Debug, Clone)]
/// All expression forms supported by the language AST.
pub enum ExpressionKind {
    /// Compile-time literal.
    Literal(LiteralValue),
    /// Local/global/module identifier.
    Identifier(Symbol),
    /// Left-associative binary operation.
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
        args: Vec<CallArg>,
    },
    Index {
        object: ExprId,
        index: ExprId,
    },

    PropertyAccess {
        object: ExprId,
        property: Symbol,
    },
    MethodCall {
        object: ExprId,
        method: Symbol,
        args: Vec<CallArg>,
    },
    ObjectCreation {
        class_name: Symbol,
        args: Vec<CallArg>,
    },
    Lambda {
        params: Vec<Parameter>,
        body: StmtId,
    },
    This,
}

#[derive(Debug, Clone)]
/// Literal value stored directly in the AST.
pub enum LiteralValue {
    Number(i64),
    Float(f64),
    Text(Symbol),
    Boolean(bool),
    Unit,
}

#[derive(Debug, Clone)]
/// Expression plus source span and optional inferred/declared type hint.
pub struct ExpressionNode {
    pub kind: ExpressionKind,
    pub span: Span,
    pub type_hint: Option<TypeId>,
}
