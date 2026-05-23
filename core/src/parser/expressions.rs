use crate::ast::prelude::*;
use crate::parser::parser::Rule;
use crate::parser::prelude::{ParseError, Parser as ParserTrait};
use pest::iterators::{Pair, Pairs};

const EXPECTED_EXPRESSION: &str = "Ожидалось выражение";

impl ParserTrait {
    fn syntax_error(&self, span: Span, message: impl Into<String>) -> ParseError {
        ParseError::InvalidSyntax(ErrorData::new(span, message.into()))
    }

    fn expected_expression(&self, span: Span) -> ParseError {
        self.syntax_error(span, EXPECTED_EXPRESSION)
    }

    fn next_required<'a>(
        &self,
        inner: &mut Pairs<'a, Rule>,
        span: Span,
    ) -> Result<Pair<'a, Rule>, ParseError> {
        inner.next().ok_or_else(|| self.expected_expression(span))
    }

    pub(crate) fn parse_expr_stmt(
        &mut self,
        pair: pest::iterators::Pair<Rule>,
    ) -> Result<StmtId, ParseError> {
        let expr_span = (pair.as_span(), self.module.name).into();
        let inner = pair.into_inner();

        for token in inner {
            if token.as_rule() == Rule::expression {
                let expr = self.parse_expression(token)?;
                let stmt_id = self
                    .module
                    .arena
                    .add_statement(StatementKind::Expression(expr), expr_span);
                return Ok(stmt_id);
            }
        }

        Err(self.expected_expression(expr_span))
    }

    pub(crate) fn parse_expression(
        &mut self,
        pair: pest::iterators::Pair<Rule>,
    ) -> Result<ExprId, ParseError> {
        let expr_span = (pair.as_span(), self.module.name).into();
        let mut inner = pair.into_inner();

        if let Some(first_token) = inner.next() {
            return self.parse_logical_or(first_token);
        }
        Err(self.expected_expression(expr_span))
    }

    pub(crate) fn parse_logical_or(
        &mut self,
        pair: pest::iterators::Pair<Rule>,
    ) -> Result<ExprId, ParseError> {
        let or_span = (pair.as_span(), self.module.name).into();
        let mut inner = pair.into_inner();
        let first = self.next_required(&mut inner, or_span)?;
        let mut left = self.parse_logical_and(first)?;

        while let Some(token) = inner.next() {
            if token.as_rule() == Rule::logical_or_op {
                let next = self.next_required(&mut inner, or_span)?;
                let right = self.parse_logical_and(next)?;
                left = self.module.arena.add_expression(
                    ExpressionKind::Binary {
                        op: BinaryOperator::Or,
                        left,
                        right,
                    },
                    or_span,
                );
            }
        }

        Ok(left)
    }

    pub(crate) fn parse_logical_and(
        &mut self,
        pair: pest::iterators::Pair<Rule>,
    ) -> Result<ExprId, ParseError> {
        let and_span = (pair.as_span(), self.module.name).into();
        let mut inner = pair.into_inner();
        let first = self.next_required(&mut inner, and_span)?;
        let mut left = self.parse_comparison(first)?;

        while let Some(token) = inner.next() {
            if token.as_rule() == Rule::logical_and_op {
                let next = self.next_required(&mut inner, and_span)?;
                let right = self.parse_comparison(next)?;
                left = self.module.arena.add_expression(
                    ExpressionKind::Binary {
                        op: BinaryOperator::And,
                        left,
                        right,
                    },
                    and_span,
                );
            }
        }

        Ok(left)
    }

    pub(crate) fn parse_comparison(
        &mut self,
        pair: pest::iterators::Pair<Rule>,
    ) -> Result<ExprId, ParseError> {
        let cmp_span = (pair.as_span(), self.module.name).into();
        let mut inner = pair.into_inner();
        let first = self.next_required(&mut inner, cmp_span)?;
        let mut left = self.parse_addition(first)?;

        while let Some(token) = inner.next() {
            if token.as_rule() == Rule::comp_op {
                let op = match token.as_str() {
                    "<=" => BinaryOperator::Le,
                    ">=" => BinaryOperator::Ge,
                    "==" => BinaryOperator::Eq,
                    "!=" => BinaryOperator::Ne,
                    "<" => BinaryOperator::Lt,
                    ">" => BinaryOperator::Gt,
                    _ => {
                        return Err(ParseError::InvalidSyntax(ErrorData::new(
                            cmp_span,
                            "Не поддерживаемая операция".into(),
                        )))
                    }
                };
                let next = self.next_required(&mut inner, cmp_span)?;
                let right = self.parse_addition(next)?;
                left = self
                    .module
                    .arena
                    .add_expression(ExpressionKind::Binary { op, left, right }, cmp_span);
            }
        }

        Ok(left)
    }

    pub(crate) fn parse_addition(
        &mut self,
        pair: pest::iterators::Pair<Rule>,
    ) -> Result<ExprId, ParseError> {
        let add_span = (pair.as_span(), self.module.name).into();
        let mut inner = pair.into_inner();
        let first = self.next_required(&mut inner, add_span)?;
        let mut left = self.parse_multiplication(first)?;

        while let Some(token) = inner.next() {
            if token.as_rule() == Rule::add_op {
                let op = match token.as_str() {
                    "+" => BinaryOperator::Add,
                    "-" => BinaryOperator::Sub,
                    _ => {
                        return Err(ParseError::InvalidSyntax(ErrorData::new(
                            add_span,
                            "Не поддерживаемая операция".into(),
                        )))
                    }
                };
                let next = self.next_required(&mut inner, add_span)?;
                let right = self.parse_multiplication(next)?;
                left = self
                    .module
                    .arena
                    .add_expression(ExpressionKind::Binary { op, left, right }, add_span);
            }
        }

        Ok(left)
    }

    pub(crate) fn parse_multiplication(
        &mut self,
        pair: pest::iterators::Pair<Rule>,
    ) -> Result<ExprId, ParseError> {
        let mul_span = (pair.as_span(), self.module.name).into();
        let mut inner = pair.into_inner();
        let first = self.next_required(&mut inner, mul_span)?;
        let mut left = self.parse_unary(first)?;

        while let Some(token) = inner.next() {
            if token.as_rule() == Rule::mul_op {
                let op = match token.as_str() {
                    "*" => BinaryOperator::Mul,
                    "/" => BinaryOperator::Div,
                    "%" => BinaryOperator::Mod,
                    _ => {
                        return Err(ParseError::InvalidSyntax(ErrorData::new(
                            mul_span,
                            "Не поддерживаемая операция".into(),
                        )))
                    }
                };
                let next = self.next_required(&mut inner, mul_span)?;
                let right = self.parse_unary(next)?;
                left = self
                    .module
                    .arena
                    .add_expression(ExpressionKind::Binary { op, left, right }, mul_span);
            }
        }

        Ok(left)
    }

    pub(crate) fn parse_unary(
        &mut self,
        pair: pest::iterators::Pair<Rule>,
    ) -> Result<ExprId, ParseError> {
        let unary_span = (pair.as_span(), self.module.name).into();
        let inner = pair.into_inner();

        let mut unary_op = None;
        for token in inner {
            match token.as_rule() {
                Rule::unary_op => {
                    unary_op = match token.as_str() {
                        "-" => Some(UnaryOperator::Negative),
                        "!" => Some(UnaryOperator::Not),
                        _ => None,
                    };
                }
                Rule::postfix => {
                    let mut expr = self.parse_postfix(token)?;
                    if let Some(op) = unary_op {
                        expr = self.module.arena.add_expression(
                            ExpressionKind::Unary { op, operand: expr },
                            unary_span,
                        );
                    }
                    return Ok(expr);
                }
                _ => {}
            }
        }

        Err(self.expected_expression(unary_span))
    }

    pub(crate) fn parse_postfix(
        &mut self,
        pair: pest::iterators::Pair<Rule>,
    ) -> Result<ExprId, ParseError> {
        let expr_span = (pair.as_span(), self.module.name).into();
        let mut inner = pair.into_inner();
        let first = self.next_required(&mut inner, expr_span)?;
        let mut expr = self.parse_primary(first)?;

        for token in inner {
            let postfix_span = (token.as_span(), self.module.name).into();
            match token.as_rule() {
                Rule::function_call => {
                    let mut args = Vec::new();
                    for arg_pair in token.into_inner() {
                        if arg_pair.as_rule() == Rule::arg_list {
                            args = self.parse_arg_list(arg_pair)?;
                        }
                    }

                    expr = self.module.arena.add_expression(
                        ExpressionKind::FunctionCall {
                            function: expr,
                            args,
                        },
                        postfix_span,
                    );
                }
                Rule::method_call => {
                    let mut method_inner = token.into_inner();
                    let method_name_str = method_inner
                        .next()
                        .ok_or_else(|| {
                            ParseError::InvalidSyntax(ErrorData::new(
                                postfix_span,
                                "Ожидалось выражение".into(),
                            ))
                        })?
                        .as_str()
                        .to_string();
                    let method_name = self
                        .module
                        .arena
                        .intern_string(&self.interner, &method_name_str);

                    let mut args = Vec::new();
                    if let Some(arg_list) = method_inner.next() {
                        if arg_list.as_rule() == Rule::arg_list {
                            args = self.parse_arg_list(arg_list)?;
                        }
                    }

                    expr = self.module.arena.add_expression(
                        ExpressionKind::MethodCall {
                            object: expr,
                            method: method_name,
                            args,
                        },
                        postfix_span,
                    );
                }
                Rule::property_access => {
                    let prop_name_str = token
                        .into_inner()
                        .next()
                        .ok_or_else(|| {
                            ParseError::InvalidSyntax(ErrorData::new(
                                postfix_span,
                                "Ожидалось выражение".into(),
                            ))
                        })?
                        .as_str()
                        .to_string();
                    let prop_name = self
                        .module
                        .arena
                        .intern_string(&self.interner, &prop_name_str);
                    expr = self.module.arena.add_expression(
                        ExpressionKind::PropertyAccess {
                            object: expr,
                            property: prop_name,
                        },
                        postfix_span,
                    );
                }
                Rule::index_access => {
                    let index_expr =
                        self.parse_expression(token.into_inner().next().ok_or_else(|| {
                            ParseError::InvalidSyntax(ErrorData::new(
                                postfix_span,
                                "Ожидалось выражение".into(),
                            ))
                        })?)?;
                    expr = self.module.arena.add_expression(
                        ExpressionKind::Index {
                            object: expr,
                            index: index_expr,
                        },
                        postfix_span,
                    );
                }
                _ => {}
            }
        }

        Ok(expr)
    }

    pub(crate) fn parse_primary(
        &mut self,
        pair: pest::iterators::Pair<Rule>,
    ) -> Result<ExprId, ParseError> {
        let primary_span = (pair.as_span(), self.module.name).into();
        match pair.as_rule() {
            Rule::lambda_expr => self.parse_lambda_expr(pair),
            Rule::paren_expr => {
                let mut inner = pair.into_inner();
                let expr = self.parse_expression(inner.next().ok_or_else(|| {
                    ParseError::InvalidSyntax(ErrorData::new(
                        primary_span,
                        "Ожидалось выражение".into(),
                    ))
                })?)?;
                Ok(expr)
            }
            Rule::new_expr => {
                let mut inner = pair.into_inner();
                let qualified_name_pair = inner.next().ok_or_else(|| {
                    ParseError::InvalidSyntax(ErrorData::new(
                        primary_span,
                        "Ожидалось выражение".into(),
                    ))
                })?;

                let class_name_str = if qualified_name_pair.as_rule() == Rule::qualified_name {
                    let mut parts = Vec::new();
                    for ident in qualified_name_pair.into_inner() {
                        if ident.as_rule() == Rule::identifier {
                            parts.push(ident.as_str().to_string());
                        }
                    }
                    parts.join(".")
                } else {
                    qualified_name_pair.as_str().to_string()
                };

                let class_name = self
                    .module
                    .arena
                    .intern_string(&self.interner, &class_name_str);

                let mut args = Vec::new();
                if let Some(arg_list) = inner.next() {
                    if arg_list.as_rule() == Rule::arg_list {
                        args = self.parse_arg_list(arg_list)?;
                    }
                }

                Ok(self.module.arena.add_expression(
                    ExpressionKind::ObjectCreation { class_name, args },
                    primary_span,
                ))
            }
            Rule::string_literal => {
                let s = pair.as_str();
                let trimmed = &s[1..s.len() - 1];
                let text_symbol = self.module.arena.intern_string(
                    &self.interner,
                    trimmed
                        .replace("\\n", "\n")
                        .replace("\\t", "\t")
                        .replace("\\\"", "\"")
                        .replace("\\\\", "\\")
                        .as_str(),
                );
                Ok(self.module.arena.add_expression(
                    ExpressionKind::Literal(LiteralValue::Text(text_symbol)),
                    primary_span,
                ))
            }
            Rule::number_literal => {
                let s = pair.as_str();
                if s.contains('.') {
                    if let Ok(num) = s.parse::<f64>() {
                        Ok(self.module.arena.add_expression(
                            ExpressionKind::Literal(LiteralValue::Float(num)),
                            primary_span,
                        ))
                    } else {
                        Err(ParseError::InvalidSyntax(ErrorData::new(
                            primary_span,
                            "Ожидалось выражение".into(),
                        )))
                    }
                } else if let Ok(num) = s.parse::<i64>() {
                    Ok(self.module.arena.add_expression(
                        ExpressionKind::Literal(LiteralValue::Number(num)),
                        primary_span,
                    ))
                } else {
                    Err(ParseError::InvalidSyntax(ErrorData::new(
                        primary_span,
                        "Ожидалось выражение".into(),
                    )))
                }
            }
            Rule::identifier => {
                let name_str = pair.as_str().to_string();
                let name = self.module.arena.intern_string(&self.interner, &name_str);
                Ok(self
                    .module
                    .arena
                    .add_expression(ExpressionKind::Identifier(name), primary_span))
            }
            Rule::bool_literal => {
                let s = pair.as_str();
                let boolean_val = s == "истина";
                Ok(self.module.arena.add_expression(
                    ExpressionKind::Literal(LiteralValue::Boolean(boolean_val)),
                    primary_span,
                ))
            }
            Rule::empty_literal => Ok(self
                .module
                .arena
                .add_expression(ExpressionKind::Literal(LiteralValue::Unit), primary_span)),
            _ => Err(ParseError::InvalidSyntax(ErrorData::new(
                primary_span,
                "Неожиданное выражение".into(),
            ))),
        }
    }

    fn parse_lambda_expr(&mut self, pair: Pair<Rule>) -> Result<ExprId, ParseError> {
        let lambda_span: Span = (pair.as_span(), self.module.name).into();
        let mut inner = pair.into_inner();
        let params_pair = inner.next().ok_or_else(|| {
            ParseError::InvalidSyntax(ErrorData::new(
                lambda_span,
                "Ожидались параметры лямбды".into(),
            ))
        })?;

        let mut params = Vec::new();
        for token in params_pair.into_inner() {
            if token.as_rule() == Rule::param_list {
                params = self.parse_param_list(token)?;
            }
        }

        let body_pair = inner.next().ok_or_else(|| {
            ParseError::InvalidSyntax(ErrorData::new(lambda_span, "Ожидалось тело лямбды".into()))
        })?;
        let body_span: Span = (body_pair.as_span(), self.module.name).into();
        let body = match body_pair.as_rule() {
            Rule::block => {
                let statements = self.parse_block(body_pair)?;
                self.module
                    .arena
                    .add_statement(StatementKind::Block(statements), body_span)
            }
            Rule::expression => {
                let expr = self.parse_expression(body_pair)?;
                let return_stmt = self
                    .module
                    .arena
                    .add_statement(StatementKind::Return(Some(expr)), body_span);
                self.module
                    .arena
                    .add_statement(StatementKind::Block(vec![return_stmt]), body_span)
            }
            _ => {
                return Err(ParseError::InvalidSyntax(ErrorData::new(
                    body_span,
                    "Ожидалось тело лямбды".into(),
                )))
            }
        };

        Ok(self
            .module
            .arena
            .add_expression(ExpressionKind::Lambda { params, body }, lambda_span))
    }

    pub(crate) fn parse_arg_list(
        &mut self,
        pair: pest::iterators::Pair<Rule>,
    ) -> Result<Vec<CallArg>, ParseError> {
        let mut args = Vec::new();
        let mut saw_named = false;

        for arg_pair in pair.into_inner() {
            let arg_span: Span = (arg_pair.as_span(), self.module.name).into();
            match arg_pair.as_rule() {
                Rule::named_arg => {
                    saw_named = true;
                    let mut inner = arg_pair.into_inner();
                    let name_token = inner.next().ok_or_else(|| {
                        ParseError::InvalidSyntax(ErrorData::new(
                            arg_span,
                            "Ожидалось имя аргумента".into(),
                        ))
                    })?;
                    let name_str = name_token.as_str().to_string();
                    let name = self.module.arena.intern_string(&self.interner, &name_str);

                    let value_token = inner.next().ok_or_else(|| {
                        ParseError::InvalidSyntax(ErrorData::new(
                            arg_span,
                            "Ожидалось выражение".into(),
                        ))
                    })?;
                    let value_expr = self.parse_expression(value_token)?;

                    args.push(CallArg {
                        name: Some(name),
                        value: value_expr,
                    });
                }
                Rule::expression => {
                    if saw_named {
                        return Err(ParseError::InvalidSyntax(ErrorData::new(
                            arg_span,
                            "Именованные аргументы должны идти после позиционных".into(),
                        )));
                    }
                    let value_expr = self.parse_expression(arg_pair)?;
                    args.push(CallArg {
                        name: None,
                        value: value_expr,
                    });
                }
                _ => {}
            }
        }

        Ok(args)
    }
}
