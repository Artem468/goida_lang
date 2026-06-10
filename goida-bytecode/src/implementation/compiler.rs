use super::{BytecodeModule, Chunk, Instruction, Register};
use crate::ast::prelude::{ExprId, FunctionDefinition, Span, StmtId};
use crate::hir::{Binding, HirExpressionKind, HirModule, HirStatementKind};
use std::collections::BTreeSet;
use std::sync::Arc;
use string_interner::DefaultSymbol as Symbol;

pub trait BytecodeSource {
    fn name(&self) -> Symbol;
}

#[derive(Clone, Debug)]
enum AssignTarget {
    Name {
        name: Symbol,
        binding: Binding,
    },
    Property {
        object: Register,
        property: Symbol,
        receiver_is_this: bool,
    },
    Index {
        object: Register,
        index: Register,
    },
}

struct ChunkCompiler<'a> {
    module: &'a dyn BytecodeSource,
    hir: &'a HirModule,
    chunk: Chunk,
    next_register: Register,
    free_registers: Vec<Register>,
}

impl<'a> ChunkCompiler<'a> {
    fn new(module: &'a dyn BytecodeSource, hir: &'a HirModule) -> Self {
        Self {
            module,
            hir,
            chunk: Chunk::default(),
            next_register: 0,
            free_registers: Vec::new(),
        }
    }

    fn register(&mut self) -> Register {
        if let Some(register) = self.free_registers.pop() {
            return register;
        }
        let register = self.next_register;
        self.next_register += 1;
        self.chunk.register_count = self.chunk.register_count.max(self.next_register);
        register
    }

    fn release(&mut self, register: Register) {
        debug_assert!(!self.free_registers.contains(&register));
        self.free_registers.push(register);
    }

    fn release_args(&mut self, args: &[super::RegisterArg]) {
        for arg in args {
            self.release(arg.register);
        }
    }

    fn finish(mut self, result: Option<Register>) -> Chunk {
        self.chunk.result = result;
        self.chunk.emit(Instruction::Halt, Span::default());
        self.chunk
    }
}

include!("compiler/expression.rs");
include!("compiler/statement.rs");

pub struct Compiler;

impl Compiler {
    pub fn compile(module: &dyn BytecodeSource, hir: &HirModule) -> BytecodeModule {
        let mut bytecode = BytecodeModule {
            module: Arc::new(Self::statements_chunk(module, hir, &hir.body)),
            ..BytecodeModule::default()
        };
        for id in Self::standalone_expression_ids(hir) {
            let mut compiler = ChunkCompiler::new(module, hir);
            let result = compiler.expression(id);
            bytecode
                .expressions
                .insert(id, Arc::new(compiler.finish(Some(result))));
        }
        for function in &hir.functions {
            bytecode.bodies.insert(
                function.body,
                Arc::new(Self::statement_chunk(module, hir, function.body)),
            );
        }
        for (_, node) in hir.arena.statements() {
            if let HirStatementKind::FunctionDefinition(function) = &node.kind {
                bytecode.bodies.insert(
                    function.body,
                    Arc::new(Self::statement_chunk(module, hir, function.body)),
                );
            }
        }
        for (_, node) in hir.arena.expressions() {
            if let HirExpressionKind::Lambda { body, .. } = node.kind {
                bytecode
                    .bodies
                    .insert(body, Arc::new(Self::statement_chunk(module, hir, body)));
            }
        }
        bytecode
    }

    fn standalone_expression_ids(hir: &HirModule) -> BTreeSet<ExprId> {
        let mut ids = BTreeSet::new();

        for function in &hir.functions {
            Self::collect_parameter_defaults(&function.params, &mut ids);
        }
        for (_, statement) in hir.arena.statements() {
            match &statement.kind {
                HirStatementKind::FunctionDefinition(function) => {
                    Self::collect_parameter_defaults(&function.params, &mut ids);
                }
                HirStatementKind::ClassDefinition(class) => {
                    for (_, _, field) in class.fields.values() {
                        if let crate::ast::program::FieldData::Expression(Some(id)) = field {
                            ids.insert(*id);
                        }
                    }
                    for (_, _, method) in class.methods.values() {
                        if let crate::ast::program::MethodType::User(function) = method {
                            Self::collect_parameter_defaults(&function.params, &mut ids);
                        }
                    }
                    if let Some(crate::ast::program::MethodType::User(function)) =
                        &class.constructor
                    {
                        Self::collect_parameter_defaults(&function.params, &mut ids);
                    }
                }
                _ => {}
            }
        }
        for (_, expression) in hir.arena.expressions() {
            if let HirExpressionKind::Lambda { params, .. } = &expression.kind {
                Self::collect_parameter_defaults(params, &mut ids);
            }
        }

        ids
    }

    fn collect_parameter_defaults(
        params: &[crate::ast::prelude::Parameter],
        ids: &mut BTreeSet<ExprId>,
    ) {
        ids.extend(params.iter().filter_map(|param| param.default_value));
    }

    fn statements_chunk(
        module: &dyn BytecodeSource,
        hir: &HirModule,
        statements: &[StmtId],
    ) -> Chunk {
        let mut compiler = ChunkCompiler::new(module, hir);
        for statement in statements {
            compiler.statement(*statement);
        }
        compiler.finish(None)
    }

    fn statement_chunk(module: &dyn BytecodeSource, hir: &HirModule, statement: StmtId) -> Chunk {
        let node = hir.arena.statement(statement).expect("valid statement");
        if let HirStatementKind::Block(statements) = &node.kind {
            Self::statements_chunk(module, hir, statements)
        } else {
            let mut compiler = ChunkCompiler::new(module, hir);
            compiler.statement(statement);
            compiler.finish(None)
        }
    }
}
