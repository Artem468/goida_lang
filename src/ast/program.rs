use string_interner::{DefaultSymbol as Symbol};
use crate::ast::prelude::{AstArena, ClassDefinition, Span, StmtId, TypeId};

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

#[derive(Debug, Clone)]
pub struct Program {
    pub name: Symbol,
    pub functions: Vec<FunctionDefinition>,
    pub classes: Vec<ClassDefinition>,
    pub statements: Vec<StmtId>,
    pub imports: Vec<Import>,
    pub arena: AstArena,
}

impl Program {
    pub fn new(name: String) -> Self {
        let mut arena = AstArena::new();
        let name_symbol = arena.intern_string(&name);

        Self {
            name: name_symbol,
            functions: Vec::new(),
            classes: Vec::new(),
            statements: Vec::new(),
            imports: Vec::new(),
            arena,
        }
    }
}
