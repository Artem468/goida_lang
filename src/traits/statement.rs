use crate::ast::prelude::{StmtId};
use crate::interpreter::prelude::RuntimeError;
use string_interner::DefaultSymbol;

pub trait StatementExecutor {
    fn execute_statement(
        &mut self,
        stmt_id: StmtId,
        current_module_id: DefaultSymbol,
    ) -> Result<(), RuntimeError>;
}
