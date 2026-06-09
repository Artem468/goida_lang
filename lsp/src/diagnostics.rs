use goida_model::SharedInterner;
use goida_runtime::builtins::registry::BUILTINS;
use goida_runtime::interpreter::prelude::Module;
use goida_syntax::ast::prelude::{ExpressionKind, Span, StatementKind, StmtId};
use goida_syntax::ast::program::MethodType;
use std::collections::HashSet;
use tower_lsp::lsp_types::{Diagnostic, DiagnosticSeverity, Position, Range};

#[derive(Clone)]
struct Declaration {
    name: String,
    span: Span,
    kind: &'static str,
}

pub(crate) fn collect_lsp_diagnostics(
    module: &Module,
    interner: &SharedInterner,
    text: &str,
    line_starts: &[usize],
) -> Vec<Diagnostic> {
    let mut declarations = Vec::new();
    collect_declarations(module, interner, &module.body, &mut declarations);
    collect_top_level_declarations(module, interner, &mut declarations);

    let mut known = BUILTINS
        .known_global_names()
        .map(str::to_string)
        .collect::<HashSet<_>>();
    for method in BUILTINS.methods() {
        known.extend(method.names.iter().map(|name| (*name).to_string()));
    }
    for declaration in &declarations {
        known.insert(declaration.name.clone());
    }

    let mut usages = Vec::new();
    collect_usages(module, interner, &module.body, &mut usages);

    let mut diagnostics = Vec::new();
    let mut seen_unknown = HashSet::new();
    for (name, span) in &usages {
        if known.contains(name) || name == "это" || name == "this" {
            continue;
        }
        if !seen_unknown.insert((name.clone(), span.start, span.end)) {
            continue;
        }
        diagnostics.push(make_diagnostic(
            text,
            line_starts,
            *span,
            format!("Имя '{}' не найдено", name),
            DiagnosticSeverity::ERROR,
        ));
    }

    let used = usages
        .into_iter()
        .map(|(name, _)| name)
        .collect::<HashSet<_>>();
    let mut seen_unused = HashSet::new();
    for declaration in declarations {
        if declaration.name.starts_with('_')
            || used.contains(&declaration.name)
            || !seen_unused.insert((declaration.name.clone(), declaration.span.start))
        {
            continue;
        }
        diagnostics.push(make_diagnostic(
            text,
            line_starts,
            declaration.span,
            format!(
                "{} '{}' объявлен, но не используется",
                declaration.kind, declaration.name
            ),
            DiagnosticSeverity::WARNING,
        ));
    }

    diagnostics
}

fn collect_top_level_declarations(
    module: &Module,
    interner: &SharedInterner,
    out: &mut Vec<Declaration>,
) {
    for import in &module.imports {
        if let Some(name) = module.arena.resolve_symbol(interner, import.item.alias) {
            out.push(Declaration {
                name,
                span: import.span,
                kind: "Импорт",
            });
        }
    }
    for function in module.functions.values() {
        if let Some(name) = module.arena.resolve_symbol(interner, function.name) {
            out.push(Declaration {
                name,
                span: function.span,
                kind: "Функция",
            });
        }
    }
    for class in module.classes.values() {
        class.read(|class_def| {
            if let Some(name) = module.arena.resolve_symbol(interner, class_def.name) {
                out.push(Declaration {
                    name,
                    span: class_def.span,
                    kind: "Класс",
                });
            }
        });
    }
}

fn collect_declarations(
    module: &Module,
    interner: &SharedInterner,
    statement_ids: &[StmtId],
    out: &mut Vec<Declaration>,
) {
    for stmt_id in statement_ids {
        let Some(statement) = module.arena.get_statement(*stmt_id) else {
            continue;
        };
        match &statement.kind {
            StatementKind::Assign { name, value, .. } => {
                if let Some(name) = module.arena.resolve_symbol(interner, *name) {
                    out.push(Declaration {
                        name,
                        span: statement.span,
                        kind: "Переменная",
                    });
                }
                collect_expression_declarations(module, interner, *value, out);
            }
            StatementKind::For {
                variable,
                update,
                body,
                ..
            } => {
                if let Some(name) = module.arena.resolve_symbol(interner, *variable) {
                    out.push(Declaration {
                        name,
                        span: statement.span,
                        kind: "Переменная",
                    });
                }
                collect_declarations(module, interner, &[*update], out);
                collect_declarations(module, interner, &[*body], out);
            }
            StatementKind::ForEach { variable, body, .. } => {
                if let Some(name) = module.arena.resolve_symbol(interner, *variable) {
                    out.push(Declaration {
                        name,
                        span: statement.span,
                        kind: "Переменная",
                    });
                }
                collect_declarations(module, interner, &[*body], out);
            }
            StatementKind::Block(items) => collect_declarations(module, interner, items, out),
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
            StatementKind::While { body, .. } | StatementKind::Thread { body } => {
                collect_declarations(module, interner, &[*body], out);
            }
            StatementKind::Try { body, handlers } => {
                collect_declarations(module, interner, &[*body], out);
                for handler in handlers {
                    if let Some(error_text) = handler.error_text {
                        if let Some(name) = module.arena.resolve_symbol(interner, error_text) {
                            out.push(Declaration {
                                name,
                                span: statement.span,
                                kind: "Переменная",
                            });
                        }
                    }
                    collect_declarations(module, interner, &[handler.body], out);
                }
            }
            StatementKind::FunctionDefinition(function) => {
                if let Some(name) = module.arena.resolve_symbol(interner, function.name) {
                    out.push(Declaration {
                        name,
                        span: function.span,
                        kind: "Функция",
                    });
                }
                collect_function_declarations(module, interner, function, out);
            }
            StatementKind::ClassDefinition(class_def) => {
                if let Some(name) = module.arena.resolve_symbol(interner, class_def.name) {
                    out.push(Declaration {
                        name,
                        span: class_def.span,
                        kind: "Класс",
                    });
                }
                if let Some(MethodType::User(constructor)) = &class_def.constructor {
                    collect_function_declarations(module, interner, constructor, out);
                }
                for (_, _, method) in class_def.methods.values() {
                    if let MethodType::User(function) = method {
                        collect_function_declarations(module, interner, function, out);
                    }
                }
            }
            _ => {}
        }
    }
}

fn collect_function_declarations(
    module: &Module,
    interner: &SharedInterner,
    function: &goida_syntax::ast::prelude::FunctionDefinition,
    out: &mut Vec<Declaration>,
) {
    for param in &function.params {
        if let Some(name) = module.arena.resolve_symbol(interner, param.name) {
            out.push(Declaration {
                name,
                span: param.span,
                kind: "Параметр",
            });
        }
        if let Some(default_value) = param.default_value {
            collect_expression_declarations(module, interner, default_value, out);
        }
    }
    collect_declarations(module, interner, &[function.body], out);
}

fn collect_expression_declarations(
    module: &Module,
    interner: &SharedInterner,
    expr_id: u32,
    out: &mut Vec<Declaration>,
) {
    let Some(expr) = module.arena.get_expression(expr_id) else {
        return;
    };
    if let ExpressionKind::Lambda { params, body } = &expr.kind {
        for param in params {
            if let Some(name) = module.arena.resolve_symbol(interner, param.name) {
                out.push(Declaration {
                    name,
                    span: param.span,
                    kind: "Параметр",
                });
            }
        }
        collect_declarations(module, interner, &[*body], out);
    }
}

fn collect_usages(
    module: &Module,
    interner: &SharedInterner,
    statement_ids: &[StmtId],
    out: &mut Vec<(String, Span)>,
) {
    for stmt_id in statement_ids {
        let Some(statement) = module.arena.get_statement(*stmt_id) else {
            continue;
        };
        match &statement.kind {
            StatementKind::Expression(expr) => {
                collect_expression_usages(module, interner, *expr, out)
            }
            StatementKind::Assign { value, .. } => {
                collect_expression_usages(module, interner, *value, out)
            }
            StatementKind::CompoundAssign { target, value, .. } => {
                collect_expression_usages(module, interner, *target, out);
                collect_expression_usages(module, interner, *value, out);
            }
            StatementKind::IndexAssign {
                object,
                index,
                value,
            } => {
                collect_expression_usages(module, interner, *object, out);
                collect_expression_usages(module, interner, *index, out);
                collect_expression_usages(module, interner, *value, out);
            }
            StatementKind::PropertyAssign { object, value, .. } => {
                collect_expression_usages(module, interner, *object, out);
                collect_expression_usages(module, interner, *value, out);
            }
            StatementKind::If {
                condition,
                then_body,
                else_body,
            } => {
                collect_expression_usages(module, interner, *condition, out);
                collect_usages(module, interner, &[*then_body], out);
                if let Some(else_body) = else_body {
                    collect_usages(module, interner, &[*else_body], out);
                }
            }
            StatementKind::While { condition, body } => {
                collect_expression_usages(module, interner, *condition, out);
                collect_usages(module, interner, &[*body], out);
            }
            StatementKind::For {
                init,
                condition,
                update,
                body,
                ..
            } => {
                collect_expression_usages(module, interner, *init, out);
                collect_expression_usages(module, interner, *condition, out);
                collect_usages(module, interner, &[*update], out);
                collect_usages(module, interner, &[*body], out);
            }
            StatementKind::ForEach { iterable, body, .. } => {
                collect_expression_usages(module, interner, *iterable, out);
                collect_usages(module, interner, &[*body], out);
            }
            StatementKind::Thread { body } => {
                collect_usages(module, interner, &[*body], out);
            }
            StatementKind::Block(body) => {
                collect_usages(module, interner, body, out);
            }
            StatementKind::Try { body, handlers } => {
                collect_usages(module, interner, &[*body], out);
                for handler in handlers {
                    collect_usages(module, interner, &[handler.body], out);
                }
            }
            StatementKind::Raise {
                error_type,
                message,
            } => {
                if let Some(name) = module.arena.resolve_symbol(interner, *error_type) {
                    out.push((name, statement.span));
                }
                if let Some(message) = message {
                    collect_expression_usages(module, interner, *message, out);
                }
            }
            StatementKind::Return(expr) => {
                if let Some(expr) = expr {
                    collect_expression_usages(module, interner, *expr, out);
                }
            }
            StatementKind::FunctionDefinition(function) => {
                collect_usages(module, interner, &[function.body], out);
            }
            StatementKind::ClassDefinition(class_def) => {
                if let Some(base_class) = class_def.base_class {
                    if let Some(name) = module.arena.resolve_symbol(interner, base_class) {
                        out.push((name, class_def.span));
                    }
                }
                if let Some(MethodType::User(constructor)) = &class_def.constructor {
                    collect_usages(module, interner, &[constructor.body], out);
                }
                for (_, _, method) in class_def.methods.values() {
                    if let MethodType::User(function) = method {
                        collect_usages(module, interner, &[function.body], out);
                    }
                }
            }
            StatementKind::NativeLibraryDefinition(_) | StatementKind::Empty => {}
        }
    }
}

fn collect_expression_usages(
    module: &Module,
    interner: &SharedInterner,
    expr_id: u32,
    out: &mut Vec<(String, Span)>,
) {
    let Some(expr) = module.arena.get_expression(expr_id) else {
        return;
    };
    match &expr.kind {
        ExpressionKind::Identifier(symbol) => {
            if let Some(name) = module.arena.resolve_symbol(interner, *symbol) {
                out.push((name, expr.span));
            }
        }
        ExpressionKind::FunctionCall { function, args } => {
            collect_expression_usages(module, interner, *function, out);
            for arg in args {
                collect_expression_usages(module, interner, arg.value, out);
            }
        }
        ExpressionKind::MethodCall { object, args, .. } => {
            collect_expression_usages(module, interner, *object, out);
            for arg in args {
                collect_expression_usages(module, interner, arg.value, out);
            }
        }
        ExpressionKind::ObjectCreation { class_name, args } => {
            if let Some(name) = module.arena.resolve_symbol(interner, *class_name) {
                out.push((name, expr.span));
            }
            for arg in args {
                collect_expression_usages(module, interner, arg.value, out);
            }
        }
        ExpressionKind::Binary { left, right, .. } => {
            collect_expression_usages(module, interner, *left, out);
            collect_expression_usages(module, interner, *right, out);
        }
        ExpressionKind::Unary { operand, .. } => {
            collect_expression_usages(module, interner, *operand, out)
        }
        ExpressionKind::Index { object, index } => {
            collect_expression_usages(module, interner, *object, out);
            collect_expression_usages(module, interner, *index, out);
        }
        ExpressionKind::PropertyAccess { object, .. } => {
            collect_expression_usages(module, interner, *object, out);
        }
        ExpressionKind::Lambda { body, .. } => collect_usages(module, interner, &[*body], out),
        ExpressionKind::Literal(_) | ExpressionKind::This => {}
    }
}

fn make_diagnostic(
    text: &str,
    line_starts: &[usize],
    span: Span,
    message: String,
    severity: DiagnosticSeverity,
) -> Diagnostic {
    let range = span.as_ariadne(text);
    let start = char_offset_to_position(line_starts, range.start).unwrap_or(Position::new(0, 0));
    let end = char_offset_to_position(line_starts, range.end).unwrap_or(start);
    Diagnostic {
        range: Range::new(start, end),
        message,
        severity: Some(severity),
        source: Some("goida-lsp".into()),
        ..Default::default()
    }
}

fn char_offset_to_position(line_starts: &[usize], char_offset: usize) -> Option<Position> {
    let line = match line_starts.binary_search(&char_offset) {
        Ok(line) => line,
        Err(0) => 0,
        Err(next_line) => next_line.saturating_sub(1),
    };
    let col = char_offset.saturating_sub(*line_starts.get(line)?);
    Some(Position::new(line as u32, col as u32))
}

#[cfg(test)]
mod tests {
    use super::collect_lsp_diagnostics;
    use crate::document::Document;
    use goida_model::new_interner;
    use goida_runtime::parser::prelude::Parser;
    use std::fs;
    use std::path::PathBuf;

    #[test]
    fn reports_multiple_unknown_names_and_unused_declarations() {
        let source = "unused = 1\nvalue = missing_one + missing_two\n";
        let interner = new_interner();
        let module = Parser::new(
            interner.clone(),
            "diagnostics_test",
            PathBuf::from("test.goida"),
        )
        .parse_syntax(source)
        .expect("source should parse without name validation");
        let document = Document::new(source);

        let diagnostics =
            collect_lsp_diagnostics(&module, &interner, document.text(), document.line_starts());
        let messages = diagnostics
            .iter()
            .map(|diagnostic| diagnostic.message.as_str())
            .collect::<Vec<_>>();

        assert!(messages
            .iter()
            .any(|message| message.contains("missing_one")));
        assert!(messages
            .iter()
            .any(|message| message.contains("missing_two")));
        assert!(messages.iter().any(|message| message.contains("unused")));
    }

    #[test]
    fn reports_unused_import_alias() {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("target")
            .join("lsp_unused_import_warning_test");
        if root.exists() {
            fs::remove_dir_all(&root).expect("failed to clear test dir");
        }
        fs::create_dir_all(&root).expect("failed to create test dir");
        fs::write(root.join("module.goida"), "exported = 1\n").expect("failed to write module");

        let source = "import \"module\" as m\nvalue = 1\n";
        let main_path = root.join("main.goida");
        let interner = new_interner();
        let module = Parser::new(interner.clone(), "main", main_path)
            .parse_syntax(source)
            .expect("source should parse");
        let document = Document::new(source);

        let diagnostics =
            collect_lsp_diagnostics(&module, &interner, document.text(), document.line_starts());

        assert!(diagnostics
            .iter()
            .any(|diagnostic| diagnostic.message.contains("m")
                && diagnostic.severity == Some(tower_lsp::lsp_types::DiagnosticSeverity::WARNING)));
    }
}
