use super::*;
use crate::parser::prelude::Parser;
use crate::session::Session;
use std::path::PathBuf;

#[test]
fn runtime_does_not_read_ast_arena() {
    let mut session = Session::new();
    let mut module = Parser::new(
        session.interner(),
        "vm_without_ast",
        PathBuf::from("vm_without_ast.goida"),
    )
    .parse(
        r#"
function sum(limit) {
    total = 0
    for (i = 0, i < limit, i += 1) {
        total += i
    }
    return total
}
result = sum(5)
"#,
    )
    .expect("program should compile");
    let module_id = module.name;
    module.arena.expressions.clear();
    module.arena.statements.clear();

    session
        .execute(module)
        .expect("VM must execute without AST nodes");

    let result_name = session.runtime().intern_string("result");
    assert_eq!(
        session.runtime().modules[&module_id]
            .globals
            .get(&result_name),
        Some(&Value::Number(10))
    );
}

#[test]
fn register_bytecode_short_circuits_boolean_expressions() {
    let mut session = Session::new();
    let mut module = Parser::new(
        session.interner(),
        "register_short_circuit",
        PathBuf::from("register_short_circuit.goida"),
    )
    .parse(
        r#"
function mark() {
    return 1 / 0
}
left = true or mark()
right = false and mark()
"#,
    )
    .expect("program should compile");
    let module_id = module.name;
    assert!(module.bytecode.module.register_count > 0);
    assert!(module
        .bytecode
        .module
        .code
        .iter()
        .any(|instruction| matches!(instruction, Instruction::JumpIfFalse { .. })));
    module.arena.expressions.clear();
    module.arena.statements.clear();

    session.execute(module).expect("register VM should execute");

    let left = session.runtime().intern_string("left");
    let right = session.runtime().intern_string("right");
    assert_eq!(
        session.runtime().modules[&module_id].globals.get(&left),
        Some(&Value::Boolean(true))
    );
    assert_eq!(
        session.runtime().modules[&module_id].globals.get(&right),
        Some(&Value::Boolean(false))
    );
}

#[test]
fn local_for_loop_uses_flat_vm_slots_without_runtime_scope() {
    let mut session = Session::new();
    let module = Parser::new(
        session.interner(),
        "flat_slots",
        PathBuf::from("flat_slots.goida"),
    )
    .parse(
        r#"
function sum(limit) {
    total = 0
    for (i = 0, i < limit, i += 1) {
        part = i * 2
        total += part
    }
    return total
}
result = sum(5)
"#,
    )
    .expect("program should compile");
    let module_id = module.name;
    let function = module.functions.values().next().expect("sum function");
    let chunk = module
        .bytecode
        .bodies
        .get(&function.body)
        .expect("compiled function body");

    assert!(!chunk
        .code
        .iter()
        .any(|instruction| matches!(instruction, Instruction::Scope(_))));
    assert!(chunk.code.iter().any(|instruction| matches!(
        instruction,
        Instruction::LoadName {
            binding: Binding::LocalSlot(_),
            ..
        }
    )));
    assert!(chunk.code.iter().any(|instruction| matches!(
        instruction,
        Instruction::StoreName {
            binding: Binding::LocalSlot(_),
            ..
        }
    )));

    session
        .execute(module)
        .expect("slot-based loop should execute");
    let result = session.runtime().intern_string("result");
    assert_eq!(
        session.runtime().modules[&module_id].globals.get(&result),
        Some(&Value::Number(20))
    );
}

#[test]
fn known_globals_use_dense_module_slots() {
    let mut session = Session::new();
    let module = Parser::new(
        session.interner(),
        "global_slots",
        PathBuf::from("global_slots.goida"),
    )
    .parse(
        r#"
answer = 40
result = answer + 2
"#,
    )
    .expect("program should compile");
    let module_id = module.name;

    assert!(module
        .bytecode
        .module
        .code
        .iter()
        .any(|instruction| matches!(
            instruction,
            Instruction::LoadName {
                binding: Binding::GlobalSlot(_),
                ..
            }
        )));

    session
        .execute(module)
        .expect("global slots should execute");
    let result = session.runtime().intern_string("result");
    assert_eq!(
        session.runtime().modules[&module_id].globals.get(&result),
        Some(&Value::Number(42))
    );
}
