use crate::ast::prelude::*;
use crate::parser::parser::Rule;
use crate::parser::prelude::{ParseError, Parser as ParserTrait};

impl ParserTrait {
    pub(crate) fn parse_block(
        &mut self,
        pair: pest::iterators::Pair<Rule>,
    ) -> Result<Vec<StmtId>, ParseError> {
        let mut statements = Vec::new();

        for inner in pair.into_inner() {
            match inner.as_rule() {
                Rule::function => {
                    let stmt_id = self.parse_function(inner)?;
                    statements.push(stmt_id);
                }
                Rule::class => {
                    let stmt_id = self.parse_class(inner)?;
                    statements.push(stmt_id);
                }
                Rule::library_stmt => {
                    let stmt_id = self.parse_library_stmt(inner)?;
                    statements.push(stmt_id);
                }
                Rule::assignment => {
                    let stmt_id = self.parse_assignment(inner)?;
                    statements.push(stmt_id);
                }
                Rule::compound_assignment => {
                    let stmt_id = self.parse_compound_assignment(inner)?;
                    statements.push(stmt_id);
                }
                Rule::property_assign => {
                    let stmt_id = self.parse_property_assign(inner)?;
                    statements.push(stmt_id);
                }
                Rule::if_stmt => {
                    let stmt_id = self.parse_if_stmt(inner)?;
                    statements.push(stmt_id);
                }
                Rule::try_stmt => {
                    let stmt_id = self.parse_try_stmt(inner)?;
                    statements.push(stmt_id);
                }
                Rule::raise_stmt => {
                    let stmt_id = self.parse_raise_stmt(inner)?;
                    statements.push(stmt_id);
                }
                Rule::while_stmt => {
                    let stmt_id = self.parse_while_stmt(inner)?;
                    statements.push(stmt_id);
                }
                Rule::for_stmt => {
                    let stmt_id = self.parse_for_stmt(inner)?;
                    statements.push(stmt_id);
                }
                Rule::thread_stmt => {
                    let stmt_id = self.parse_thread_stmt(inner)?;
                    statements.push(stmt_id);
                }
                Rule::return_stmt => {
                    let stmt_id = self.parse_return_stmt(inner)?;
                    statements.push(stmt_id);
                }
                Rule::expr_stmt => {
                    let stmt_id = self.parse_expr_stmt(inner)?;
                    statements.push(stmt_id);
                }
                _ => {}
            }
        }

        Ok(statements)
    }

    pub(crate) fn parse_assignment(
        &mut self,
        pair: pest::iterators::Pair<Rule>,
    ) -> Result<StmtId, ParseError> {
        let assignment_span: Span = (pair.as_span(), self.module.name).into();
        let mut inner = pair.into_inner();
        let name_str = inner
            .next()
            .ok_or_else(|| {
                ParseError::InvalidSyntax(ErrorData::new(
                    assignment_span,
                    "Ожидалось имя переменной".into(),
                ))
            })?
            .as_str()
            .to_string();
        let name = self.module.arena.intern_string(&self.interner, &name_str);

        let mut type_hint = None;
        let mut value = None;

        for token in inner {
            match token.as_rule() {
                Rule::type_hint => {
                    if let Some(type_token) = token.into_inner().next() {
                        let type_span = (type_token.as_span(), self.module.name).into();
                        let type_str = type_token.as_str();
                        type_hint = Some(
                            self.module
                                .arena
                                .find_type_by_name(&self.interner, type_str)
                                .ok_or_else(|| {
                                    ParseError::TypeError(ErrorData::new(
                                        type_span,
                                        format!("Неизвестный тип: {}", type_str),
                                    ))
                                })?,
                        );
                    }
                }
                Rule::expression => {
                    value = Some(self.parse_expression(token)?);
                }
                _ => {}
            }
        }

        let stmt_id = self.module.arena.add_statement(
            StatementKind::Assign {
                name,
                type_hint,
                value: value.ok_or_else(|| {
                    ParseError::TypeError(ErrorData::new(
                        assignment_span,
                        format!("Отсутствует значение у переменной: {}", name_str),
                    ))
                })?,
            },
            assignment_span,
        );
        Ok(stmt_id)
    }

    pub(crate) fn parse_compound_assignment(
        &mut self,
        pair: pest::iterators::Pair<Rule>,
    ) -> Result<StmtId, ParseError> {
        let span: Span = (pair.as_span(), self.module.name).into();
        let compound_pair = pair.into_inner().next().ok_or_else(|| {
            ParseError::InvalidSyntax(ErrorData::new(span, "Ожидалось присваивание".into()))
        })?;

        self.parse_compound_assign_pair(compound_pair, span)
    }

    fn parse_compound_assign_pair(
        &mut self,
        pair: pest::iterators::Pair<Rule>,
        span: Span,
    ) -> Result<StmtId, ParseError> {
        let mut inner = pair.into_inner();
        let target_pair = inner.next().ok_or_else(|| {
            ParseError::InvalidSyntax(ErrorData::new(
                span,
                "Ожидалась левая часть присваивания".into(),
            ))
        })?;
        let target = self.parse_postfix(target_pair)?;

        let op_token = inner.next().ok_or_else(|| {
            ParseError::InvalidSyntax(ErrorData::new(
                span,
                "Ожидался оператор присваивания".into(),
            ))
        })?;
        let op = match op_token.as_str() {
            "+=" => BinaryOperator::Add,
            "-=" => BinaryOperator::Sub,
            "*=" => BinaryOperator::Mul,
            "/=" => BinaryOperator::Div,
            "%=" => BinaryOperator::Mod,
            _ => {
                return Err(ParseError::InvalidSyntax(ErrorData::new(
                    span,
                    "Неподдерживаемый оператор присваивания".into(),
                )))
            }
        };

        let value = self.parse_expression(inner.next().ok_or_else(|| {
            ParseError::InvalidSyntax(ErrorData::new(span, "Ожидалось выражение".into()))
        })?)?;

        let target_expr = self.module.arena.get_expression(target).ok_or_else(|| {
            ParseError::InvalidSyntax(ErrorData::new(span, "Не найдена нода для выражения".into()))
        })?;

        match target_expr.kind {
            ExpressionKind::Identifier(_)
            | ExpressionKind::PropertyAccess { .. }
            | ExpressionKind::Index { .. } => Ok(self
                .module
                .arena
                .add_statement(StatementKind::CompoundAssign { target, op, value }, span)),
            _ => Err(ParseError::InvalidSyntax(ErrorData::new(
                span,
                "Левая часть составного присваивания должна быть переменной, полем или индексом"
                    .into(),
            ))),
        }
    }

    pub(crate) fn parse_property_assign(
        &mut self,
        pair: pest::iterators::Pair<Rule>,
    ) -> Result<StmtId, ParseError> {
        let property_span: Span = (pair.as_span(), self.module.name).into();
        let mut inner = pair.into_inner();

        let postfix_pair = inner.next().ok_or_else(|| {
            ParseError::InvalidSyntax(ErrorData::new(property_span, "Ожидалось выражение".into()))
        })?;

        let postfix_expr = self.parse_postfix(postfix_pair)?;

        let value_expr = self.parse_expression(inner.next().ok_or_else(|| {
            ParseError::InvalidSyntax(ErrorData::new(property_span, "Ожидалось выражение".into()))
        })?)?;

        let expr_k = self
            .module
            .arena
            .get_expression(postfix_expr)
            .ok_or_else(|| {
                ParseError::InvalidSyntax(ErrorData::new(
                    property_span,
                    "Не найдена нода для выражения".into(),
                ))
            })?;

        match expr_k.kind {
            ExpressionKind::PropertyAccess { object, property } => {
                Ok(self.module.arena.add_statement(
                    StatementKind::PropertyAssign {
                        object,
                        property,
                        value: value_expr,
                    },
                    property_span,
                ))
            }

            ExpressionKind::Index { object, index } => Ok(self.module.arena.add_statement(
                StatementKind::IndexAssign {
                    object,
                    index,
                    value: value_expr,
                },
                property_span,
            )),

            _ => Err(ParseError::InvalidSyntax(ErrorData::new(
                property_span,
                "Левая часть присваивания должна быть полем объекта или индексом списка".into(),
            ))),
        }
    }

    pub(crate) fn parse_if_stmt(
        &mut self,
        pair: pest::iterators::Pair<Rule>,
    ) -> Result<StmtId, ParseError> {
        let if_stmt_span: Span = (pair.as_span(), self.module.name).into();
        let mut inner = pair.into_inner();

        let condition = self.parse_expression(inner.next().ok_or_else(|| {
            ParseError::InvalidSyntax(ErrorData::new(if_stmt_span, "Ожидалось выражение".into()))
        })?)?;

        let then_block = self.parse_block(inner.next().ok_or_else(|| {
            ParseError::InvalidSyntax(ErrorData::new(if_stmt_span, "Ожидалось выражение".into()))
        })?)?;
        let then_body = self
            .module
            .arena
            .add_statement(StatementKind::Block(then_block), if_stmt_span);

        let mut else_body = None;

        if let Some(else_clause) = inner.next() {
            if else_clause.as_rule() == Rule::else_clause {
                let mut clause_inner = else_clause.into_inner();

                if let Some(else_content) = clause_inner.next() {
                    let else_span = (else_content.as_span(), self.module.name).into();
                    match else_content.as_rule() {
                        Rule::else_if_clause => {
                            if let Some(if_stmt) = else_content.into_inner().next() {
                                else_body = Some(self.parse_if_stmt(if_stmt)?);
                            }
                        }
                        Rule::block => {
                            let else_block = self.parse_block(else_content)?;
                            else_body = Some(
                                self.module
                                    .arena
                                    .add_statement(StatementKind::Block(else_block), else_span),
                            );
                        }
                        _ => {}
                    }
                }
            }
        }

        let stmt_id = self.module.arena.add_statement(
            StatementKind::If {
                condition,
                then_body,
                else_body,
            },
            if_stmt_span,
        );
        Ok(stmt_id)
    }

    pub(crate) fn parse_try_stmt(
        &mut self,
        pair: pest::iterators::Pair<Rule>,
    ) -> Result<StmtId, ParseError> {
        let try_span: Span = (pair.as_span(), self.module.name).into();
        let mut inner = pair.into_inner();

        let try_block = inner.next().ok_or_else(|| {
            ParseError::InvalidSyntax(ErrorData::new(try_span, "Ожидался блок попробовать".into()))
        })?;
        let try_stmts = self.parse_block(try_block)?;
        let body = self
            .module
            .arena
            .add_statement(StatementKind::Block(try_stmts), try_span);

        let mut handlers = Vec::new();
        for handler in inner {
            if handler.as_rule() != Rule::catch_clause {
                continue;
            }

            let handler_span: Span = (handler.as_span(), self.module.name).into();
            let mut error_type = None;
            let mut error_text = None;
            let mut block_pair = None;

            for token in handler.into_inner() {
                match token.as_rule() {
                    Rule::catch_pattern => {
                        let identifiers: Vec<_> = token
                            .into_inner()
                            .filter(|inner| inner.as_rule() == Rule::identifier)
                            .collect();

                        match identifiers.as_slice() {
                            [] => {}
                            [single] => {
                                let name = single.as_str();
                                let symbol = self.module.arena.intern_string(&self.interner, name);
                                if self.module.classes.contains_key(&symbol) {
                                    error_type = Some(symbol);
                                } else {
                                    error_text = Some(symbol);
                                }
                            }
                            [class_token, text_token] => {
                                let class_name = class_token.as_str();
                                let class_symbol =
                                    self.module.arena.intern_string(&self.interner, class_name);
                                if !self.module.classes.contains_key(&class_symbol) {
                                    return Err(ParseError::TypeError(ErrorData::new(
                                        (class_token.as_span(), self.module.name).into(),
                                        format!("Класс ошибки '{}' не найден", class_name),
                                    )));
                                }
                                error_type = Some(class_symbol);
                                error_text = Some(
                                    self.module
                                        .arena
                                        .intern_string(&self.interner, text_token.as_str()),
                                );
                            }
                            _ => {
                                return Err(ParseError::InvalidSyntax(ErrorData::new(
                                    handler_span,
                                    "Некорректный перехватчик ошибки".into(),
                                )));
                            }
                        }
                    }
                    Rule::block => block_pair = Some(token),
                    _ => {}
                }
            }

            let block_pair = block_pair.ok_or_else(|| {
                ParseError::InvalidSyntax(ErrorData::new(
                    handler_span,
                    "Ожидался блок перехватить".into(),
                ))
            })?;
            let block_stmts = self.parse_block(block_pair)?;
            let handler_body = self
                .module
                .arena
                .add_statement(StatementKind::Block(block_stmts), handler_span);
            handlers.push(TryHandler {
                error_type,
                error_text,
                body: handler_body,
            });
        }

        if handlers.is_empty() {
            return Err(ParseError::InvalidSyntax(ErrorData::new(
                try_span,
                "Ожидался хотя бы один блок перехватить".into(),
            )));
        }

        Ok(self
            .module
            .arena
            .add_statement(StatementKind::Try { body, handlers }, try_span))
    }

    pub(crate) fn parse_raise_stmt(
        &mut self,
        pair: pest::iterators::Pair<Rule>,
    ) -> Result<StmtId, ParseError> {
        let raise_span: Span = (pair.as_span(), self.module.name).into();
        let mut inner = pair.into_inner();

        let error_token = inner.next().ok_or_else(|| {
            ParseError::InvalidSyntax(ErrorData::new(raise_span, "Ожидался класс ошибки".into()))
        })?;
        let error_name = error_token.as_str();
        let error_type = self.module.arena.intern_string(&self.interner, error_name);
        if !self.module.classes.contains_key(&error_type) {
            return Err(ParseError::TypeError(ErrorData::new(
                (error_token.as_span(), self.module.name).into(),
                format!("Класс ошибки '{}' не найден", error_name),
            )));
        }

        let message = if let Some(message_expr) = inner.next() {
            Some(self.parse_expression(message_expr)?)
        } else {
            None
        };

        Ok(self.module.arena.add_statement(
            StatementKind::Raise {
                error_type,
                message,
            },
            raise_span,
        ))
    }

    pub(crate) fn parse_while_stmt(
        &mut self,
        pair: pest::iterators::Pair<Rule>,
    ) -> Result<StmtId, ParseError> {
        let while_stmt_span: Span = (pair.as_span(), self.module.name).into();
        let mut inner = pair.into_inner();

        let condition = self.parse_expression(inner.next().ok_or_else(|| {
            ParseError::InvalidSyntax(ErrorData::new(
                while_stmt_span,
                "Ожидалось выражение".into(),
            ))
        })?)?;

        let block_stmts = self.parse_block(inner.next().ok_or_else(|| {
            ParseError::InvalidSyntax(ErrorData::new(
                while_stmt_span,
                "Ожидалось выражение".into(),
            ))
        })?)?;
        let body = self
            .module
            .arena
            .add_statement(StatementKind::Block(block_stmts), while_stmt_span);

        let stmt_id = self
            .module
            .arena
            .add_statement(StatementKind::While { condition, body }, while_stmt_span);
        Ok(stmt_id)
    }

    pub(crate) fn parse_for_stmt(
        &mut self,
        pair: pest::iterators::Pair<Rule>,
    ) -> Result<StmtId, ParseError> {
        let for_span = (pair.as_span(), self.module.name).into();
        let mut inner = pair.into_inner();

        let for_init = inner.next().ok_or_else(|| {
            ParseError::InvalidSyntax(ErrorData::new(for_span, "Ожидалось выражение".into()))
        })?;
        let mut init_inner = for_init.into_inner();
        let variable_str = init_inner
            .next()
            .ok_or_else(|| {
                ParseError::InvalidSyntax(ErrorData::new(for_span, "Ожидалось выражение".into()))
            })?
            .as_str()
            .to_string();
        let variable = self
            .module
            .arena
            .intern_string(&self.interner, &variable_str);
        let init_expr = self.parse_expression(init_inner.next().ok_or_else(|| {
            ParseError::InvalidSyntax(ErrorData::new(for_span, "Ожидалось выражение".into()))
        })?)?;

        let for_cond_token = inner.next().ok_or_else(|| {
            ParseError::InvalidSyntax(ErrorData::new(for_span, "Ожидалось выражение".into()))
        })?;
        let mut cond_inner = for_cond_token.into_inner();
        let cond_expr_token = cond_inner.next().ok_or_else(|| {
            ParseError::InvalidSyntax(ErrorData::new(for_span, "Ожидалось выражение".into()))
        })?;
        let condition_expr = self.parse_expression(cond_expr_token)?;

        let for_upd_token = inner.next().ok_or_else(|| {
            ParseError::InvalidSyntax(ErrorData::new(for_span, "Ожидалось выражение".into()))
        })?;

        let upd_span: Span = (for_upd_token.as_span(), self.module.name).into();
        let mut upd_inner = for_upd_token.into_inner();
        let first_upd_token = upd_inner.next().ok_or_else(|| {
            ParseError::InvalidSyntax(ErrorData::new(upd_span, "Ожидалось выражение".into()))
        })?;
        let update = match first_upd_token.as_rule() {
            Rule::compound_assign => self.parse_compound_assign_pair(first_upd_token, upd_span)?,
            Rule::assignment_expr => {
                let ae_span = (first_upd_token.as_span(), self.module.name).into();
                let mut ae_inner = first_upd_token.into_inner();
                let var_str = ae_inner
                    .next()
                    .ok_or_else(|| {
                        ParseError::InvalidSyntax(ErrorData::new(
                            ae_span,
                            "Ожидалось выражение".into(),
                        ))
                    })?
                    .as_str()
                    .to_string();
                let name = self.module.arena.intern_string(&self.interner, &var_str);
                let value = self.parse_expression(ae_inner.next().ok_or_else(|| {
                    ParseError::InvalidSyntax(ErrorData::new(ae_span, "Ожидалось выражение".into()))
                })?)?;
                self.module.arena.add_statement(
                    StatementKind::Assign {
                        name,
                        type_hint: None,
                        value,
                    },
                    ae_span,
                )
            }
            _ => {
                let value = self.parse_expression(first_upd_token)?;
                self.module.arena.add_statement(
                    StatementKind::Assign {
                        name: variable,
                        type_hint: None,
                        value,
                    },
                    upd_span,
                )
            }
        };

        let block_stmts = self.parse_block(inner.next().ok_or_else(|| {
            ParseError::InvalidSyntax(ErrorData::new(for_span, "Ожидалось выражение".into()))
        })?)?;
        let body = self
            .module
            .arena
            .add_statement(StatementKind::Block(block_stmts), for_span);

        let stmt_id = self.module.arena.add_statement(
            StatementKind::For {
                variable,
                init: init_expr,
                condition: condition_expr,
                update,
                body,
            },
            for_span,
        );
        Ok(stmt_id)
    }

    pub(crate) fn parse_return_stmt(
        &mut self,
        pair: pest::iterators::Pair<Rule>,
    ) -> Result<StmtId, ParseError> {
        let return_span = (pair.as_span(), self.module.name).into();
        let inner = pair.into_inner();

        let mut expr = None;
        for token in inner {
            if token.as_rule() == Rule::expression {
                expr = Some(self.parse_expression(token)?);
                break;
            }
        }

        let stmt_id = self
            .module
            .arena
            .add_statement(StatementKind::Return(expr), return_span);
        Ok(stmt_id)
    }

    pub(crate) fn parse_thread_stmt(
        &mut self,
        pair: pest::iterators::Pair<Rule>,
    ) -> Result<StmtId, ParseError> {
        let thread_span: Span = (pair.as_span(), self.module.name).into();
        let mut inner = pair.into_inner();
        let block_pair = inner.next().ok_or_else(|| {
            ParseError::InvalidSyntax(ErrorData::new(thread_span, "Ожидался блок потока".into()))
        })?;
        let block_stmts = self.parse_block(block_pair)?;
        let body = self
            .module
            .arena
            .add_statement(StatementKind::Block(block_stmts), thread_span);

        Ok(self
            .module
            .arena
            .add_statement(StatementKind::Thread { body }, thread_span))
    }
}
