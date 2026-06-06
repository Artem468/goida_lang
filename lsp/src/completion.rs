use goida_core::ast::prelude::StatementKind;
use goida_core::builtins::registry::BUILTINS;
use goida_core::interpreter::prelude::{Module, SharedInterner};
use tower_lsp::lsp_types::{CompletionItem, CompletionItemKind};

pub(crate) fn completion_items(
    module: Option<&Module>,
    interner: &SharedInterner,
) -> Vec<CompletionItem> {
    let mut items = Vec::new();

    for entry in BUILTINS.functions() {
        push_names(
            &mut items,
            entry.names,
            CompletionItemKind::FUNCTION,
            "builtin function",
        );
    }
    for entry in BUILTINS.macros() {
        push_names(
            &mut items,
            entry.names,
            CompletionItemKind::FUNCTION,
            "builtin macro",
        );
    }
    for class in BUILTINS.classes() {
        push_names(
            &mut items,
            class.names.names,
            CompletionItemKind::CLASS,
            "builtin class",
        );
    }
    for ty in BUILTINS.types() {
        push_names(
            &mut items,
            ty.names,
            CompletionItemKind::TYPE_PARAMETER,
            "builtin type",
        );
    }
    for entry in BUILTINS.methods() {
        push_names(
            &mut items,
            entry.names,
            CompletionItemKind::METHOD,
            "builtin method",
        );
    }

    if let Some(module) = module {
        add_module_items(&mut items, module, interner);
    }

    items.sort_by(|left, right| left.label.cmp(&right.label));
    items.dedup_by(|left, right| left.label == right.label && left.kind == right.kind);
    items
}

pub(crate) fn module_member_completion_items(
    module: &Module,
    interner: &SharedInterner,
) -> Vec<CompletionItem> {
    let mut items = Vec::new();
    add_module_items(&mut items, module, interner);
    items.sort_by(|left, right| left.label.cmp(&right.label));
    items
}

fn add_module_items(items: &mut Vec<CompletionItem>, module: &Module, interner: &SharedInterner) {
    for import in &module.imports {
        if let Some(alias) = module.arena.resolve_symbol(interner, import.item.alias) {
            items.push(item(alias, CompletionItemKind::MODULE, "import alias"));
        }
    }
    for function in module.functions.values() {
        if let Some(name) = module.arena.resolve_symbol(interner, function.name) {
            items.push(item(name, CompletionItemKind::FUNCTION, "function"));
        }
    }
    for class in module.classes.values() {
        class.read(|class_def| {
            if let Some(name) = module.arena.resolve_symbol(interner, class_def.name) {
                items.push(item(name, CompletionItemKind::CLASS, "class"));
            }
        });
    }
    for stmt_id in &module.body {
        let Some(stmt) = module.arena.get_statement(*stmt_id) else {
            continue;
        };
        if let StatementKind::Assign { name, .. } = stmt.kind {
            if let Some(name) = module.arena.resolve_symbol(interner, name) {
                items.push(item(name, CompletionItemKind::VARIABLE, "variable"));
            }
        }
    }
}

fn push_names(
    items: &mut Vec<CompletionItem>,
    names: &[&str],
    kind: CompletionItemKind,
    detail: &'static str,
) {
    for name in names {
        items.push(item((*name).to_string(), kind, detail));
    }
}

fn item(label: String, kind: CompletionItemKind, detail: &'static str) -> CompletionItem {
    CompletionItem {
        label,
        kind: Some(kind),
        detail: Some(detail.to_string()),
        ..Default::default()
    }
}

#[cfg(test)]
mod tests {
    use super::completion_items;
    use goida_core::interpreter::prelude::SharedInterner;
    use goida_core::shared::SharedMut;
    use string_interner::StringInterner;
    use tower_lsp::lsp_types::CompletionItemKind;

    #[test]
    fn includes_builtin_functions_macros_classes_and_types() {
        let interner: SharedInterner = SharedMut::new(StringInterner::new());
        let items = completion_items(None, &interner);

        assert!(items
            .iter()
            .any(|item| item.label == "print" && item.kind == Some(CompletionItemKind::FUNCTION)));
        assert!(items
            .iter()
            .any(|item| item.label == "format" && item.kind == Some(CompletionItemKind::FUNCTION)));
        assert!(items
            .iter()
            .any(|item| item.label == "String" && item.kind == Some(CompletionItemKind::CLASS)));
        assert!(items
            .iter()
            .any(|item| item.label == "number"
                && item.kind == Some(CompletionItemKind::TYPE_PARAMETER)));
    }
}
