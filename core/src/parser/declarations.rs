use crate::ast::prelude::*;
use crate::parser::parser::Rule;
use crate::parser::prelude::{ParseError, Parser as ParserTrait};
use crate::shared::SharedMut;
use std::sync::Arc;

impl ParserTrait {
    pub(crate) fn parse_function(
        &mut self,
        pair: pest::iterators::Pair<Rule>,
    ) -> Result<StmtId, ParseError> {
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

        for token in inner {
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
                    return if self.nesting_level == 0 {
                        self.module
                            .functions
                            .insert(symbol_name, Arc::new(func_def));
                        Ok(self
                            .module
                            .arena
                            .add_statement(StatementKind::Empty, func_span))
                    } else {
                        let stmt_id = self
                            .module
                            .arena
                            .add_statement(StatementKind::FunctionDefinition(func_def), func_span);
                        Ok(stmt_id)
                    };
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

    pub(crate) fn parse_library_stmt(
        &mut self,
        pair: pest::iterators::Pair<Rule>,
    ) -> Result<StmtId, ParseError> {
        let library_span: Span = (pair.as_span(), self.module.name).into();
        let mut inner = pair.into_inner();

        let path_token = inner.next().ok_or_else(|| {
            ParseError::InvalidSyntax(ErrorData::new(
                library_span,
                "Ожидался путь к библиотеке".into(),
            ))
        })?;
        let raw_path = path_token.as_str();
        let clean_path = if raw_path.len() >= 2 {
            &raw_path[1..raw_path.len() - 1]
        } else {
            raw_path
        };

        let path = self.module.arena.intern_string(&self.interner, clean_path);
        let mut functions = Vec::new();
        let mut globals = Vec::new();

        for token in inner {
            match token.as_rule() {
                Rule::library_function => functions.push(self.parse_library_function(token)?),
                Rule::library_global => globals.push(self.parse_library_global(token)?),
                _ => {}
            }
        }

        Ok(self.module.arena.add_statement(
            StatementKind::NativeLibraryDefinition(NativeLibraryDefinition {
                path,
                functions,
                globals,
                span: library_span,
            }),
            library_span,
        ))
    }

    pub(crate) fn parse_library_function(
        &mut self,
        pair: pest::iterators::Pair<Rule>,
    ) -> Result<NativeFunctionDefinition, ParseError> {
        let function_span: Span = (pair.as_span(), self.module.name).into();
        let mut inner = pair.into_inner();

        let name_token = inner.next().ok_or_else(|| {
            ParseError::InvalidSyntax(ErrorData::new(
                function_span,
                "Ожидалось имя функции библиотеки".into(),
            ))
        })?;
        let name = self
            .module
            .arena
            .intern_string(&self.interner, name_token.as_str());

        let mut params = Vec::new();
        let mut return_type = None;

        for token in inner {
            match token.as_rule() {
                Rule::library_param_list => params = self.parse_library_param_list(token)?,
                Rule::return_type => {
                    if let Some(type_token) = token.into_inner().next() {
                        return_type = Some(self.parse_type_name(type_token)?);
                    }
                }
                Rule::empty_block => {}
                _ => {}
            }
        }

        Ok(NativeFunctionDefinition {
            name,
            params,
            return_type,
            span: function_span,
        })
    }

    pub(crate) fn parse_library_global(
        &mut self,
        pair: pest::iterators::Pair<Rule>,
    ) -> Result<NativeGlobalDefinition, ParseError> {
        let global_span: Span = (pair.as_span(), self.module.name).into();
        let mut inner = pair.into_inner();

        let name_token = inner.next().ok_or_else(|| {
            ParseError::InvalidSyntax(ErrorData::new(
                global_span,
                "Ожидалось имя глобальной переменной библиотеки".into(),
            ))
        })?;
        let type_token = inner.next().ok_or_else(|| {
            ParseError::InvalidSyntax(ErrorData::new(
                global_span,
                "Ожидался тип глобальной переменной библиотеки".into(),
            ))
        })?;

        Ok(NativeGlobalDefinition {
            name: self
                .module
                .arena
                .intern_string(&self.interner, name_token.as_str()),
            value_type: self.parse_type_name(type_token)?,
            span: global_span,
        })
    }

    pub(crate) fn parse_library_param_list(
        &mut self,
        pair: pest::iterators::Pair<Rule>,
    ) -> Result<Vec<Parameter>, ParseError> {
        let mut params = Vec::new();

        for token in pair.into_inner() {
            if token.as_rule() != Rule::library_param {
                continue;
            }

            let token_span: Span = (token.as_span(), self.module.name).into();
            let mut inner = token.into_inner();
            let name_token = inner.next().ok_or_else(|| {
                ParseError::InvalidSyntax(ErrorData::new(
                    token_span,
                    "Ожидалось имя параметра".into(),
                ))
            })?;
            let type_token = inner.next().ok_or_else(|| {
                ParseError::InvalidSyntax(ErrorData::new(
                    token_span,
                    "Ожидался тип параметра".into(),
                ))
            })?;

            params.push(Parameter {
                name: self
                    .module
                    .arena
                    .intern_string(&self.interner, name_token.as_str()),
                param_type: self.parse_type_name(type_token)?,
                default_value: None,
                span: token_span,
            });
        }

        Ok(params)
    }

    pub(crate) fn parse_type_name(
        &self,
        token: pest::iterators::Pair<Rule>,
    ) -> Result<TypeId, ParseError> {
        let type_span: Span = (token.as_span(), self.module.name).into();
        let type_str = token.as_str();
        self.module
            .arena
            .find_type_by_name(&self.interner, type_str)
            .ok_or_else(|| {
                ParseError::TypeError(ErrorData::new(
                    type_span,
                    format!("Неизвестный тип: {}", type_str),
                ))
            })
    }

    pub(crate) fn parse_class(
        &mut self,
        pair: pest::iterators::Pair<Rule>,
    ) -> Result<StmtId, ParseError> {
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
        let mut base_class = None;
        if let Some(token) = inner.peek() {
            if token.as_rule() == Rule::inheritance_clause {
                let inheritance = inner.next().unwrap();
                if let Some(base_token) = inheritance.into_inner().next() {
                    let base_name = base_token.as_str();
                    let base_symbol = self.module.arena.intern_string(&self.interner, base_name);
                    if !self.module.classes.contains_key(&base_symbol) {
                        return Err(ParseError::TypeError(ErrorData::new(
                            (base_token.as_span(), self.module.name).into(),
                            format!("Базовый класс '{}' не найден", base_name),
                        )));
                    }
                    base_class = Some(base_symbol);
                }
            }
        }
        let mut class_def = ClassDefinition::new_with_base(symbol_name, base_class, class_span);
        if let Some(base_symbol) = base_class {
            if let Some(base_def) = self.module.classes.get(&base_symbol) {
                base_def.read(|base| {
                    class_def.fields.extend(base.fields.clone());
                    class_def.methods.extend(base.methods.clone());
                    class_def.constructor = base.constructor.clone();
                });
            }
        }

        for token in inner {
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

    pub(crate) fn parse_class_field(
        &mut self,
        pair: pest::iterators::Pair<Rule>,
    ) -> Result<ClassField, ParseError> {
        let field_span: Span = (pair.as_span(), self.module.name).into();
        let inner = pair.into_inner();
        let mut visibility = Visibility::Private;
        let mut is_static = false;
        let mut field_name = String::new();
        let mut field_type = None;
        let mut default_value = None;

        for token in inner {
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

    pub(crate) fn parse_constructor(
        &mut self,
        pair: pest::iterators::Pair<Rule>,
    ) -> Result<ClassMethod, ParseError> {
        let constructor_span: Span = (pair.as_span(), self.module.name).into();
        let inner = pair.into_inner();
        let mut visibility = Visibility::Private;
        let mut is_static = false;
        let mut method_name = String::new();
        let mut params = Vec::new();
        let mut return_type = None;
        let mut body = None;

        for token in inner {
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

    pub(crate) fn parse_class_method(
        &mut self,
        pair: pest::iterators::Pair<Rule>,
    ) -> Result<ClassMethod, ParseError> {
        let method_span: Span = (pair.as_span(), self.module.name).into();
        let inner = pair.into_inner();
        let mut visibility = Visibility::Private;
        let mut is_static = false;
        let mut method_name = String::new();
        let mut params = Vec::new();
        let mut return_type = None;
        let mut body = None;

        for token in inner {
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

    pub(crate) fn parse_param_list(
        &mut self,
        pair: pest::iterators::Pair<Rule>,
    ) -> Result<Vec<Parameter>, ParseError> {
        let mut params = Vec::new();
        let mut saw_default = false;

        for param_pair in pair.into_inner() {
            if param_pair.as_rule() == Rule::param {
                let token_span: Span = (param_pair.as_span(), self.module.name).into();
                let mut param_inner = param_pair.into_inner();

                let name_str = param_inner
                    .next()
                    .map(|p| p.as_str().to_string())
                    .unwrap_or_default();
                let name_symbol = self.module.arena.intern_string(&self.interner, &name_str);

                let mut param_type = None;
                let mut default_value = None;

                for inner in param_inner {
                    match inner.as_rule() {
                        Rule::type_name => {
                            let type_span = (inner.as_span(), self.module.name).into();
                            let type_str = inner.as_str();
                            param_type = Some(
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
                            let expr_id = self.parse_expression(inner)?;
                            default_value = Some(expr_id);
                        }
                        _ => {}
                    }
                }

                if default_value.is_some() {
                    saw_default = true;
                } else if saw_default {
                    // Если у этого параметра НЕТ дефолта, но у предыдущего ОН БЫЛ
                    return Err(ParseError::TypeError(ErrorData::new(
                        token_span,
                        format!(
                            "Обязательный параметр '{}' не может следовать за параметром со значением по умолчанию",
                            name_str
                        ),
                    )));
                }

                let final_type = param_type.unwrap_or_else(|| {
                    self.module
                        .arena
                        .register_custom_type(&self.interner, "неизвестно")
                });

                params.push(Parameter {
                    name: name_symbol,
                    param_type: final_type,
                    default_value,
                    span: token_span,
                });
            }
        }

        Ok(params)
    }
}
