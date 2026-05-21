use crate::ast::prelude::{
    ClassDefinition, ExprId, FunctionDefinition, NativeLibraryDefinition, Span, TypeId,
};
use string_interner::DefaultSymbol as Symbol;

/// Stable index of a statement inside [`AstArena`](crate::ast::arena::AstArena).
pub type StmtId = u32;

#[derive(Debug, Clone)]
/// Statement plus its source span.
pub struct StatementNode {
    pub kind: StatementKind,
    pub span: Span,
}

#[derive(Debug, Clone)]
/// All executable and declarative statement forms.
pub enum StatementKind {
    Expression(ExprId),
    Assign {
        name: Symbol,
        type_hint: Option<TypeId>,
        value: ExprId,
    },
    IndexAssign {
        object: ExprId,
        index: ExprId,
        value: ExprId,
    },
    If {
        condition: ExprId,
        then_body: StmtId,
        else_body: Option<StmtId>,
    },
    While {
        condition: ExprId,
        body: StmtId,
    },
    For {
        variable: Symbol,
        init: ExprId,
        condition: ExprId,
        update: ExprId,
        body: StmtId,
    },
    Thread {
        body: StmtId,
    },
    Try {
        body: StmtId,
        handlers: Vec<TryHandler>,
    },
    Raise {
        error_type: Symbol,
        message: Option<ExprId>,
    },
    Block(Vec<StmtId>),
    Return(Option<ExprId>),
    FunctionDefinition(FunctionDefinition),
    NativeLibraryDefinition(NativeLibraryDefinition),
    ClassDefinition(ClassDefinition),
    PropertyAssign {
        object: ExprId,
        property: Symbol,
        value: ExprId,
    },
    Empty,
}

#[derive(Debug, Clone)]
/// Single `перехватить` branch of a `попробовать` statement.
pub struct TryHandler {
    /// Error class accepted by this handler; `None` means catch any runtime error.
    pub error_type: Option<Symbol>,
    /// Optional binding for the error message text.
    pub error_text: Option<Symbol>,
    /// Handler body statement id.
    pub body: StmtId,
}
