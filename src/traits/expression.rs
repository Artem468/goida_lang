use crate::ast::prelude::{ExprId, Program};
use crate::interpreter::prelude::{RuntimeError, Value};

pub trait ExpressionEvaluator {
    fn evaluate_expression(
        &mut self,
        expr_id: ExprId,
        program: &Program,
    ) -> Result<Value, RuntimeError>;
}