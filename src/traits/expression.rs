use string_interner::DefaultSymbol;
use crate::ast::prelude::{ExprId, ExpressionKind, LiteralValue};
use crate::ast::program::FieldData;
use crate::interpreter::prelude::{RuntimeError, Value, Module};

pub trait ExpressionEvaluator {
    fn evaluate_expression(
        &mut self,
        expr_id: ExprId,
        current_module_id: DefaultSymbol,
    ) -> Result<Value, RuntimeError>;
    fn find_in_module_imports(&self, module: &Module, symbol: &DefaultSymbol) -> Option<Value>;
}

impl ExpressionKind {
    pub fn as_literal(&self) -> Option<&LiteralValue> {
        if let ExpressionKind::Literal(lit) = self {
            Some(lit)
        } else {
            None
        }
    }
}

impl From<Option<ExprId>> for FieldData {
    fn from(expr: Option<ExprId>) -> Self {
        FieldData::Expression(expr)
    }
}