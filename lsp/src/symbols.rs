use goida_core::ast::prelude::{Span, StatementKind, StmtId};
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
            StatementKind::Assign { name, .. } => {
                if let Some(name) = module.arena.resolve_symbol(interner, *name) {
                    out.push(ResolvedSymbol {
                        name,
                        span: statement.span,
                    });
                }
            }
            StatementKind::For { variable, body, .. } => {
                if let Some(name) = module.arena.resolve_symbol(interner, *variable) {
                    out.push(ResolvedSymbol {
                        name,
                        span: statement.span,
                    });
                }
                collect_declarations(module, interner, &[*body], out);
            }
            StatementKind::ForEach { variable, body, .. } => {
                if let Some(name) = module.arena.resolve_symbol(interner, *variable) {
                    out.push(ResolvedSymbol {
                        name,
                        span: statement.span,
                    });
                }
                collect_declarations(module, interner, &[*body], out);
            }
            StatementKind::If {
                then_body,
                else_body,
                ..
            } => {
                collect_declarations(module, interner, &[*then_body], out);
                if let Some(else_body) = else_body {
                    collect_declarations(module, interner, &[*else_body], out);
                }
            }
            StatementKind::While { body, .. } => {
                collect_declarations(module, interner, &[*body], out)
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
                        }
                        collect_declarations(module, interner, &[method.body], out);
                    }
                }
            }
            StatementKind::Expression(_)
            | StatementKind::CompoundAssign { .. }
            | StatementKind::IndexAssign { .. }
            | StatementKind::PropertyAssign { .. }
            | StatementKind::Raise { .. }
            | StatementKind::Return(_)
            | StatementKind::NativeLibraryDefinition(_)
            | StatementKind::Empty => {}
        }
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
