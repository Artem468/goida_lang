use crate::ast::prelude::DataType;
use crate::bytecode::Instruction;
use crate::hir::{Binding, HirExpressionKind};
use crate::interpreter::prelude::SharedInterner;
use crate::parser::prelude::Parser;
use crate::shared::SharedMut;
use std::path::PathBuf;
use string_interner::StringInterner;

fn parse_error(source: &str) -> crate::parser::prelude::ParseError {
    Parser::new(
        goida_model::new_interner(),
        "type_check",
        PathBuf::from("type_check.goida"),
    )
    .parse(source)
    .expect_err("source should fail type checking")
}

fn assert_type_error(source: &str, message_fragment: &str) {
    let crate::parser::prelude::ParseError::TypeError(data) = parse_error(source) else {
        panic!("expected type error");
    };
    assert!(
        data.message.contains(message_fragment),
        "unexpected error: {}",
        data.message
    );
}

#[test]
fn macro_expansion_preview_contains_expanded_source_without_macro_definition() {
    let interner: SharedInterner = SharedMut::new(StringInterner::new());
    let parser = Parser::new(interner, "preview_test", PathBuf::from("preview.goida"));
    let preview = parser
        .macro_expansion_preview(
            r#"
macro twice {
    ($x:expr) => { $x + $x };
}

value = twice!(2)
"#,
        )
        .expect("macro preview should expand");

    assert!(preview.contains("value = 2 + 2"));
    assert!(!preview.contains("macro twice"));
}

#[test]
fn parser_lowers_names_and_callable_bodies_to_bytecode() {
    let interner: SharedInterner = SharedMut::new(StringInterner::new());
    let parser = Parser::new(interner, "lower_test", PathBuf::from("lower.goida"));
    let module = parser
        .parse(
            r#"
function identity(value) {
    result = value
    return result
}
answer = identity(42)
"#,
        )
        .expect("module should lower");

    assert!(module
        .hir
        .arena
        .expressions()
        .any(|(_, expression)| matches!(
            expression.kind,
            HirExpressionKind::Identifier {
                binding: Binding::LocalSlot(_) | Binding::UpvalueSlot(_),
                ..
            }
        )));
    assert_eq!(
        module.hir.arena.expressions().count(),
        module.arena.expressions.len()
    );
    assert_eq!(
        module.hir.arena.statements().count(),
        module.arena.statements.len()
    );
    assert!(module
        .bytecode
        .module
        .code
        .iter()
        .any(|instruction| matches!(instruction, Instruction::StoreName { .. })));
    assert_eq!(module.bytecode.bodies.len(), 1);
}

#[test]
fn type_checker_rejects_invalid_assignments() {
    assert_type_error(
        "value: number = \"text\"\n",
        "Несовместимый тип присваивания",
    );
    assert_type_error(
        "value: number = 1\nvalue = \"text\"\n",
        "Несовместимый тип присваивания",
    );
}

#[test]
fn type_checker_rejects_invalid_function_arguments_defaults_and_returns() {
    assert_type_error(
        "function accept(value: number) {}\naccept(\"text\")\n",
        "Несовместимый тип аргумента функции",
    );
    assert_type_error(
        "function value() -> number { return \"text\" }\n",
        "Несовместимый тип возвращаемого значения",
    );
    assert_type_error(
        "function value(input: number = \"text\") {}\n",
        "Несовместимый тип значения параметра по умолчанию",
    );
}

#[test]
fn type_checker_rejects_invalid_native_function_arguments_before_execution() {
    assert_type_error(
        r#"
library "native" {
    function add(a: number, b: number) -> number {}
}
add("text", 1)
"#,
        "Несовместимый тип аргумента функции",
    );
}

#[test]
fn type_checker_records_inferred_expression_types_and_keeps_dynamic_values_dynamic() {
    let module = Parser::new(
        goida_model::new_interner(),
        "type_check",
        PathBuf::from("type_check.goida"),
    )
    .parse(
        r#"
function identity(value) {
    return value
}
dynamic = 1
dynamic = "text"
number_value: number = 42
identity(dynamic)
"#,
    )
    .expect("dynamic values should remain valid");

    assert!(!module.hir.inferred_types.is_empty());
    assert!(module
        .hir
        .arena
        .expressions()
        .any(|(_, expression)| expression.inferred_type != DataType::Any));
}

#[test]
fn compiler_only_emits_standalone_expression_chunks_when_needed() {
    let interner = goida_model::new_interner();
    let module = Parser::new(
        interner,
        "standalone_expressions",
        PathBuf::from("standalone_expressions.goida"),
    )
    .parse(
        r#"
function answer(value = 40 + 2) {
    return value
}

class Box {
    public value: number = 6 * 7
}

result = answer() + 1
"#,
    )
    .expect("module should compile");

    assert_eq!(module.bytecode.expressions.len(), 2);
    assert!(module.bytecode.expressions.len() < module.arena.expressions.len());
}

#[test]
fn syntax_only_parse_does_not_build_hir_or_bytecode() {
    let interner = goida_model::new_interner();
    let module = Parser::new(interner, "syntax_only", PathBuf::from("syntax_only.goida"))
        .parse_syntax("value = 1\n")
        .expect("source should parse");

    assert_eq!(module.hir.arena.expressions().count(), 0);
    assert_eq!(module.hir.arena.statements().count(), 0);
    assert!(module.bytecode.module.code.is_empty());
    assert!(module.bytecode.bodies.is_empty());
    assert!(module.bytecode.expressions.is_empty());
}

#[test]
fn syntax_parse_retains_import_node_but_emits_no_import_bytecode() {
    let root = std::env::temp_dir().join(format!("goida-import-ast-{}", std::process::id()));
    std::fs::create_dir_all(&root).expect("test directory should exist");
    std::fs::write(root.join("module.goida"), "value = 1\n").expect("module should be written");

    let interner = goida_model::new_interner();
    let module = Parser::new(interner, "imports", root.join("main.goida"))
        .parse("import \"module.goida\" as module\n")
        .expect("import should parse");

    assert!(matches!(
        module
            .arena
            .get_statement(module.body[0])
            .map(|node| &node.kind),
        Some(crate::ast::prelude::StatementKind::Import(_))
    ));
    assert_eq!(module.bytecode.module.code.len(), 1);
    assert!(matches!(
        module.bytecode.module.code[0],
        crate::bytecode::Instruction::Halt
    ));

    std::fs::remove_dir_all(root).expect("test directory should be removed");
}

#[test]
fn parser_reports_cyclic_module_imports() {
    let root = std::env::temp_dir().join(format!("goida-import-cycle-{}", std::process::id()));
    std::fs::create_dir_all(&root).expect("test directory should exist");
    std::fs::write(root.join("a.goida"), "import \"b.goida\" as b\n")
        .expect("module a should be written");
    std::fs::write(root.join("b.goida"), "import \"a.goida\" as a\n")
        .expect("module b should be written");

    let interner = goida_model::new_interner();
    let error = Parser::new(interner, "cycle", root.join("a.goida"))
        .parse("import \"b.goida\" as b\n")
        .expect_err("cyclic import should fail");

    let crate::parser::prelude::ParseError::ImportError(data) = error else {
        panic!("cycle should produce an import error");
    };
    assert!(data.message.contains("Cyclic module import"));

    std::fs::remove_dir_all(root).expect("test directory should be removed");
}

#[test]
fn ast_formatter_preserves_comments_and_ignores_comment_markers_in_strings() {
    let interner = goida_model::new_interner();
    let parser = Parser::new(interner, "format", PathBuf::from("format.goida"));
    let formatted = parser
        .format_source_ast("// before\nvalue=\"// text\" // trailing\n")
        .expect("source should format");

    assert_eq!(formatted, "// before\nvalue = \"// text\"\n// trailing\n");
}

#[test]
fn format_language_detection_prefers_the_dominant_keyword_language() {
    use crate::parser::prelude::FormatLanguage;

    assert_eq!(
        FormatLanguage::detect("функция main() { вернуть истина }\n"),
        FormatLanguage::Russian
    );
    assert_eq!(
        FormatLanguage::detect("function main() { return true }\n"),
        FormatLanguage::English
    );
    assert_eq!(
        FormatLanguage::detect("value = 1\n"),
        FormatLanguage::English
    );
}
