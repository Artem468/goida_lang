use super::*;
use crate::ast::prelude::{BinaryOperator, ExpressionKind, LiteralValue, StatementKind};
use crate::interpreter::prelude::SharedInterner;
use crate::parser::prelude::Parser as ProgramParser;
use crate::shared::SharedMut;
use std::path::PathBuf;
use string_interner::StringInterner;

#[test]
fn define_builtin_macro_registers_rules() {
    let mut expander = MacroExpander::default();
    let result: Result<(), ParseError> = (|| {
        crate::define_builtin_macro!(expander, "twice" => {
            ("$x:expr") => { "$x + $x" };
        });
        Ok(())
    })();

    assert!(result.is_ok(), "{result:?}");
    let definition = expander
        .definitions
        .get("twice")
        .expect("builtin macro should be registered");
    assert_eq!(definition.rules.len(), 1);
}

#[test]
fn format_macro_expands_to_expected_ast() {
    let interner = SharedMut::new(StringInterner::new());
    let actual = parse_for_ast(
        interner.clone(),
        r#"
имя = "Анна"
возраст = 21
результат = format!("Привет, {}, тебе {} лет", имя, возраст)
"#,
    );
    let expected = parse_for_ast(
        interner.clone(),
        r#"
имя = "Анна"
возраст = 21
результат = "Привет, " + имя + ", тебе " + возраст + " лет"
"#,
    );

    assert_eq!(
        normalize_statements(&actual, &interner),
        normalize_statements(&expected, &interner)
    );
}

#[test]
fn single_english_expr_macro_expands_to_expected_ast() {
    assert_same_ast(
        r#"
macro twice {
    ($x:expr) => { $x + $x };
}

value = twice!(input + 1)
"#,
        r#"
value = input + 1 + input + 1
"#,
    );
}

#[test]
fn multiple_english_macro_rules_pick_first_full_match_for_ast() {
    assert_same_ast(
        r#"
macro choose {
    () => { "empty" };
    ($value:expr) => { $value };
}

empty_value = choose!()
filled_value = choose!("filled")
"#,
        r#"
empty_value = "empty"
filled_value = "filled"
"#,
    );
}

#[test]
fn russian_macro_keyword_and_fragment_aliases_expand_to_expected_ast() {
    assert_same_ast(
        r#"
macro choose_ru {
    ($name:имя) => { $name };
    ($value:выр) => { $value };
}

source = 9
same = choose_ru!(source)
computed = choose_ru!(source + 1)
"#,
        r#"
source = 9
same = source
computed = source + 1
"#,
    );
}

#[test]
fn repetition_macro_with_zero_one_and_many_items_expands_to_expected_ast() {
    assert_same_ast(
        r#"
macro make_list {
    ($( $x:expr ),*) => { list($( $x ),*) };
}

empty = make_list!()
single = make_list!(1)
many = make_list!(1, 2, 3)
"#,
        r#"
empty = list()
single = list(1)
many = list(1, 2, 3)
"#,
    );
}

#[test]
fn one_or_more_repetition_macro_expands_to_expected_ast() {
    assert_same_ast(
        r#"
macro sum {
    ($first:expr $( + $rest:expr )+) => { $first $( + $rest )+ };
}

value = sum!(1 + 2 + 3)
"#,
        r#"
value = 1 + 2 + 3
"#,
    );
}

#[test]
fn statement_and_block_fragments_expand_to_expected_ast() {
    assert_same_ast(
        r#"
macro when {
    ($condition:expr, $statement:stmt) => { if ($condition) { $statement } };
}

macro guarded {
    ($condition:expr, $body:block) => { if ($condition) $body };
}

value = 0
when!(true, value = 1)
guarded!(false, { value = 2 })
"#,
        r#"
value = 0
if (true) {
    value = 1
}
if (false) {
    value = 2
}
"#,
    );
}

fn parse_for_ast(interner: SharedInterner, source: &str) -> crate::interpreter::prelude::Module {
    ProgramParser::new(
        interner,
        "macro_ast_test",
        PathBuf::from("macro_ast_test.goida"),
    )
    .parse(source)
    .expect("source should parse")
}

fn assert_same_ast(actual_source: &str, expected_source: &str) {
    let interner = SharedMut::new(StringInterner::new());
    let actual = parse_for_ast(interner.clone(), actual_source);
    let expected = parse_for_ast(interner.clone(), expected_source);

    assert_eq!(
        normalize_statements(&actual, &interner),
        normalize_statements(&expected, &interner)
    );
}

fn normalize_statements(
    module: &crate::interpreter::prelude::Module,
    interner: &SharedInterner,
) -> Vec<String> {
    module
        .body
        .iter()
        .map(|stmt| normalize_statement(module, interner, *stmt))
        .collect()
}

fn normalize_statement(
    module: &crate::interpreter::prelude::Module,
    interner: &SharedInterner,
    stmt_id: crate::ast::prelude::StmtId,
) -> String {
    let stmt = module
        .arena
        .get_statement(stmt_id)
        .expect("statement should exist");
    match &stmt.kind {
        StatementKind::Assign {
            name,
            is_const,
            type_hint,
            value,
        } => format!(
            "assign({},{is_const},{:?},{})",
            resolve(interner, *name),
            type_hint,
            normalize_expr(module, interner, *value)
        ),
        StatementKind::Expression(expr) => {
            format!("expr({})", normalize_expr(module, interner, *expr))
        }
        StatementKind::Block(items) => format!(
            "block({})",
            items
                .iter()
                .map(|stmt| normalize_statement(module, interner, *stmt))
                .collect::<Vec<_>>()
                .join(",")
        ),
        StatementKind::If {
            condition,
            then_body,
            else_body,
        } => format!(
            "if({},{},{})",
            normalize_expr(module, interner, *condition),
            normalize_statement(module, interner, *then_body),
            else_body
                .map(|stmt| normalize_statement(module, interner, stmt))
                .unwrap_or_else(|| "none".to_string())
        ),
        other => format!("{other:?}"),
    }
}

fn normalize_expr(
    module: &crate::interpreter::prelude::Module,
    interner: &SharedInterner,
    expr_id: crate::ast::prelude::ExprId,
) -> String {
    let expr = module
        .arena
        .get_expression(expr_id)
        .expect("expression should exist");
    match &expr.kind {
        ExpressionKind::Literal(LiteralValue::Text(symbol)) => {
            format!("text({:?})", resolve(interner, *symbol))
        }
        ExpressionKind::Literal(LiteralValue::Number(value)) => format!("number({value})"),
        ExpressionKind::Literal(LiteralValue::Float(value)) => format!("float({value})"),
        ExpressionKind::Literal(LiteralValue::Boolean(value)) => format!("bool({value})"),
        ExpressionKind::Literal(LiteralValue::Unit) => "unit".to_string(),
        ExpressionKind::Identifier(symbol) => format!("ident({})", resolve(interner, *symbol)),
        ExpressionKind::FunctionCall { function, args } => format!(
            "call({},{})",
            normalize_expr(module, interner, *function),
            normalize_args(module, interner, args)
        ),
        ExpressionKind::MethodCall {
            object,
            method,
            args,
        } => format!(
            "method({},{},{})",
            normalize_expr(module, interner, *object),
            resolve(interner, *method),
            normalize_args(module, interner, args)
        ),
        ExpressionKind::PropertyAccess { object, property } => format!(
            "prop({},{})",
            normalize_expr(module, interner, *object),
            resolve(interner, *property)
        ),
        ExpressionKind::Binary {
            op: BinaryOperator::Add,
            left,
            right,
        } => format!(
            "add({},{})",
            normalize_expr(module, interner, *left),
            normalize_expr(module, interner, *right)
        ),
        ExpressionKind::Binary { op, left, right } => format!(
            "binary({op:?},{},{})",
            normalize_expr(module, interner, *left),
            normalize_expr(module, interner, *right)
        ),
        other => format!("{other:?}"),
    }
}

fn normalize_args(
    module: &crate::interpreter::prelude::Module,
    interner: &SharedInterner,
    args: &[crate::ast::prelude::CallArg],
) -> String {
    args.iter()
        .map(|arg| {
            let name = arg
                .name
                .map(|name| resolve(interner, name))
                .unwrap_or_else(|| "_".to_string());
            format!("{name}:{}", normalize_expr(module, interner, arg.value))
        })
        .collect::<Vec<_>>()
        .join(",")
}

fn resolve(interner: &SharedInterner, symbol: string_interner::DefaultSymbol) -> String {
    interner
        .read(|i| i.resolve(symbol).map(str::to_string))
        .unwrap_or_else(|| format!("<{:?}>", symbol))
}
