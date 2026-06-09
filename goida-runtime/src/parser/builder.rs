mod expression;
mod statement;

use crate::ast::prelude::*;
use crate::interpreter::prelude::{RuntimeClassDefinition, RuntimeFieldData, Value};
use crate::parser::prelude::{ParseError, Parser as ParserTrait};
use crate::parser::syntax as syn;
use crate::shared::SharedMut;
use std::ops::Range;
use std::sync::Arc;

impl ParserTrait {
    pub(crate) fn build_program(&mut self, program: syn::Program) -> Result<(), ParseError> {
        for item in program.items {
            let stmt = self.build_item(item, true)?;
            self.module.body.push(stmt);
        }
        Ok(())
    }

    fn build_items_as_block(&mut self, items: Vec<syn::Item>) -> Result<Vec<StmtId>, ParseError> {
        let mut statements = Vec::new();
        for item in items {
            statements.push(self.build_item(item, false)?);
        }
        Ok(statements)
    }

    fn build_item(&mut self, item: syn::Item, top_level: bool) -> Result<StmtId, ParseError> {
        let span = self.span(item.span.clone());
        match item.node {
            syn::ItemKind::Import(import) => self.build_import(import, span),
            syn::ItemKind::Function(function) => {
                self.build_function(function, item.span, top_level)
            }
            syn::ItemKind::Class(class) => self.build_class(class, item.span),
            syn::ItemKind::Library(library) => self.build_library(library, item.span),
            syn::ItemKind::MacroDefinition(_) => Err(ParseError::InvalidSyntax(ErrorData::new(
                span,
                "Макросы должны быть раскрыты до построения AST".into(),
            ))),
            syn::ItemKind::Statement(stmt) => self.build_statement(*stmt),
        }
    }

    fn build_import(&mut self, import: syn::Import, span: Span) -> Result<StmtId, ParseError> {
        let path_symbol = self
            .module
            .arena
            .intern_string(&self.interner, &import.path);
        let alias_symbol = self
            .module
            .arena
            .intern_string(&self.interner, &import.alias);

        self.module.imports.push(Import {
            item: ImportItem {
                path: path_symbol,
                alias: alias_symbol,
            },
            span,
        });

        let module = self.parse_and_register_import(path_symbol, span)?;
        self.register_imported_type_aliases(alias_symbol, module);
        self.module
            .globals
            .insert(alias_symbol, Value::Module(module));

        Ok(self.module.arena.add_statement(
            StatementKind::Import(ImportItem {
                path: path_symbol,
                alias: alias_symbol,
            }),
            span,
        ))
    }

    fn build_function(
        &mut self,
        function: syn::Function,
        span: Range<usize>,
        top_level: bool,
    ) -> Result<StmtId, ParseError> {
        let func_span = self.span(span);
        let name = self.intern(&function.name);
        let params = self.build_params(function.params)?;
        let return_type = self.build_optional_type(function.return_type, func_span)?;
        let body_items = self.build_items_as_block(function.body)?;
        let body = self
            .module
            .arena
            .add_statement(StatementKind::Block(body_items), func_span);
        let definition = FunctionDefinition {
            name,
            params,
            return_type,
            body,
            span: func_span,
            module: None,
        };

        if top_level {
            self.module.functions.insert(name, Arc::new(definition));
            Ok(self
                .module
                .arena
                .add_statement(StatementKind::Empty, func_span))
        } else {
            Ok(self
                .module
                .arena
                .add_statement(StatementKind::FunctionDefinition(definition), func_span))
        }
    }

    fn build_library(
        &mut self,
        library: syn::Library,
        span: Range<usize>,
    ) -> Result<StmtId, ParseError> {
        let library_span = self.span(span);
        let path = self.intern(&library.path);
        let mut functions = Vec::new();
        let mut globals = Vec::new();

        for item in library.items {
            let item_span = self.span(item.span);
            match item.node {
                syn::LibraryItemKind::Function(function) => {
                    functions.push(NativeFunctionDefinition {
                        name: self.intern(&function.name),
                        params: self.build_library_params(function.params)?,
                        return_type: self.build_optional_type(function.return_type, item_span)?,
                        span: item_span,
                    });
                }
                syn::LibraryItemKind::Global(global) => {
                    globals.push(NativeGlobalDefinition {
                        name: self.intern(&global.name),
                        value_type: self.build_type(&global.type_name, item_span)?,
                        span: item_span,
                    });
                }
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

    fn build_class(&mut self, class: syn::Class, span: Range<usize>) -> Result<StmtId, ParseError> {
        let class_span = self.span(span);
        self.module
            .arena
            .register_custom_type(&self.interner, &class.name);
        let name = self.intern(&class.name);
        let mut base_class = None;
        if let Some(base) = class.base {
            let base_symbol = self.intern(&base);
            if !self.module.classes.contains_key(&base_symbol) {
                return Err(ParseError::TypeError(ErrorData::new(
                    class_span,
                    format!("Базовый класс '{}' не найден", base),
                )));
            }
            base_class = Some(base_symbol);
        }

        let mut class_def = RuntimeClassDefinition::new_with_base(name, base_class, class_span);
        if let Some(base_symbol) = base_class {
            if let Some(base_def) = self.module.classes.get(&base_symbol) {
                base_def.read(|base| {
                    class_def.fields.extend(base.fields.clone());
                    class_def.methods.extend(base.methods.clone());
                    class_def.constructor = base.constructor.clone();
                });
            }
        }

        for item in class.items {
            let item_span = self.span(item.span);
            match item.node {
                syn::ClassItemKind::Field(field) => {
                    let name = self.intern(&field.name);
                    let field_type = self.build_type(&field.type_name, item_span)?;
                    let default_value = match field.default_value {
                        Some(expr) => Some(self.build_expr(expr)?),
                        None => None,
                    };
                    class_def.add_field(
                        name,
                        self.visibility(field.visibility),
                        field.is_static,
                        RuntimeFieldData::Expression(default_value),
                    );
                    let _ = field_type;
                }
                syn::ClassItemKind::Constructor(method) => {
                    let function = self.build_method_function(method, item_span)?;
                    class_def.set_constructor(function);
                }
                syn::ClassItemKind::Method(method) => {
                    let method_name = self.intern(&method.name);
                    let visibility = self.visibility(method.visibility.clone());
                    let is_static = method.is_static;
                    let function = self.build_method_function(method, item_span)?;
                    class_def.add_method(method_name, visibility, is_static, function);
                }
            }
        }

        self.module
            .classes
            .insert(name, SharedMut::new(class_def.clone()));
        Ok(self.module.arena.add_statement(
            StatementKind::ClassDefinition(class_def.to_syntax()),
            class_span,
        ))
    }

    fn build_method_function(
        &mut self,
        method: syn::ClassMethod,
        span: Span,
    ) -> Result<FunctionDefinition, ParseError> {
        let body_items = self.build_items_as_block(method.body)?;
        let body = self
            .module
            .arena
            .add_statement(StatementKind::Block(body_items), span);
        Ok(FunctionDefinition {
            name: self.intern(&method.name),
            params: self.build_params(method.params)?,
            return_type: self.build_optional_type(method.return_type, span)?,
            body,
            span,
            module: None,
        })
    }

    fn build_params(&mut self, params: Vec<syn::Param>) -> Result<Vec<Parameter>, ParseError> {
        let mut output = Vec::new();
        let mut saw_default = false;
        for param in params {
            let span = self.span(param.span);
            let default_value = param
                .default_value
                .map(|expr| self.build_expr(expr))
                .transpose()?;
            if default_value.is_some() {
                saw_default = true;
            } else if saw_default {
                return Err(ParseError::TypeError(ErrorData::new(
                    span,
                    format!(
                        "Обязательный параметр '{}' не может следовать за параметром со значением по умолчанию",
                        param.name
                    ),
                )));
            }
            let param_type = match param.type_name {
                Some(type_name) => self.build_type(&type_name, span)?,
                None => self
                    .module
                    .arena
                    .register_custom_type(&self.interner, "неизвестно"),
            };
            output.push(Parameter {
                name: self.intern(&param.name),
                param_type,
                default_value,
                span,
            });
        }
        Ok(output)
    }

    fn build_library_params(
        &mut self,
        params: Vec<syn::LibraryParam>,
    ) -> Result<Vec<Parameter>, ParseError> {
        let mut output = Vec::new();
        for param in params {
            let span = self.span(param.span);
            output.push(Parameter {
                name: self.intern(&param.name),
                param_type: self.build_type(&param.type_name, span)?,
                default_value: None,
                span,
            });
        }
        Ok(output)
    }

    fn build_call_args(&mut self, args: Vec<syn::CallArg>) -> Result<Vec<CallArg>, ParseError> {
        let mut output = Vec::new();
        for arg in args {
            output.push(CallArg {
                name: arg.name.map(|name| self.intern(&name)),
                value: self.build_expr(arg.value)?,
            });
        }
        Ok(output)
    }

    fn build_optional_type(
        &mut self,
        type_name: Option<String>,
        span: Span,
    ) -> Result<Option<TypeId>, ParseError> {
        type_name
            .map(|type_name| self.build_type(&type_name, span))
            .transpose()
    }

    fn build_type(&mut self, type_name: &str, span: Span) -> Result<TypeId, ParseError> {
        self.module
            .arena
            .find_type_by_name(&self.interner, type_name)
            .ok_or_else(|| {
                ParseError::TypeError(ErrorData::new(
                    span,
                    format!("Неизвестный тип: {}", type_name),
                ))
            })
    }

    fn visibility(&self, visibility: Option<syn::Visibility>) -> Visibility {
        match visibility {
            Some(syn::Visibility::Public) => Visibility::Public,
            Some(syn::Visibility::Private) | None => Visibility::Private,
        }
    }

    fn binary_op(&self, op: syn::BinaryOp) -> BinaryOperator {
        match op {
            syn::BinaryOp::Add => BinaryOperator::Add,
            syn::BinaryOp::Sub => BinaryOperator::Sub,
            syn::BinaryOp::Mul => BinaryOperator::Mul,
            syn::BinaryOp::Div => BinaryOperator::Div,
            syn::BinaryOp::Mod => BinaryOperator::Mod,
            syn::BinaryOp::Eq => BinaryOperator::Eq,
            syn::BinaryOp::Ne => BinaryOperator::Ne,
            syn::BinaryOp::Lt => BinaryOperator::Lt,
            syn::BinaryOp::Le => BinaryOperator::Le,
            syn::BinaryOp::Gt => BinaryOperator::Gt,
            syn::BinaryOp::Ge => BinaryOperator::Ge,
            syn::BinaryOp::And => BinaryOperator::And,
            syn::BinaryOp::Or => BinaryOperator::Or,
        }
    }

    fn compound_op(&self, op: syn::CompoundOp) -> BinaryOperator {
        match op {
            syn::CompoundOp::Add => BinaryOperator::Add,
            syn::CompoundOp::Sub => BinaryOperator::Sub,
            syn::CompoundOp::Mul => BinaryOperator::Mul,
            syn::CompoundOp::Div => BinaryOperator::Div,
            syn::CompoundOp::Mod => BinaryOperator::Mod,
        }
    }

    fn intern(&self, value: &str) -> string_interner::DefaultSymbol {
        self.module.arena.intern_string(&self.interner, value)
    }

    fn span(&self, span: Range<usize>) -> Span {
        Span::new(span.start, span.end, self.module.name)
    }
}
