use goida_core::ast::prelude::{ExpressionKind, Span, StatementKind, StmtId};
use goida_core::ast::program::MethodType;
use goida_core::interpreter::prelude::{Module, SharedInterner};
use std::collections::HashMap;

#[derive(Clone)]
pub(crate) struct ResolvedSymbol {
    pub(crate) name: String,
    pub(crate) span: Span,
}

pub(crate) fn find_top_level_symbol(
    module: &Module,
    interner: &SharedInterner,
    name: &str,
) -> Option<ResolvedSymbol> {
    for function in module.functions.values() {
        if let Some(function_name) = module.arena.resolve_symbol(interner, function.name) {
            if function_name == name {
                return Some(ResolvedSymbol {
                    name: function_name,
                    span: function.span,
                });
            }
        }
    }

    for class in module.classes.values() {
        let class_match = class.read(|class_def| {
            module
                .arena
                .resolve_symbol(interner, class_def.name)
                .filter(|class_name| class_name == name)
                .map(|class_name| ResolvedSymbol {
                    name: class_name,
                    span: class_def.span,
                })
        });
        if class_match.is_some() {
            return class_match;
        }
    }

    for stmt_id in &module.body {
        let Some(statement) = module.arena.get_statement(*stmt_id) else {
            continue;
        };
        if let StatementKind::Assign { name: symbol, .. } = statement.kind {
            if let Some(assign_name) = module.arena.resolve_symbol(interner, symbol) {
                if assign_name == name {
                    return Some(ResolvedSymbol {
                        name: assign_name,
                        span: statement.span,
                    });
                }
            }
        } else if let StatementKind::NativeLibraryDefinition(definition) = &statement.kind {
            for function in &definition.functions {
                if let Some(function_name) = module.arena.resolve_symbol(interner, function.name) {
                    if function_name == name {
                        return Some(ResolvedSymbol {
                            name: function_name,
                            span: function.span,
                        });
                    }
                }
            }

            for global in &definition.globals {
                if let Some(global_name) = module.arena.resolve_symbol(interner, global.name) {
                    if global_name == name {
                        return Some(ResolvedSymbol {
                            name: global_name,
                            span: global.span,
                        });
                    }
                }
            }
        }
    }

    None
}

pub(crate) fn collect_declarations(
    module: &Module,
    interner: &SharedInterner,
    statement_ids: &[StmtId],
    out: &mut Vec<ResolvedSymbol>,
) {
    for stmt_id in statement_ids {
        let Some(statement) = module.arena.get_statement(*stmt_id) else {
            continue;
        };
        match &statement.kind {
            StatementKind::Assign { name, value, .. } => {
                if let Some(name) = module.arena.resolve_symbol(interner, *name) {
                    out.push(ResolvedSymbol {
                        name,
                        span: statement.span,
                    });
                }
                collect_expression_declarations(module, interner, *value, out);
            }
            StatementKind::CompoundAssign { target, value, .. } => {
                collect_expression_declarations(module, interner, *target, out);
                collect_expression_declarations(module, interner, *value, out);
            }
            StatementKind::IndexAssign {
                object,
                index,
                value,
            } => {
                collect_expression_declarations(module, interner, *object, out);
                collect_expression_declarations(module, interner, *index, out);
                collect_expression_declarations(module, interner, *value, out);
            }
            StatementKind::PropertyAssign { object, value, .. } => {
                collect_expression_declarations(module, interner, *object, out);
                collect_expression_declarations(module, interner, *value, out);
            }
            StatementKind::Expression(expr) => {
                collect_expression_declarations(module, interner, *expr, out);
            }
            StatementKind::For {
                variable,
                init,
                condition,
                update,
                body,
            } => {
                if let Some(name) = module.arena.resolve_symbol(interner, *variable) {
                    out.push(ResolvedSymbol {
                        name,
                        span: statement.span,
                    });
                }
                collect_expression_declarations(module, interner, *init, out);
                collect_expression_declarations(module, interner, *condition, out);
                collect_declarations(module, interner, &[*update], out);
                collect_declarations(module, interner, &[*body], out);
            }
            StatementKind::ForEach {
                variable,
                iterable,
                body,
            } => {
                if let Some(name) = module.arena.resolve_symbol(interner, *variable) {
                    out.push(ResolvedSymbol {
                        name,
                        span: statement.span,
                    });
                }
                collect_expression_declarations(module, interner, *iterable, out);
                collect_declarations(module, interner, &[*body], out);
            }
            StatementKind::If {
                condition,
                then_body,
                else_body,
            } => {
                collect_expression_declarations(module, interner, *condition, out);
                collect_declarations(module, interner, &[*then_body], out);
                if let Some(else_body) = else_body {
                    collect_declarations(module, interner, &[*else_body], out);
                }
            }
            StatementKind::While { condition, body } => {
                collect_expression_declarations(module, interner, *condition, out);
                collect_declarations(module, interner, &[*body], out)
            }
            StatementKind::Raise { message, .. } => {
                if let Some(message) = message {
                    collect_expression_declarations(module, interner, *message, out);
                }
            }
            StatementKind::Thread { body } => collect_declarations(module, interner, &[*body], out),
            StatementKind::Try { body, handlers } => {
                collect_declarations(module, interner, &[*body], out);
                for handler in handlers {
                    if let Some(error_text) = handler.error_text {
                        if let Some(name) = module.arena.resolve_symbol(interner, error_text) {
                            out.push(ResolvedSymbol {
                                name,
                                span: statement.span,
                            });
                        }
                    }
                    collect_declarations(module, interner, &[handler.body], out);
                }
            }
            StatementKind::Block(items) => collect_declarations(module, interner, items, out),
            StatementKind::Return(expr) => {
                if let Some(expr) = expr {
                    collect_expression_declarations(module, interner, *expr, out);
                }
            }
            StatementKind::FunctionDefinition(function) => {
                if let Some(name) = module.arena.resolve_symbol(interner, function.name) {
                    out.push(ResolvedSymbol {
                        name,
                        span: function.span,
                    });
                }
                for param in &function.params {
                    if let Some(name) = module.arena.resolve_symbol(interner, param.name) {
                        out.push(ResolvedSymbol {
                            name,
                            span: param.span,
                        });
                    }
                    if let Some(default_value) = param.default_value {
                        collect_expression_declarations(module, interner, default_value, out);
                    }
                }
                collect_declarations(module, interner, &[function.body], out);
            }
            StatementKind::ClassDefinition(class_def) => {
                if let Some(name) = module.arena.resolve_symbol(interner, class_def.name) {
                    out.push(ResolvedSymbol {
                        name,
                        span: class_def.span,
                    });
                }
                if let Some(MethodType::User(constructor)) = &class_def.constructor {
                    for param in &constructor.params {
                        if let Some(name) = module.arena.resolve_symbol(interner, param.name) {
                            out.push(ResolvedSymbol {
                                name,
                                span: param.span,
                            });
                        }
                        if let Some(default_value) = param.default_value {
                            collect_expression_declarations(module, interner, default_value, out);
                        }
                    }
                    collect_declarations(module, interner, &[constructor.body], out);
                }
                for (_, _, method_type) in class_def.methods.values() {
                    if let MethodType::User(method) = method_type {
                        if let Some(name) = module.arena.resolve_symbol(interner, method.name) {
                            out.push(ResolvedSymbol {
                                name,
                                span: method.span,
                            });
                        }
                        for param in &method.params {
                            if let Some(name) = module.arena.resolve_symbol(interner, param.name) {
                                out.push(ResolvedSymbol {
                                    name,
                                    span: param.span,
                                });
                            }
                            if let Some(default_value) = param.default_value {
                                collect_expression_declarations(
                                    module,
                                    interner,
                                    default_value,
                                    out,
                                );
                            }
                        }
                        collect_declarations(module, interner, &[method.body], out);
                    }
                }
            }
            StatementKind::NativeLibraryDefinition(_) | StatementKind::Empty => {}
        }
    }
}

fn collect_expression_declarations(
    module: &Module,
    interner: &SharedInterner,
    expr_id: u32,
    out: &mut Vec<ResolvedSymbol>,
) {
    let Some(expr) = module.arena.get_expression(expr_id) else {
        return;
    };

    match &expr.kind {
        ExpressionKind::FunctionCall { function, args } => {
            collect_expression_declarations(module, interner, *function, out);
            for arg in args {
                collect_expression_declarations(module, interner, arg.value, out);
            }
        }
        ExpressionKind::MethodCall { object, args, .. } => {
            collect_expression_declarations(module, interner, *object, out);
            for arg in args {
                collect_expression_declarations(module, interner, arg.value, out);
            }
        }
        ExpressionKind::ObjectCreation { args, .. } => {
            for arg in args {
                collect_expression_declarations(module, interner, arg.value, out);
            }
        }
        ExpressionKind::Binary { left, right, .. } => {
            collect_expression_declarations(module, interner, *left, out);
            collect_expression_declarations(module, interner, *right, out);
        }
        ExpressionKind::Unary { operand, .. } => {
            collect_expression_declarations(module, interner, *operand, out);
        }
        ExpressionKind::Index { object, index } => {
            collect_expression_declarations(module, interner, *object, out);
            collect_expression_declarations(module, interner, *index, out);
        }
        ExpressionKind::PropertyAccess { object, .. } => {
            collect_expression_declarations(module, interner, *object, out);
        }
        ExpressionKind::Lambda { params, body } => {
            for param in params {
                if let Some(name) = module.arena.resolve_symbol(interner, param.name) {
                    out.push(ResolvedSymbol {
                        name,
                        span: param.span,
                    });
                }
                if let Some(default_value) = param.default_value {
                    collect_expression_declarations(module, interner, default_value, out);
                }
            }
            collect_declarations(module, interner, &[*body], out);
        }
        ExpressionKind::Identifier(_) | ExpressionKind::Literal(_) | ExpressionKind::This => {}
    }
}

pub(crate) fn collect_imports(
    module: &Module,
    interner: &SharedInterner,
) -> HashMap<String, String> {
    let mut imports = HashMap::new();
    for import in &module.imports {
        let Some(alias) = module.arena.resolve_symbol(interner, import.item.alias) else {
            continue;
        };
        let Some(path) = module.arena.resolve_symbol(interner, import.item.path) else {
            continue;
        };
        imports.insert(alias, path);
    }
    imports
}
