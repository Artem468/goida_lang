use crate::ast::prelude::*;
use crate::parser::prelude::{ParseError, Parser as ParserTrait};
use crate::parser::syntax as syn;

impl ParserTrait {
    pub(super) fn build_expr(&mut self, expr: syn::Expr) -> Result<ExprId, ParseError> {
        let span = self.span(expr.span);
        let kind = match expr.node {
            syn::ExprKind::Number(value) => ExpressionKind::Literal(LiteralValue::Number(value)),
            syn::ExprKind::Float(value) => ExpressionKind::Literal(LiteralValue::Float(value)),
            syn::ExprKind::Text(value) => {
                let symbol = self.intern(&value);
                ExpressionKind::Literal(LiteralValue::Text(symbol))
            }
            syn::ExprKind::Boolean(value) => ExpressionKind::Literal(LiteralValue::Boolean(value)),
            syn::ExprKind::Empty => ExpressionKind::Literal(LiteralValue::Unit),
            syn::ExprKind::Identifier(name) => ExpressionKind::Identifier(self.intern(&name)),
            syn::ExprKind::Binary { op, left, right } => ExpressionKind::Binary {
                op: self.binary_op(op),
                left: self.build_expr(*left)?,
                right: self.build_expr(*right)?,
            },
            syn::ExprKind::Unary { op, operand } => ExpressionKind::Unary {
                op: match op {
                    syn::UnaryOp::Negative => UnaryOperator::Negative,
                    syn::UnaryOp::Not => UnaryOperator::Not,
                },
                operand: self.build_expr(*operand)?,
            },
            syn::ExprKind::FunctionCall { function, args } => ExpressionKind::FunctionCall {
                function: self.build_expr(*function)?,
                args: self.build_call_args(args)?,
            },
            syn::ExprKind::MethodCall {
                object,
                method,
                args,
            } => ExpressionKind::MethodCall {
                object: self.build_expr(*object)?,
                method: self.intern(&method),
                args: self.build_call_args(args)?,
            },
            syn::ExprKind::PropertyAccess { object, property } => ExpressionKind::PropertyAccess {
                object: self.build_expr(*object)?,
                property: self.intern(&property),
            },
            syn::ExprKind::Index { object, index } => ExpressionKind::Index {
                object: self.build_expr(*object)?,
                index: self.build_expr(*index)?,
            },
            syn::ExprKind::ObjectCreation { class_name, args } => ExpressionKind::ObjectCreation {
                class_name: self.intern(&class_name),
                args: self.build_call_args(args)?,
            },
            syn::ExprKind::Lambda { params, body } => {
                let params = self.build_params(params)?;
                let body = match body {
                    syn::LambdaBody::Expr(expr) => {
                        let expr_span = self.span(expr.span.clone());
                        let expr = self.build_expr(*expr)?;
                        let return_stmt = self
                            .module
                            .arena
                            .add_statement(StatementKind::Return(Some(expr)), expr_span);
                        self.module
                            .arena
                            .add_statement(StatementKind::Block(vec![return_stmt]), expr_span)
                    }
                    syn::LambdaBody::Block(items, block_span) => {
                        let block_span = self.span(block_span);
                        let items = self.build_items_as_block(items)?;
                        self.module
                            .arena
                            .add_statement(StatementKind::Block(items), block_span)
                    }
                };
                ExpressionKind::Lambda { params, body }
            }
            syn::ExprKind::MacroCall(_) => {
                return Err(ParseError::InvalidSyntax(ErrorData::new(
                    span,
                    "Вызов макроса должен быть раскрыт до построения AST".into(),
                )));
            }
        };
        Ok(self.module.arena.add_expression(kind, span))
    }
}
