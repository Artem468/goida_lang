use crate::ast::prelude::{AstArena, ExprId, FunctionDefinition, Program, StmtId};

pub trait AstVisitor<T> {
    fn visit_program(&mut self, program: &Program) -> T;
    fn visit_function(&mut self, function: &FunctionDefinition, arena: &AstArena) -> T;
    fn visit_statement(&mut self, stmt_id: StmtId, arena: &AstArena) -> T;
    fn visit_expression(&mut self, expr_id: ExprId, arena: &AstArena) -> T;
}