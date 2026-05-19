use crate::ast::prelude::*;
use crate::ast::program::{FieldData, MethodType};
use crate::interpreter::prelude::{Module, Value};
use crate::parser::prelude::{ParseError, Parser as ParserTrait};
use std::collections::HashSet;
use string_interner::DefaultSymbol as Symbol;

impl ParserTrait {
    pub(crate) fn validate_module_names(&self) -> Result<(), ParseError> {
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

    pub(crate) fn known_global_names(&self) -> HashSet<Symbol> {
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
            "Поток",
            "Мьютекс",
            "БлокировкаЧтенияЗаписи",
        ] {
            names.insert(self.module.arena.intern_string(&self.interner, name));
        }

        names
    }

    pub(crate) fn collect_module_exported_names(
        &self,
        module: &Module,
        names: &mut HashSet<Symbol>,
    ) {
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

    pub(crate) fn validate_statement_names(
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
            StatementKind::Thread { body } => self.validate_thread_body_names(*body, scopes),
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

    fn validate_thread_body_names(
        &self,
        stmt_id: StmtId,
        scopes: &mut Vec<HashSet<Symbol>>,
    ) -> Result<(), ParseError> {
        let stmt = self.module.arena.get_statement(stmt_id).unwrap();
        if let StatementKind::Block(statements) = &stmt.kind {
            for stmt_id in statements {
                self.validate_statement_names(*stmt_id, scopes)?;
            }
            Ok(())
        } else {
            self.validate_statement_names(stmt_id, scopes)
        }
    }

    pub(crate) fn validate_function_body_names(
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

    pub(crate) fn validate_expression_names(
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

    pub(crate) fn is_name_known(&self, symbol: Symbol, scopes: &[HashSet<Symbol>]) -> bool {
        if scopes.iter().rev().any(|scope| scope.contains(&symbol)) {
            return true;
        }

        let Some(name) = self.module.arena.resolve_symbol(&self.interner, symbol) else {
            return false;
        };
        let parts = name.split('.').collect::<Vec<_>>();
        if parts.len() > 1 {
            let module_symbol = self.module.arena.intern_string(&self.interner, parts[0]);
            if !scopes
                .iter()
                .rev()
                .any(|scope| scope.contains(&module_symbol))
            {
                return false;
            }

            let member_name = parts.last().copied().unwrap_or_default();
            let member_symbol = self.module.arena.intern_string(&self.interner, member_name);
            return self
                .resolve_module_path_for_validation(&self.module, &parts[..parts.len() - 1])
                .map(|module| {
                    module.functions.contains_key(&member_symbol)
                        || module.classes.contains_key(&member_symbol)
                        || module.globals.contains_key(&member_symbol)
                })
                .unwrap_or(false);
        }

        false
    }

    fn resolve_module_path_for_validation<'a>(
        &'a self,
        current_module: &'a Module,
        parts: &[&str],
    ) -> Option<&'a Module> {
        let (first, rest) = parts.split_first()?;
        let first_symbol = self.module.arena.intern_string(&self.interner, first);
        let mut module = self.resolve_import_alias_for_validation(current_module, first_symbol)?;

        for part in rest {
            let part_symbol = self.module.arena.intern_string(&self.interner, part);
            module = self.resolve_import_alias_for_validation(module, part_symbol)?;
        }

        Some(module)
    }

    fn resolve_import_alias_for_validation<'a>(
        &'a self,
        module: &'a Module,
        alias: Symbol,
    ) -> Option<&'a Module> {
        match module.globals.get(&alias) {
            Some(Value::Module(module_symbol)) => module.modules.get(module_symbol),
            _ => None,
        }
    }
}
