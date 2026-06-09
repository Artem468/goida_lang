use crate::ast::prelude::*;
use crate::parser::prelude::{ParseError, Parser as ParserTrait};
use crate::parser::syntax as syn;

impl ParserTrait {
    pub(super) fn build_statement(&mut self, stmt: syn::Stmt) -> Result<StmtId, ParseError> {
        let span = self.span(stmt.span);
        match stmt.node {
            syn::StmtKind::Assign {
                name,
                is_const,
                type_hint,
                value,
            } => {
                let type_hint = self.build_optional_type(type_hint, span)?;
                let value = self.build_expr(value)?;
                Ok(self.module.arena.add_statement(
                    StatementKind::Assign {
                        name: self.intern(&name),
                        is_const,
                        type_hint,
                        value,
                    },
                    span,
                ))
            }
            syn::StmtKind::AssignTarget { target, value } => {
                let target = self.build_expr(target)?;
                let value = self.build_expr(value)?;
                self.build_target_assignment(target, value, span)
            }
            syn::StmtKind::CompoundAssign { target, op, value } => {
                let target = self.build_expr(target)?;
                let value = self.build_expr(value)?;
                Ok(self.module.arena.add_statement(
                    StatementKind::CompoundAssign {
                        target,
                        op: self.compound_op(op),
                        value,
                    },
                    span,
                ))
            }
            syn::StmtKind::If {
                condition,
                then_body,
                else_body,
            } => {
                let condition = self.build_expr(condition)?;
                let then_items = self.build_items_as_block(then_body)?;
                let then_body = self
                    .module
                    .arena
                    .add_statement(StatementKind::Block(then_items), span);
                let else_body = match else_body {
                    Some(syn::ElseBody::Block(items, else_span)) => {
                        let else_span = self.span(else_span);
                        let items = self.build_items_as_block(items)?;
                        Some(
                            self.module
                                .arena
                                .add_statement(StatementKind::Block(items), else_span),
                        )
                    }
                    Some(syn::ElseBody::If(stmt)) => Some(self.build_statement(*stmt)?),
                    None => None,
                };
                Ok(self.module.arena.add_statement(
                    StatementKind::If {
                        condition,
                        then_body,
                        else_body,
                    },
                    span,
                ))
            }
            syn::StmtKind::While { condition, body } => {
                let condition = self.build_expr(condition)?;
                let body_items = self.build_items_as_block(body)?;
                let body = self
                    .module
                    .arena
                    .add_statement(StatementKind::Block(body_items), span);
                Ok(self
                    .module
                    .arena
                    .add_statement(StatementKind::While { condition, body }, span))
            }
            syn::StmtKind::For {
                variable,
                init,
                condition,
                update,
                body,
            } => {
                let init = self.build_expr(init)?;
                let condition = self.build_expr(condition)?;
                let update = self.build_for_update(*update)?;
                let body_items = self.build_items_as_block(body)?;
                let body = self
                    .module
                    .arena
                    .add_statement(StatementKind::Block(body_items), span);
                Ok(self.module.arena.add_statement(
                    StatementKind::For {
                        variable: self.intern(&variable),
                        init,
                        condition,
                        update,
                        body,
                    },
                    span,
                ))
            }
            syn::StmtKind::ForEach {
                variable,
                iterable,
                body,
            } => {
                let iterable = self.build_expr(iterable)?;
                let body_items = self.build_items_as_block(body)?;
                let body = self
                    .module
                    .arena
                    .add_statement(StatementKind::Block(body_items), span);
                Ok(self.module.arena.add_statement(
                    StatementKind::ForEach {
                        variable: self.intern(&variable),
                        iterable,
                        body,
                    },
                    span,
                ))
            }
            syn::StmtKind::Thread { body } => {
                let body_items = self.build_items_as_block(body)?;
                let body = self
                    .module
                    .arena
                    .add_statement(StatementKind::Block(body_items), span);
                Ok(self
                    .module
                    .arena
                    .add_statement(StatementKind::Thread { body }, span))
            }
            syn::StmtKind::Try { body, handlers } => {
                let body_items = self.build_items_as_block(body)?;
                let body = self
                    .module
                    .arena
                    .add_statement(StatementKind::Block(body_items), span);
                let handlers = self.build_handlers(handlers)?;
                Ok(self
                    .module
                    .arena
                    .add_statement(StatementKind::Try { body, handlers }, span))
            }
            syn::StmtKind::Raise {
                error_type,
                message,
            } => {
                let error_type = self.intern(&error_type);
                if !self.module.classes.contains_key(&error_type) {
                    let name = self
                        .module
                        .arena
                        .resolve_symbol(&self.interner, error_type)
                        .unwrap_or_default();
                    return Err(ParseError::TypeError(ErrorData::new(
                        span,
                        format!("Класс ошибки '{}' не найден", name),
                    )));
                }
                let message = message.map(|expr| self.build_expr(expr)).transpose()?;
                Ok(self.module.arena.add_statement(
                    StatementKind::Raise {
                        error_type,
                        message,
                    },
                    span,
                ))
            }
            syn::StmtKind::Return(expr) => {
                let expr = expr.map(|expr| self.build_expr(expr)).transpose()?;
                Ok(self
                    .module
                    .arena
                    .add_statement(StatementKind::Return(expr), span))
            }
            syn::StmtKind::Expr(expr) => {
                let expr = self.build_expr(expr)?;
                Ok(self
                    .module
                    .arena
                    .add_statement(StatementKind::Expression(expr), span))
            }
        }
    }

    fn build_target_assignment(
        &mut self,
        target: ExprId,
        value: ExprId,
        span: Span,
    ) -> Result<StmtId, ParseError> {
        let target_expr = self.module.arena.get_expression(target).ok_or_else(|| {
            ParseError::InvalidSyntax(ErrorData::new(span, "Не найдена нода для выражения".into()))
        })?;
        match target_expr.kind {
            ExpressionKind::PropertyAccess { object, property } => {
                Ok(self.module.arena.add_statement(
                    StatementKind::PropertyAssign {
                        object,
                        property,
                        value,
                    },
                    span,
                ))
            }
            ExpressionKind::Index { object, index } => Ok(self.module.arena.add_statement(
                StatementKind::IndexAssign {
                    object,
                    index,
                    value,
                },
                span,
            )),
            _ => Err(ParseError::InvalidSyntax(ErrorData::new(
                span,
                "Левая часть присваивания должна быть переменной, полем объекта или индексом списка"
                    .into(),
            ))),
        }
    }

    fn build_for_update(&mut self, update: syn::ForUpdate) -> Result<StmtId, ParseError> {
        match update {
            syn::ForUpdate::Assign { name, value, span } => {
                let span = self.span(span);
                let value = self.build_expr(value)?;
                Ok(self.module.arena.add_statement(
                    StatementKind::Assign {
                        name: self.intern(&name),
                        is_const: false,
                        type_hint: None,
                        value,
                    },
                    span,
                ))
            }
            syn::ForUpdate::AssignTarget {
                target,
                value,
                span,
            } => {
                let span = self.span(span);
                let target = self.build_expr(target)?;
                let value = self.build_expr(value)?;
                self.build_target_assignment(target, value, span)
            }
            syn::ForUpdate::Compound {
                target,
                op,
                value,
                span,
            } => {
                let span = self.span(span);
                let target = self.build_expr(target)?;
                let value = self.build_expr(value)?;
                Ok(self.module.arena.add_statement(
                    StatementKind::CompoundAssign {
                        target,
                        op: self.compound_op(op),
                        value,
                    },
                    span,
                ))
            }
            syn::ForUpdate::Expr(expr) => {
                let span = self.span(expr.span.clone());
                let expr = self.build_expr(expr)?;
                Ok(self
                    .module
                    .arena
                    .add_statement(StatementKind::Expression(expr), span))
            }
        }
    }

    fn build_handlers(&mut self, handlers: Vec<syn::Catch>) -> Result<Vec<TryHandler>, ParseError> {
        let mut output = Vec::new();
        for handler in handlers {
            let mut error_type = None;
            let mut error_text = None;
            if let Some(pattern) = handler.pattern {
                match pattern {
                    syn::CatchPattern::Text(name, _) => {
                        error_text = Some(self.intern(&name));
                    }
                    syn::CatchPattern::Type(name, span) => {
                        let symbol = self.intern(&name);
                        if self.module.classes.contains_key(&symbol) {
                            error_type = Some(symbol);
                        } else {
                            error_text = Some(symbol);
                        }
                        let _ = span;
                    }
                    syn::CatchPattern::TypeAndText {
                        type_name,
                        type_span,
                        text_name,
                        ..
                    } => {
                        let type_symbol = self.intern(&type_name);
                        if !self.module.classes.contains_key(&type_symbol) {
                            return Err(ParseError::TypeError(ErrorData::new(
                                self.span(type_span),
                                format!("Класс ошибки '{}' не найден", type_name),
                            )));
                        }
                        error_type = Some(type_symbol);
                        error_text = Some(self.intern(&text_name));
                    }
                }
            }
            let body_span = self.span(handler.span);
            let body_items = self.build_items_as_block(handler.body)?;
            let body = self
                .module
                .arena
                .add_statement(StatementKind::Block(body_items), body_span);
            output.push(TryHandler {
                error_type,
                error_text,
                body,
            });
        }
        Ok(output)
    }
}
