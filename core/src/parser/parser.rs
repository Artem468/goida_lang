use crate::ast::prelude::*;
use crate::ast::program::{FieldData, MethodType};
use crate::interpreter::prelude::{Module, SharedInterner, Value};
use crate::parser::prelude::{
    extract_last_token, translate_rule, ParseError, Parser as ParserTrait,
};
use crate::shared::SharedMut;
use pest::error::ErrorVariant;
use pest::Parser;
use pest_derive::Parser;
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use string_interner::DefaultSymbol as Symbol;

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

    fn get_char_range(&self, s: &str, start: usize, end: usize) -> String {
        let mut indices = s
            .char_indices()
            .map(|(idx, _)| idx)
            .chain(std::iter::once(s.len()));
        let byte_start = indices.nth(start).unwrap_or(s.len());
        let byte_end = indices
            .nth(end.saturating_sub(start + 1))
            .unwrap_or(s.len());
        s[byte_start..byte_end].to_string()
    }

    pub fn parse(mut self, code: &str) -> Result<Module, ParseError> {
        let pairs = ProgramParser::parse(Rule::program, code).map_err(|e| {
            let (start, end) = match e.location {
                pest::error::InputLocation::Pos(pos) => extract_last_token(code, pos),
                pest::error::InputLocation::Span((start, end)) => (start, end),
            };

            let message = match &e.variant {
                ErrorVariant::ParsingError {
                    positives,
                    negatives,
                } => {
                    let mut parts = Vec::new();

                    if !positives.is_empty() {
                        let found = self.get_char_range(code, start, end);

                        let found = if found.is_empty() {
                            code.chars()
                                .nth(start)
                                .map(|c| c.to_string())
                                .unwrap_or_else(|| "конец файла".into())
                        } else {
                            found.to_string()
                        };

                        let expected: Vec<String> =
                            positives.iter().map(|r| translate_rule(r)).collect();
                        parts.push(format!(
                            "ожидалось: \n{} получено {}",
                            expected.join("\n"),
                            found
                        ));
                    }

                    if !negatives.is_empty() {
                        let unexpected: Vec<String> =
                            negatives.iter().map(|r| translate_rule(r)).collect();
                        parts.push(format!("неожиданный токен: \n{}", unexpected.join("\n")));
                    }

                    if parts.is_empty() {
                        "неизвестная ошибка синтаксиса".to_string()
                    } else {
                        parts.join("; ")
                    }
                }
                ErrorVariant::CustomError { message } => message.clone(),
            };

            ParseError::InvalidSyntax(ErrorData::new(
                Span::new(start, end, self.module.name),
                message,
            ))
        })?;
        self.module.arena.init_builtin_types(&self.interner);
        self.init_builtin_error_classes();
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
                        Rule::library_stmt => {
                            let stmt_id = self.parse_library_stmt(inner)?;
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
                        Rule::try_stmt => {
                            let stmt_id = self.parse_try_stmt(inner)?;
                            self.module.body.push(stmt_id);
                        }
                        Rule::raise_stmt => {
                            let stmt_id = self.parse_raise_stmt(inner)?;
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
        self.validate_module_names()?;
        self.module.arena.optimize_all(&self.interner);
        Ok(self.module)
    }

    fn init_builtin_error_classes(&mut self) {
        let error_root = self
            .module
            .arena
            .intern_string(&self.interner, "Ошибка");
        self.module
            .arena
            .register_custom_type(&self.interner, "Ошибка");
        self.module.classes.entry(error_root).or_insert_with(|| {
            SharedMut::new(ClassDefinition::new(error_root, Span::default()))
        });

        for class_name in [
            "ОшибкаПеременной",
            "ОшибкаФункции",
            "ОшибкаМетода",
            "ОшибкаТипа",
            "ОшибкаДеленияНаНоль",
            "ОшибкаОперации",
            "ОшибкаВводаВывода",
            "ОшибкаИмпорта",
            "Паника",
        ] {
            let symbol = self.module.arena.intern_string(&self.interner, class_name);
            self.module
                .arena
                .register_custom_type(&self.interner, class_name);
            self.module.classes.entry(symbol).or_insert_with(|| {
                SharedMut::new(ClassDefinition::new_with_base(
                    symbol,
                    Some(error_root),
                    Span::default(),
                ))
            });
        }
    }

    fn validate_module_names(&self) -> Result<(), ParseError> {
        let mut known = self.known_global_names();
        for stmt_id in &self.module.body {
            let stmt = self.module.arena.get_statement(*stmt_id).unwrap();
            match &stmt.kind {
                StatementKind::Assign { name, .. } => {
                    known.insert(*name);
                }
                StatementKind::NativeLibraryDefinition(definition) => {
                    for function in &definition.functions {
                        known.insert(function.name);
                    }
                    for global in &definition.globals {
                        known.insert(global.name);
                    }
                }
                _ => {}
            }
        }

        let mut scopes = vec![known];
        for stmt_id in &self.module.body {
            self.validate_statement_names(*stmt_id, &mut scopes)?;
        }
        Ok(())
    }

    fn known_global_names(&self) -> HashSet<Symbol> {
        let mut names = HashSet::new();

        for name in self.module.functions.keys() {
            names.insert(*name);
        }
        for name in self.module.classes.keys() {
            names.insert(*name);
        }
        for import in &self.module.imports {
            names.insert(import.item.alias);
        }
        for module in self.module.modules.values() {
            self.collect_module_exported_names(module, &mut names);
        }

        for name in [
            "печать",
            "ввод",
            "тип",
            "является",
            "число",
            "строка",
            "логический",
            "дробь",
            "список",
            "массив",
            "словарь",
            "из_json",
            "в_json",
            "строка_из_указателя",
            "Список",
            "Массив",
            "Словарь",
            "Строка",
            "Файл",
            "Система",
            "Терминал",
            "ДатаВремя",
        ] {
            names.insert(self.module.arena.intern_string(&self.interner, name));
        }

        names
    }

    fn collect_module_exported_names(&self, module: &Module, names: &mut HashSet<Symbol>) {
        for name in module.functions.keys() {
            names.insert(*name);
        }
        for name in module.classes.keys() {
            names.insert(*name);
        }
        for stmt_id in &module.body {
            let Some(stmt) = module.arena.get_statement(*stmt_id) else {
                continue;
            };
            if let StatementKind::Assign { name, .. } = stmt.kind {
                names.insert(name);
            } else if let StatementKind::NativeLibraryDefinition(definition) = &stmt.kind {
                for function in &definition.functions {
                    names.insert(function.name);
                }
                for global in &definition.globals {
                    names.insert(global.name);
                }
            }
        }
        for nested in module.modules.values() {
            self.collect_module_exported_names(nested, names);
        }
    }

    fn validate_statement_names(
        &self,
        stmt_id: StmtId,
        scopes: &mut Vec<HashSet<Symbol>>,
    ) -> Result<(), ParseError> {
        let stmt = self.module.arena.get_statement(stmt_id).unwrap();
        match &stmt.kind {
            StatementKind::Expression(expr) => self.validate_expression_names(*expr, scopes),
            StatementKind::Assign { name, value, .. } => {
                self.validate_expression_names(*value, scopes)?;
                scopes.last_mut().unwrap().insert(*name);
                Ok(())
            }
            StatementKind::IndexAssign {
                object,
                index,
                value,
            } => {
                self.validate_expression_names(*object, scopes)?;
                self.validate_expression_names(*index, scopes)?;
                self.validate_expression_names(*value, scopes)
            }
            StatementKind::PropertyAssign { object, value, .. } => {
                self.validate_expression_names(*object, scopes)?;
                self.validate_expression_names(*value, scopes)
            }
            StatementKind::If {
                condition,
                then_body,
                else_body,
            } => {
                self.validate_expression_names(*condition, scopes)?;
                self.validate_statement_names(*then_body, scopes)?;
                if let Some(else_body) = else_body {
                    self.validate_statement_names(*else_body, scopes)?;
                }
                Ok(())
            }
            StatementKind::While { condition, body } => {
                self.validate_expression_names(*condition, scopes)?;
                self.validate_statement_names(*body, scopes)
            }
            StatementKind::For {
                variable,
                init,
                condition,
                update,
                body,
            } => {
                self.validate_expression_names(*init, scopes)?;
                scopes.push(HashSet::new());
                scopes.last_mut().unwrap().insert(*variable);
                self.validate_expression_names(*condition, scopes)?;
                self.validate_expression_names(*update, scopes)?;
                self.validate_statement_names(*body, scopes)?;
                scopes.pop();
                Ok(())
            }
            StatementKind::Try { body, handlers } => {
                self.validate_statement_names(*body, scopes)?;
                for handler in handlers {
                    if let Some(error_text) = handler.error_text {
                        scopes.push(HashSet::new());
                        scopes.last_mut().unwrap().insert(error_text);
                        self.validate_statement_names(handler.body, scopes)?;
                        scopes.pop();
                    } else {
                        self.validate_statement_names(handler.body, scopes)?;
                    }
                }
                Ok(())
            }
            StatementKind::Raise { message, .. } => {
                if let Some(message) = message {
                    self.validate_expression_names(*message, scopes)?;
                }
                Ok(())
            }
            StatementKind::Block(statements) => {
                scopes.push(HashSet::new());
                for stmt_id in statements {
                    self.validate_statement_names(*stmt_id, scopes)?;
                }
                scopes.pop();
                Ok(())
            }
            StatementKind::Return(expr) => {
                if let Some(expr) = expr {
                    self.validate_expression_names(*expr, scopes)?;
                }
                Ok(())
            }
            StatementKind::FunctionDefinition(def) => {
                scopes.last_mut().unwrap().insert(def.name);
                let mut local = HashSet::new();
                for param in &def.params {
                    local.insert(param.name);
                    if let Some(default_value) = param.default_value {
                        self.validate_expression_names(default_value, scopes)?;
                    }
                }
                scopes.push(local);
                self.validate_statement_names(def.body, scopes)?;
                scopes.pop();
                Ok(())
            }
            StatementKind::ClassDefinition(def) => {
                for (_, _, field_data) in def.fields.values() {
                    if let FieldData::Expression(Some(expr)) = field_data {
                        self.validate_expression_names(*expr, scopes)?;
                    }
                }
                for (_, _, method) in def.methods.values() {
                    if let MethodType::User(function) = method {
                        self.validate_function_body_names(function, scopes)?;
                    }
                }
                if let Some(MethodType::User(function)) = &def.constructor {
                    self.validate_function_body_names(function, scopes)?;
                }
                Ok(())
            }
            StatementKind::NativeLibraryDefinition(_) | StatementKind::Empty => Ok(()),
        }
    }

    fn validate_function_body_names(
        &self,
        function: &FunctionDefinition,
        scopes: &mut Vec<HashSet<Symbol>>,
    ) -> Result<(), ParseError> {
        let mut local = HashSet::new();
        for param in &function.params {
            local.insert(param.name);
            if let Some(default_value) = param.default_value {
                self.validate_expression_names(default_value, scopes)?;
            }
        }
        scopes.push(local);
        self.validate_statement_names(function.body, scopes)?;
        scopes.pop();
        Ok(())
    }

    fn validate_expression_names(
        &self,
        expr_id: ExprId,
        scopes: &mut Vec<HashSet<Symbol>>,
    ) -> Result<(), ParseError> {
        let expr = self.module.arena.get_expression(expr_id).unwrap();
        match &expr.kind {
            ExpressionKind::Identifier(symbol) => {
                if self.is_name_known(*symbol, scopes) {
                    Ok(())
                } else {
                    let name = self
                        .module
                        .arena
                        .resolve_symbol(&self.interner, *symbol)
                        .unwrap_or_default();
                    Err(ParseError::InvalidSyntax(ErrorData::new(
                        expr.span,
                        format!("Имя '{}' не найдено", name),
                    )))
                }
            }
            ExpressionKind::Binary { left, right, .. } => {
                self.validate_expression_names(*left, scopes)?;
                self.validate_expression_names(*right, scopes)
            }
            ExpressionKind::Unary { operand, .. } => {
                self.validate_expression_names(*operand, scopes)
            }
            ExpressionKind::FunctionCall { function, args } => {
                self.validate_expression_names(*function, scopes)?;
                for arg in args {
                    self.validate_expression_names(arg.value, scopes)?;
                }
                Ok(())
            }
            ExpressionKind::Index { object, index } => {
                self.validate_expression_names(*object, scopes)?;
                self.validate_expression_names(*index, scopes)
            }
            ExpressionKind::PropertyAccess { object, .. } => {
                self.validate_expression_names(*object, scopes)
            }
            ExpressionKind::MethodCall { object, args, .. } => {
                self.validate_expression_names(*object, scopes)?;
                for arg in args {
                    self.validate_expression_names(arg.value, scopes)?;
                }
                Ok(())
            }
            ExpressionKind::ObjectCreation { class_name, args } => {
                if !self.is_name_known(*class_name, scopes) {
                    let name = self
                        .module
                        .arena
                        .resolve_symbol(&self.interner, *class_name)
                        .unwrap_or_default();
                    return Err(ParseError::InvalidSyntax(ErrorData::new(
                        expr.span,
                        format!("Класс '{}' не найден", name),
                    )));
                }
                for arg in args {
                    self.validate_expression_names(arg.value, scopes)?;
                }
                Ok(())
            }
            ExpressionKind::Literal(_) | ExpressionKind::This => Ok(()),
        }
    }

    fn is_name_known(&self, symbol: Symbol, scopes: &[HashSet<Symbol>]) -> bool {
        if scopes.iter().rev().any(|scope| scope.contains(&symbol)) {
            return true;
        }

        let Some(name) = self.module.arena.resolve_symbol(&self.interner, symbol) else {
            return false;
        };
        if let Some((module_name, member_name)) = name.split_once('.') {
            let module_symbol = self.module.arena.intern_string(&self.interner, module_name);
            let member_symbol = self.module.arena.intern_string(&self.interner, member_name);
            if !scopes
                .iter()
                .rev()
                .any(|scope| scope.contains(&module_symbol))
            {
                return false;
            }

            return self.module.modules.values().any(|module| {
                module.functions.contains_key(&member_symbol)
                    || module.classes.contains_key(&member_symbol)
                    || module.globals.contains_key(&member_symbol)
            });
        }

        false
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

    fn parse_library_stmt(
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

    fn parse_library_function(
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

    fn parse_library_global(
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

    fn parse_library_param_list(
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

    fn parse_type_name(&self, token: pest::iterators::Pair<Rule>) -> Result<TypeId, ParseError> {
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

    fn parse_class_field(
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

    fn parse_constructor(
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

    fn parse_class_method(
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

    fn parse_param_list(
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
                Rule::library_stmt => {
                    let stmt_id = self.parse_library_stmt(inner)?;
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

    fn parse_try_stmt(&mut self, pair: pest::iterators::Pair<Rule>) -> Result<StmtId, ParseError> {
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
                                let symbol =
                                    self.module.arena.intern_string(&self.interner, name);
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

    fn parse_raise_stmt(
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
                self.parse_expression(ae_inner.next().ok_or_else(|| {
                    ParseError::InvalidSyntax(ErrorData::new(ae_span, "Ожидалось выражение".into()))
                })?)?
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
            ParseError::InvalidSyntax(ErrorData::new(import_span, "Неожиданный токен".into()))
        })?;
        let alias_token = inner.next().ok_or_else(|| {
            ParseError::InvalidSyntax(ErrorData::new(import_span, "Неожиданный токен".into()))
        })?;

        let raw_path = path_token.as_str();
        let clean_path = if raw_path.len() >= 2 {
            &raw_path[1..raw_path.len() - 1]
        } else {
            raw_path
        };

        let path_symbol = self.module.arena.intern_string(&self.interner, clean_path);
        let alias_symbol = self
            .module
            .arena
            .intern_string(&self.interner, alias_token.as_str());

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

        let module = self.parse_and_register_import(path_symbol, import_span)?;
        self.register_imported_type_aliases(alias_symbol, module);
        self.module
            .globals
            .insert(alias_symbol, Value::Module(module));

        Ok(stmt_id)
    }

    fn register_imported_type_aliases(&mut self, alias_symbol: Symbol, module_symbol: Symbol) {
        let alias_name = self
            .interner
            .read(|i| i.resolve(alias_symbol).unwrap_or_default().to_string());

        let qualified_type_names = self
            .module
            .modules
            .get(&module_symbol)
            .map(|module| {
                module
                    .classes
                    .keys()
                    .filter_map(|class_symbol| {
                        self.interner.read(|i| {
                            i.resolve(*class_symbol)
                                .map(|class_name| format!("{alias_name}.{class_name}"))
                        })
                    })
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();

        for qualified_name in qualified_type_names {
            self.module
                .arena
                .register_custom_type(&self.interner, qualified_name.as_str());
        }
    }

    pub fn parse_and_register_import(
        &mut self,
        import_path_symbol: Symbol,
        span: Span,
    ) -> Result<Symbol, ParseError> {
        let path_str = self.interner.read(|i| {
            i.resolve(import_path_symbol)
                .unwrap_or_default()
                .to_string()
        });
        let relative_path = Path::new(&path_str);

        let module_dir = self.module.path.parent().unwrap_or_else(|| Path::new("."));
        let full_path = module_dir.join(relative_path).with_extension("goida");

        let _file_stem = full_path
            .file_stem()
            .and_then(|s| s.to_str())
            .ok_or_else(|| {
                ParseError::ImportError(ErrorData::new(
                    span,
                    format!("Неверный путь: {}", full_path.display()),
                ))
            })?;

        let normalized_full_path = full_path
            .canonicalize()
            .unwrap_or(full_path.clone())
            .to_string_lossy()
            .to_string();
        let module_symbol = self
            .interner
            .write(|i| i.get_or_intern(normalized_full_path.as_str()));

        if self.module.modules.get(&module_symbol).is_some() {
            return Ok(module_symbol);
        }

        let code = std::fs::read_to_string(&full_path).map_err(|e| {
            ParseError::ImportError(ErrorData::new(
                span,
                format!("Не нашел файл {}: {}", full_path.display(), e),
            ))
        })?;

        let sub_parser = ParserTrait::new(
            self.interner.clone(),
            normalized_full_path.as_str(),
            full_path.clone(),
        );
        let new_module = sub_parser.parse(&code)?;

        for (class_name_symbol, _) in &new_module.classes {
            let class_name = self.interner.read(|i| {
                i.resolve(*class_name_symbol)
                    .unwrap_or_default()
                    .to_string()
            });
            self.module
                .arena
                .register_custom_type(&self.interner, class_name.as_str());
        }

        self.module.modules.insert(module_symbol, new_module);

        Ok(module_symbol)
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
            if token.as_rule() == Rule::logical_or_op {
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
            if token.as_rule() == Rule::logical_and_op {
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
        }

        Ok(left)
    }

    fn parse_unary(&mut self, pair: pest::iterators::Pair<Rule>) -> Result<ExprId, ParseError> {
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

    fn parse_arg_list(
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
