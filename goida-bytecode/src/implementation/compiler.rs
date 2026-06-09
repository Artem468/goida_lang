use super::{BytecodeHandler, BytecodeModule, Chunk, Instruction, Register, RegisterArg};
use crate::ast::prelude::{
    AstArena, BinaryOperator, CallArg, ExprId, ExpressionKind, FunctionDefinition, Span,
    StatementKind, StmtId, TryHandler,
};
use crate::hir::{Binding, HirModule, MethodResolution};
use std::sync::Arc;
use string_interner::DefaultSymbol as Symbol;

pub trait BytecodeSource {
    fn name(&self) -> Symbol;
    fn arena(&self) -> &AstArena;
    fn body(&self) -> &[StmtId];
    fn functions_to_compile(&self) -> Vec<Arc<FunctionDefinition>>;
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
}

impl<'a> ChunkCompiler<'a> {
    fn new(module: &'a dyn BytecodeSource, hir: &'a HirModule) -> Self {
        Self {
            module,
            hir,
            chunk: Chunk::default(),
            next_register: 0,
        }
    }

    fn register(&mut self) -> Register {
        let register = self.next_register;
        self.next_register += 1;
        self.chunk.register_count = self.chunk.register_count.max(self.next_register);
        register
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
            module: Arc::new(Self::statements_chunk(module, hir, module.body())),
            ..BytecodeModule::default()
        };
        for id in 0..module.arena().expressions.len() as ExprId {
            let mut compiler = ChunkCompiler::new(module, hir);
            let result = compiler.expression(id);
            bytecode
                .expressions
                .insert(id, Arc::new(compiler.finish(Some(result))));
        }
        for function in module.functions_to_compile() {
            bytecode.bodies.insert(
                function.body,
                Arc::new(Self::statement_chunk(module, hir, function.body)),
            );
        }
        for node in &module.arena().statements {
            if let StatementKind::FunctionDefinition(function) = &node.kind {
                bytecode.bodies.insert(
                    function.body,
                    Arc::new(Self::statement_chunk(module, hir, function.body)),
                );
            }
        }
        for node in &module.arena().expressions {
            if let ExpressionKind::Lambda { body, .. } = node.kind {
                bytecode
                    .bodies
                    .insert(body, Arc::new(Self::statement_chunk(module, hir, body)));
            }
        }
        bytecode
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
        let node = module
            .arena()
            .get_statement(statement)
            .expect("valid statement");
        if let StatementKind::Block(statements) = &node.kind {
            Self::statements_chunk(module, hir, statements)
        } else {
            let mut compiler = ChunkCompiler::new(module, hir);
            compiler.statement(statement);
            compiler.finish(None)
        }
    }
}
