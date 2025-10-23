use string_interner::{DefaultSymbol as Symbol};
use crate::ast::prelude::{ClassDefinition, ExprId, FunctionDefinition, Span, TypeId};

pub type StmtId = u32;

#[derive(Debug, Clone)]
pub struct StatementNode {
    pub kind: StatementKind,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub enum StatementKind {
    Expression(ExprId),
    Let {
        name: Symbol,
        type_hint: Option<TypeId>,
        value: Option<ExprId>,
    },
    Assign {
        name: Symbol,
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
        start: ExprId,
        end: ExprId,
        body: StmtId,
    },
    Block(Vec<StmtId>),
    Return(Option<ExprId>),
    Print(ExprId),
    Input(ExprId),
    FunctionDefinition(FunctionDefinition),
    ClassDefinition(ClassDefinition),
    PropertyAssign {
        object: ExprId,
        property: Symbol,
        value: ExprId,
    },
}
