use crate::ast::prelude::*;
use crate::interpreter::prelude::{Module, SharedInterner};
use crate::parser::prelude::{ParseError, Parser as ParserTrait};
use crate::shared::SharedMut;
use pest::error::ErrorVariant;
use pest::Parser;
use pest_derive::Parser;
use std::path::PathBuf;

#[derive(Parser)]
#[grammar = "grammar.pest"]
struct ProgramParser;

impl ParserTrait {
    pub fn new(interner: SharedInterner, name: &str, path: PathBuf) -> Self {
        Self {
            module: Module::new(&interner, name, path),
            interner,
            nesting_level: 0,
        }
    }

    pub fn parse(mut self, code: &str) -> Result<Module, ParseError> {
        let pairs = ProgramParser::parse(Rule::program, code).map_err(|e| {
            let message = match e.variant {
                ErrorVariant::ParsingError { .. } => "Проверьте правильность выражения".into(),
                ErrorVariant::CustomError { message } => message,
            };

            let (start, end) = match e.location {
                pest::error::InputLocation::Pos(pos) => (pos, pos),
                pest::error::InputLocation::Span((start, end)) => (start, end),
            };
            ParseError::UnexpectedToken(ErrorData::new(Span::new(start, end, self.module.name), message))
        })?;
        self.module.arena.init_builtin_types(&self.interner);
        for pair in pairs {
            if pair.as_rule() == Rule::program {
                for inner in pair.into_inner() {
                    match inner.as_rule() {
                        Rule::function => {
                            let stmt_id = self.parse_function(inner)?;
                            self.module.body.push(stmt_id);
                        }
                        Rule::class => {
                            let stmt_id = self.parse_class(inner)?;
                            self.module.body.push(stmt_id);
                        }
                        Rule::assignment => {
                            let stmt_id = self.parse_assignment(inner)?;
                            self.module.body.push(stmt_id);
                        }
                        Rule::property_assign => {
                            let stmt_id = self.parse_property_assign(inner)?;
                            self.module.body.push(stmt_id);
                        }
                        Rule::if_stmt => {
                            let stmt_id = self.parse_if_stmt(inner)?;
                            self.module.body.push(stmt_id);
                        }
                        Rule::while_stmt => {
                            let stmt_id = self.parse_while_stmt(inner)?;
                            self.module.body.push(stmt_id);
                        }
                        Rule::for_stmt => {
                            let stmt_id = self.parse_for_stmt(inner)?;
                            self.module.body.push(stmt_id);
                        }
                        Rule::return_stmt => {
                            let stmt_id = self.parse_return_stmt(inner)?;
                            self.module.body.push(stmt_id);
                        }
                        Rule::expr_stmt => {
                            let stmt_id = self.parse_expr_stmt(inner)?;
                            self.module.body.push(stmt_id);
                        }
                        Rule::import_stmt => {
                            let stmt_id = self.parse_import_stmt(inner)?;
                            self.module.body.push(stmt_id);
                        }
                        _ => {}
                    }
                }
            }
        }
        self.module.arena.optimize_all(&self.interner);
        Ok(self.module)
    }

    fn parse_function(&mut self, pair: pest::iterators::Pair<Rule>) -> Result<StmtId, ParseError> {
        let func_span: Span = (pair.as_span(), self.module.name).into();
        let mut inner = pair.into_inner();

        let name_token = inner.next().ok_or_else(|| {
            ParseError::InvalidSyntax(ErrorData::new(func_span, "Ожидалось имя функции".into()))
        })?;
        let name = name_token.as_str();
        let symbol_name = self.module.arena.intern_string(&self.interner, name);

        self.nesting_level += 1;

        let mut params = Vec::new();
        let mut return_type = None;

        while let Some(token) = inner.next() {
            let token_span: Span = (token.as_span(), self.module.name).into();
            match token.as_rule() {
                Rule::param_list => {
                    params = self.parse_param_list(token)?;
                }
                Rule::return_type => {
                    if let Some(type_token) = token.into_inner().next() {
                        let type_span: Span = (type_token.as_span(), self.module.name).into();
                        let type_str = type_token.as_str();
                        return_type = Some(
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
                Rule::block => {
                    let body = self.parse_block(token)?;
                    self.nesting_level -= 1;

                    let body_id = self
                        .module
                        .arena
                        .add_statement(StatementKind::Block(body), token_span);

                    let func_def = FunctionDefinition {
                        name: symbol_name,
                        params,
                        return_type,
                        body: body_id,
                        span: func_span,
                        module: None,
                    };
                    if self.nesting_level == 0 {
                        self.module.functions.insert(symbol_name, func_def);
                        return Ok(self
                            .module
                            .arena
                            .add_statement(StatementKind::Empty, func_span));
                    } else {
                        let stmt_id = self
                            .module
                            .arena
                            .add_statement(StatementKind::FunctionDefinition(func_def), func_span);
                        return Ok(stmt_id);
                    }
                }
                _ => {}
            }
        }
        self.nesting_level -= 1;
        Err(ParseError::InvalidSyntax(ErrorData::new(
            func_span,
            "Ожидалась функция".into(),
        )))
    }

    fn parse_class(&mut self, pair: pest::iterators::Pair<Rule>) -> Result<StmtId, ParseError> {
        let class_span: Span = (pair.as_span(), self.module.name).into();
        let mut inner = pair.into_inner();
        let name = inner
            .next()
            .ok_or_else(|| {
                ParseError::InvalidSyntax(ErrorData::new(class_span, "Ожидалось имя класса".into()))
            })?
            .as_str();
        self.module.arena.register_custom_type(&self.interner, name);
        let symbol_name = self.module.arena.intern_string(&self.interner, name);
        let mut class_def = ClassDefinition::new(symbol_name, class_span);

        while let Some(token) = inner.next() {
            match token.as_rule() {
                Rule::class_field => {
                    let field = self.parse_class_field(token)?;
                    class_def.add_field(
                        field.name,
                        field.visibility,
                        field.is_static,
                        field.default_value,
                    );
                }
                Rule::constructor => {
                    let mut method = self.parse_constructor(token)?;
                    method.is_constructor = true;
                    class_def.set_constructor(FunctionDefinition {
                        name: method.name,
                        params: method.params.clone(),
                        return_type: method.return_type,
                        body: method.body,
                        span: method.span,
                        module: None,
                    });
                }

                Rule::class_method => {
                    let method = self.parse_class_method(token)?;
                    class_def.add_method(
                        method.name,
                        method.visibility,
                        method.is_static,
                        FunctionDefinition {
                            name: method.name,
                            params: method.params,
                            return_type: method.return_type,
                            body: method.body,
                            span: method.span,
                            module: None,
                        },
                    );
                }
                _ => {}
            }
        }

        self.module
            .classes
            .insert(symbol_name, SharedMut::new(class_def.clone()));
        let stmt_id = self
            .module
            .arena
            .add_statement(StatementKind::ClassDefinition(class_def), class_span);
        Ok(stmt_id)
    }

    fn parse_class_field(
        &mut self,
        pair: pest::iterators::Pair<Rule>,
    ) -> Result<ClassField, ParseError> {
        let field_span: Span = (pair.as_span(), self.module.name).into();
        let mut inner = pair.into_inner();
        let mut visibility = Visibility::Private;
        let mut is_static = false;
        let mut field_name = String::new();
        let mut field_type = None;
        let mut default_value = None;

        while let Some(token) = inner.next() {
            match token.as_rule() {
                Rule::visibility => {
                    visibility = if token.as_str() == "публичный" {
                        Visibility::Public
                    } else {
                        Visibility::Private
                    };
                }
                Rule::static_mod => {
                    is_static = true;
                }
                Rule::identifier => {
                    field_name = token.as_str().to_string();
                }
                Rule::type_name => {
                    let type_span = (token.as_span(), self.module.name).into();
                    let type_str = token.as_str();
                    field_type = Some(
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
                Rule::expression => {
                    default_value = Some(self.parse_expression(token)?);
                }
                _ => {}
            }
        }

        Ok(ClassField {
            name: self.module.arena.intern_string(&self.interner, &field_name),
            field_type,
            visibility,
            is_static,
            default_value,
            span: field_span,
        })
    }

    fn parse_constructor(
        &mut self,
        pair: pest::iterators::Pair<Rule>,
    ) -> Result<ClassMethod, ParseError> {
        let constructor_span: Span = (pair.as_span(), self.module.name).into();
        let mut inner = pair.into_inner();
        let mut visibility = Visibility::Private;
        let mut is_static = false;
        let mut method_name = String::new();
        let mut params = Vec::new();
        let mut return_type = None;
        let mut body = None;

        while let Some(token) = inner.next() {
            let token_span: Span = (token.as_span(), self.module.name).into();
            match token.as_rule() {
                Rule::visibility => {
                    visibility = if token.as_str() == "публичный" {
                        Visibility::Public
                    } else {
                        Visibility::Private
                    };
                }
                Rule::static_mod => {
                    is_static = true;
                }
                Rule::identifier => {
                    method_name = token.as_str().to_string();
                }
                Rule::param_list => {
                    params = self.parse_param_list(token)?;
                }
                Rule::return_type => {
                    if let Some(type_token) = token.into_inner().next() {
                        let type_span = (type_token.as_span(), self.module.name).into();
                        let type_str = type_token.as_str();
                        return_type = Some(
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
                Rule::block => {
                    let block_stmts = self.parse_block(token)?;
                    body = Some(
                        self.module
                            .arena
                            .add_statement(StatementKind::Block(block_stmts), token_span),
                    );
                }
                _ => {}
            }
        }

        Ok(ClassMethod {
            name: self
                .module
                .arena
                .intern_string(&self.interner, &method_name),
            params,
            return_type,
            body: body.ok_or_else(|| {
                ParseError::InvalidSyntax(ErrorData::new(
                    constructor_span,
                    "Ожидалось тело метода".into(),
                ))
            })?,
            visibility,
            is_static,
            is_constructor: false,
            span: constructor_span,
        })
    }

    fn parse_class_method(
        &mut self,
        pair: pest::iterators::Pair<Rule>,
    ) -> Result<ClassMethod, ParseError> {
        let method_span: Span = (pair.as_span(), self.module.name).into();
        let mut inner = pair.into_inner();
        let mut visibility = Visibility::Private;
        let mut is_static = false;
        let mut method_name = String::new();
        let mut params = Vec::new();
        let mut return_type = None;
        let mut body = None;

        while let Some(token) = inner.next() {
            let token_span: Span =(token.as_span(), self.module.name).into();
            match token.as_rule() {
                Rule::visibility => {
                    visibility = if token.as_str() == "публичный" {
                        Visibility::Public
                    } else {
                        Visibility::Private
                    };
                }
                Rule::static_mod => {
                    is_static = true;
                }
                Rule::identifier => {
                    method_name = token.as_str().to_string();
                }
                Rule::param_list => {
                    params = self.parse_param_list(token)?;
                }
                Rule::return_type => {
                    if let Some(type_token) = token.into_inner().next() {
                        let type_span = (type_token.as_span(), self.module.name).into();
                        let type_str = type_token.as_str();
                        return_type = Some(
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
                Rule::block => {
                    let block_stmts = self.parse_block(token)?;
                    body = Some(
                        self.module
                            .arena
                            .add_statement(StatementKind::Block(block_stmts), token_span),
                    );
                }
                _ => {}
            }
        }

        Ok(ClassMethod {
            name: self
                .module
                .arena
                .intern_string(&self.interner, &method_name),
            params,
            return_type,
            body: body.ok_or_else(|| {
                ParseError::InvalidSyntax(ErrorData::new(
                    method_span,
                    "Ожидалось тело метода".into(),
                ))
            })?,
            visibility,
            is_static,
            is_constructor: false,
            span: method_span,
        })
    }

    fn parse_param_list(
        &mut self,
        pair: pest::iterators::Pair<Rule>,
    ) -> Result<Vec<Parameter>, ParseError> {
        let mut params = Vec::new();

        for param_pair in pair.into_inner() {
            if param_pair.as_rule() == Rule::param {
                let token_span: Span = (param_pair.as_span(), self.module.name).into();
                let mut param_inner = param_pair.into_inner();
                let name = param_inner
                    .next()
                    .map(|p| p.as_str().to_string())
                    .unwrap_or_default();
                let param_type = if let Some(type_pair) = param_inner.next() {
                    let type_span = (type_pair.as_span(), self.module.name).into();
                    let type_str = type_pair.as_str();
                    self.module
                        .arena
                        .find_type_by_name(&self.interner, type_str)
                        .ok_or_else(|| {
                            ParseError::TypeError(ErrorData::new(
                                type_span,
                                format!("Неизвестный тип: {}", type_str),
                            ))
                        })?
                } else {
                    self.module
                        .arena
                        .register_custom_type(&self.interner, "неизвестно")
                };

                params.push(Parameter {
                    name: self.module.arena.intern_string(&self.interner, &name),
                    param_type,
                    span: token_span,
                });
            }
        }

        Ok(params)
    }

    fn parse_block(
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
                Rule::assignment => {
                    let stmt_id = self.parse_assignment(inner)?;
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
                Rule::while_stmt => {
                    let stmt_id = self.parse_while_stmt(inner)?;
                    statements.push(stmt_id);
                }
                Rule::for_stmt => {
                    let stmt_id = self.parse_for_stmt(inner)?;
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

    fn parse_assignment(
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

        while let Some(token) = inner.next() {
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
                        assignment_span.into(),
                        format!("Отсутствует значение у переменной: {}", name_str),
                    ))
                })?,
            },
            assignment_span,
        );
        Ok(stmt_id)
    }

    fn parse_property_assign(
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
                ParseError::UnexpectedToken(ErrorData::new(
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

    fn parse_if_stmt(&mut self, pair: pest::iterators::Pair<Rule>) -> Result<StmtId, ParseError> {
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

    fn parse_while_stmt(
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

    fn parse_for_stmt(&mut self, pair: pest::iterators::Pair<Rule>) -> Result<StmtId, ParseError> {
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

        let upd_span = (for_upd_token.as_span(), self.module.name).into();
        let mut upd_inner = for_upd_token.into_inner();
        let first_upd_token = upd_inner.next().ok_or_else(|| {
            ParseError::InvalidSyntax(ErrorData::new(upd_span, "Ожидалось выражение".into()))
        })?;
        let ca_span = (first_upd_token.as_span(), self.module.name).into();
        let update_expr = match first_upd_token.as_rule() {
            Rule::compound_assign => {
                let mut ca_inner = first_upd_token.into_inner();
                let var_str = ca_inner
                    .next()
                    .ok_or_else(|| {
                        ParseError::InvalidSyntax(ErrorData::new(
                            ca_span,
                            "Ожидалось выражение".into(),
                        ))
                    })?
                    .as_str()
                    .to_string();
                let op_str = ca_inner
                    .next()
                    .ok_or_else(|| {
                        ParseError::InvalidSyntax(ErrorData::new(
                            ca_span,
                            "Ожидалось выражение".into(),
                        ))
                    })?
                    .as_str()
                    .to_string();
                let val_expr = self.parse_expression(ca_inner.next().ok_or_else(|| {
                    ParseError::InvalidSyntax(ErrorData::new(ca_span, "Ожидалось выражение".into()))
                })?)?;

                let var_sym = self.module.arena.intern_string(&self.interner, &var_str);
                let var_expr = self
                    .module
                    .arena
                    .add_expression(ExpressionKind::Identifier(var_sym), ca_span);

                let bin_op = match op_str.as_str() {
                    "+=" => BinaryOperator::Add,
                    "-=" => BinaryOperator::Sub,
                    "*=" => BinaryOperator::Mul,
                    "/=" => BinaryOperator::Div,
                    _ => BinaryOperator::Add,
                };

                self.module.arena.add_expression(
                    ExpressionKind::Binary {
                        left: var_expr,
                        op: bin_op,
                        right: val_expr,
                    },
                    ca_span,
                )
            }
            Rule::assignment_expr => {
                let ae_span = (first_upd_token.as_span(), self.module.name).into();
                let mut ae_inner = first_upd_token.into_inner();
                let _var_str = ae_inner
                    .next()
                    .ok_or_else(|| {
                        ParseError::InvalidSyntax(ErrorData::new(
                            ae_span,
                            "Ожидалось выражение".into(),
                        ))
                    })?
                    .as_str()
                    .to_string();
                let val_expr = self.parse_expression(ae_inner.next().ok_or_else(|| {
                    ParseError::InvalidSyntax(ErrorData::new(ae_span, "Ожидалось выражение".into()))
                })?)?;
                val_expr
            }
            _ => self.parse_expression(first_upd_token)?,
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
                update: update_expr,
                body,
            },
            for_span,
        );
        Ok(stmt_id)
    }

    fn parse_return_stmt(
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

    fn parse_import_stmt(
        &mut self,
        pair: pest::iterators::Pair<Rule>,
    ) -> Result<StmtId, ParseError> {
        let import_span: Span = (pair.as_span(), self.module.name).into();
        let mut inner = pair.into_inner();

        let path_token = inner.next().ok_or_else(|| {
            ParseError::InvalidSyntax(ErrorData::new(
                import_span,
                "Неожиданный токен".into(),
            ))
        })?;
        let alias_token = inner.next().ok_or_else(|| {
            ParseError::InvalidSyntax(ErrorData::new(
                import_span,
                "Неожиданный токен".into(),
            ))
        })?;

        let raw_path = path_token.as_str();
        let clean_path = if raw_path.len() >= 2 {
            &raw_path[1..raw_path.len() - 1]
        } else {
            raw_path
        };

        let path_symbol = self.module.arena.intern_string(&self.interner, clean_path);
        let alias_symbol = self.module.arena.intern_string(&self.interner, alias_token.as_str());

        let import_data = Import {
            item: ImportItem {
                path: path_symbol,
                alias: alias_symbol,
            },
            span: import_span,
        };

        self.module.imports.push(import_data);
        let stmt_id = self
            .module
            .arena
            .add_statement(StatementKind::Empty, import_span);

        Ok(stmt_id)
    }

fn parse_expr_stmt(&mut self, pair: pest::iterators::Pair<Rule>) -> Result<StmtId, ParseError> {
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

        Err(ParseError::InvalidSyntax(ErrorData::new(
            expr_span,
            "Ожидалось выражение".into(),
        )))
    }

    fn parse_expression(
        &mut self,
        pair: pest::iterators::Pair<Rule>,
    ) -> Result<ExprId, ParseError> {
        let expr_span = (pair.as_span(), self.module.name).into();
        let mut inner = pair.into_inner();

        if let Some(first_token) = inner.next() {
            return self.parse_logical_or(first_token);
        }
        Err(ParseError::InvalidSyntax(ErrorData::new(
            expr_span,
            "Ожидалось выражение".into(),
        )))
    }

    fn parse_logical_or(
        &mut self,
        pair: pest::iterators::Pair<Rule>,
    ) -> Result<ExprId, ParseError> {
        let or_span = (pair.as_span(), self.module.name).into();
        let mut inner = pair.into_inner();
        let mut left = self.parse_logical_and(inner.next().ok_or_else(|| {
            ParseError::InvalidSyntax(ErrorData::new(or_span, "Ожидалось выражение".into()))
        })?)?;

        while let Some(token) = inner.next() {
            if token.as_str() == "или" {
                let right = self.parse_logical_and(inner.next().ok_or_else(|| {
                    ParseError::InvalidSyntax(ErrorData::new(or_span, "Ожидалось выражение".into()))
                })?)?;
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

    fn parse_logical_and(
        &mut self,
        pair: pest::iterators::Pair<Rule>,
    ) -> Result<ExprId, ParseError> {
        let and_span = (pair.as_span(), self.module.name).into();
        let mut inner = pair.into_inner();
        let mut left = self.parse_comparison(inner.next().ok_or_else(|| {
            ParseError::InvalidSyntax(ErrorData::new(and_span, "Ожидалось выражение".into()))
        })?)?;

        while let Some(token) = inner.next() {
            if token.as_str() == "и" {
                let right = self.parse_comparison(inner.next().ok_or_else(|| {
                    ParseError::InvalidSyntax(ErrorData::new(
                        and_span,
                        "Ожидалось выражение".into(),
                    ))
                })?)?;
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

    fn parse_comparison(
        &mut self,
        pair: pest::iterators::Pair<Rule>,
    ) -> Result<ExprId, ParseError> {
        let cmp_span = (pair.as_span(), self.module.name).into();
        let mut inner = pair.into_inner();
        let mut left = self.parse_addition(inner.next().ok_or_else(|| {
            ParseError::InvalidSyntax(ErrorData::new(cmp_span, "Ожидалось выражение".into()))
        })?)?;

        while let Some(token) = inner.next() {
            match token.as_rule() {
                Rule::comp_op => {
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
                    let right = self.parse_addition(inner.next().ok_or_else(|| {
                        ParseError::InvalidSyntax(ErrorData::new(
                            cmp_span,
                            "Ожидалось выражение".into(),
                        ))
                    })?)?;
                    left = self
                        .module
                        .arena
                        .add_expression(ExpressionKind::Binary { op, left, right }, cmp_span);
                }
                _ => {}
            }
        }

        Ok(left)
    }

    fn parse_addition(&mut self, pair: pest::iterators::Pair<Rule>) -> Result<ExprId, ParseError> {
        let add_span = (pair.as_span(), self.module.name).into();
        let mut inner = pair.into_inner();
        let mut left = self.parse_multiplication(inner.next().ok_or_else(|| {
            ParseError::InvalidSyntax(ErrorData::new(add_span, "Ожидалось выражение".into()))
        })?)?;

        while let Some(token) = inner.next() {
            match token.as_rule() {
                Rule::add_op => {
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
                    let right = self.parse_multiplication(inner.next().ok_or_else(|| {
                        ParseError::InvalidSyntax(ErrorData::new(
                            add_span,
                            "Ожидалось выражение".into(),
                        ))
                    })?)?;
                    left = self
                        .module
                        .arena
                        .add_expression(ExpressionKind::Binary { op, left, right }, add_span);
                }
                _ => {}
            }
        }

        Ok(left)
    }

    fn parse_multiplication(
        &mut self,
        pair: pest::iterators::Pair<Rule>,
    ) -> Result<ExprId, ParseError> {
        let mul_span = (pair.as_span(), self.module.name).into();
        let mut inner = pair.into_inner();
        let mut left = self.parse_unary(inner.next().ok_or_else(|| {
            ParseError::InvalidSyntax(ErrorData::new(mul_span, "Ожидалось выражение".into()))
        })?)?;

        while let Some(token) = inner.next() {
            match token.as_rule() {
                Rule::mul_op => {
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
                    let right = self.parse_unary(inner.next().ok_or_else(|| {
                        ParseError::InvalidSyntax(ErrorData::new(
                            mul_span,
                            "Ожидалось выражение".into(),
                        ))
                    })?)?;
                    left = self
                        .module
                        .arena
                        .add_expression(ExpressionKind::Binary { op, left, right }, mul_span);
                }
                _ => {}
            }
        }

        Ok(left)
    }

    fn parse_unary(&mut self, pair: pest::iterators::Pair<Rule>) -> Result<ExprId, ParseError> {
        let unary_span = (pair.as_span(), self.module.name).into();
        let mut inner = pair.into_inner();

        let mut unary_op = None;
        while let Some(token) = inner.next() {
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

        Err(ParseError::InvalidSyntax(ErrorData::new(
            unary_span,
            "Ожидалось выражение".into(),
        )))
    }

    fn parse_postfix(&mut self, pair: pest::iterators::Pair<Rule>) -> Result<ExprId, ParseError> {
        let expr_span = (pair.as_span(), self.module.name).into();
        let mut inner = pair.into_inner();
        let mut expr = self.parse_primary(inner.next().ok_or_else(|| {
            ParseError::InvalidSyntax(ErrorData::new(expr_span, "Ожидалось выражение".into()))
        })?)?;

        while let Some(token) = inner.next() {
            let postfix_span = (token.as_span(), self.module.name).into();
            match token.as_rule() {
                Rule::function_call => {
                    let mut args = Vec::new();
                    for arg_pair in token.into_inner() {
                        if arg_pair.as_rule() == Rule::arg_list {
                            for arg in arg_pair.into_inner() {
                                let arg_expr = self.parse_expression(arg)?;
                                args.push(arg_expr);
                            }
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
                            for arg_pair in arg_list.into_inner() {
                                let arg_expr = self.parse_expression(arg_pair)?;
                                args.push(arg_expr);
                            }
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

    fn parse_primary(&mut self, pair: pest::iterators::Pair<Rule>) -> Result<ExprId, ParseError> {
        let primary_span = (pair.as_span(), self.module.name).into();
        match pair.as_rule() {
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
                        for arg_pair in arg_list.into_inner() {
                            let arg_expr = self.parse_expression(arg_pair)?;
                            args.push(arg_expr);
                        }
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
                let text_symbol = self.module.arena.intern_string(&self.interner, trimmed);
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
                } else {
                    if let Ok(num) = s.parse::<i64>() {
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
            Rule::this_expr => Ok(self
                .module
                .arena
                .add_expression(ExpressionKind::This, primary_span)),
            _ => Err(ParseError::InvalidSyntax(ErrorData::new(
                primary_span,
                "Неожиданное выражение".into(),
            ))),
        }
    }
}
