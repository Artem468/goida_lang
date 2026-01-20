use crate::ast::prelude::{Program, StmtId};
use crate::interpreter::prelude::RuntimeError;

pub trait StatementExecutor {
    fn execute_statement(&mut self, stmt_id: StmtId, program: &Program)
                         -> Result<(), RuntimeError>;
}