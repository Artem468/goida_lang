use goida_syntax::prelude::TryHandler;
use crate::BytecodeHandler;

impl<'a> ChunkCompiler<'a> {
    fn statement(&mut self, id: StmtId) {
        let node = self
            .module
            .arena()
            .get_statement(id)
            .expect("valid statement");
        let span = node.span;
        match &node.kind {
            StatementKind::Expression(expr) => {
                self.expression(*expr);
            }
            StatementKind::Assign {
                name,
                is_const,
                value,
                ..
            } => {
                let source = self.expression(*value);
                self.chunk.emit(
                    Instruction::StoreName {
                        name: *name,
                        binding: self
                            .hir
                            .stores
                            .get(&id)
                            .copied()
                            .unwrap_or(Binding::Dynamic(*name)),
                        is_const: *is_const,
                        source,
                    },
                    span,
                );
            }
            StatementKind::CompoundAssign { target, op, value } => {
                let target = self.assign_target(*target);
                let left = self.read_target(&target, span);
                let right = self.expression(*value);
                let result = self.register();
                self.chunk.emit(
                    Instruction::Binary {
                        dst: result,
                        op: *op,
                        left,
                        right,
                    },
                    span,
                );
                self.store_target(target, result, span);
            }
            StatementKind::IndexAssign {
                object,
                index,
                value,
            } => {
                let object = self.expression(*object);
                let index = self.expression(*index);
                let source = self.expression(*value);
                self.chunk.emit(
                    Instruction::StoreIndex {
                        object,
                        index,
                        source,
                    },
                    span,
                );
            }
            StatementKind::If {
                condition,
                then_body,
                else_body,
            } => self.if_statement(*condition, *then_body, *else_body, span),
            StatementKind::While { condition, body } => {
                self.while_statement(*condition, *body, span)
            }
            StatementKind::For {
                variable,
                init,
                condition,
                update,
                body,
            } => {
                let binding = self
                    .hir
                    .stores
                    .get(&id)
                    .copied()
                    .unwrap_or(Binding::Dynamic(*variable));
                if matches!(binding, Binding::LocalSlot(_)) {
                    let initial = self.expression(*init);
                    self.chunk.emit(
                        Instruction::StoreName {
                            name: *variable,
                            binding,
                            is_const: false,
                            source: initial,
                        },
                        span,
                    );
                    self.while_with_update(*condition, *body, *update, span);
                    return;
                }
                let mut nested = ChunkCompiler::new(self.module, self.hir);
                let initial = nested.expression(*init);
                nested.chunk.emit(
                    Instruction::StoreName {
                        name: *variable,
                        binding,
                        is_const: false,
                        source: initial,
                    },
                    span,
                );
                nested.while_with_update(*condition, *body, *update, span);
                self.chunk
                    .emit(Instruction::Scope(Arc::new(nested.finish(None))), span);
            }
            StatementKind::ForEach {
                variable,
                iterable,
                body,
            } => {
                let iterable = self.expression(*iterable);
                let body = Arc::new(Compiler::statement_chunk(self.module, self.hir, *body));
                self.chunk.emit(
                    Instruction::ForEach {
                        variable: *variable,
                        iterable,
                        body,
                    },
                    span,
                );
            }
            StatementKind::Thread { body } => {
                let body = Arc::new(Compiler::statement_chunk(self.module, self.hir, *body));
                self.chunk.emit(Instruction::Thread(body), span);
            }
            StatementKind::Try { body, handlers } => {
                let body = Arc::new(Compiler::statement_chunk(self.module, self.hir, *body));
                let handlers = handlers
                    .iter()
                    .map(|handler| self.handler(handler))
                    .collect();
                self.chunk.emit(Instruction::Try { body, handlers }, span);
            }
            StatementKind::Raise {
                error_type,
                message,
            } => {
                let message = message.map(|message| self.expression(message));
                self.chunk.emit(
                    Instruction::Raise {
                        error_type: *error_type,
                        message,
                    },
                    span,
                );
            }
            StatementKind::Block(statements) => {
                if self.block_needs_scope(statements) {
                    let body = Arc::new(Compiler::statements_chunk(
                        self.module,
                        self.hir,
                        statements,
                    ));
                    self.chunk.emit(Instruction::Scope(body), span);
                } else {
                    for statement in statements {
                        self.statement(*statement);
                    }
                }
            }
            StatementKind::Return(value) => {
                let value = value.map(|value| self.expression(value));
                self.chunk.emit(Instruction::Return(value), span);
            }
            StatementKind::FunctionDefinition(function) => {
                self.chunk
                    .emit(Instruction::DefineFunction(function.clone()), span);
            }
            StatementKind::NativeLibraryDefinition(definition) => {
                self.chunk
                    .emit(Instruction::LoadNativeLibrary(definition.clone()), span);
            }
            StatementKind::ClassDefinition(class) => {
                self.chunk
                    .emit(Instruction::DefineClass(class.clone()), span);
            }
            StatementKind::PropertyAssign {
                object,
                property,
                value,
            } => {
                let receiver_is_this = matches!(
                    self.module.arena().get_expression(*object).map(|e| &e.kind),
                    Some(ExpressionKind::This)
                );
                let object = self.expression(*object);
                let source = self.expression(*value);
                self.chunk.emit(
                    Instruction::StoreProperty {
                        object,
                        property: *property,
                        source,
                        receiver_is_this,
                    },
                    span,
                );
            }
            StatementKind::Import(_) | StatementKind::Empty => {}
        }
    }

    fn if_statement(
        &mut self,
        condition: ExprId,
        then_body: StmtId,
        else_body: Option<StmtId>,
        span: Span,
    ) {
        let condition = self.expression(condition);
        let false_jump = self.chunk.emit(
            Instruction::JumpIfFalse {
                condition,
                target: usize::MAX,
            },
            span,
        );
        self.statement(then_body);
        if let Some(else_body) = else_body {
            let end_jump = self.chunk.emit(Instruction::Jump(usize::MAX), span);
            let else_start = self.chunk.code.len();
            self.patch_jump_if_false(false_jump, else_start);
            self.statement(else_body);
            let end = self.chunk.code.len();
            self.patch_jump(end_jump, end);
        } else {
            let end = self.chunk.code.len();
            self.patch_jump_if_false(false_jump, end);
        }
    }

    fn while_statement(&mut self, condition: ExprId, body: StmtId, span: Span) {
        self.while_with_update(condition, body, None, span);
    }

    fn while_with_update(
        &mut self,
        condition: ExprId,
        body: StmtId,
        update: impl Into<Option<StmtId>>,
        span: Span,
    ) {
        let loop_start = self.chunk.code.len();
        let condition = self.expression(condition);
        let exit = self.chunk.emit(
            Instruction::JumpIfFalse {
                condition,
                target: usize::MAX,
            },
            span,
        );
        self.statement(body);
        if let Some(update) = update.into() {
            self.statement(update);
        }
        self.chunk.emit(Instruction::Jump(loop_start), span);
        let end = self.chunk.code.len();
        self.patch_jump_if_false(exit, end);
    }

    fn assign_target(&mut self, id: ExprId) -> AssignTarget {
        let node = self.module.arena().get_expression(id).expect("valid target");
        match &node.kind {
            ExpressionKind::Identifier(name) => AssignTarget::Name {
                name: *name,
                binding: self
                    .hir
                    .names
                    .get(&id)
                    .copied()
                    .unwrap_or(Binding::Dynamic(*name)),
            },
            ExpressionKind::PropertyAccess { object, property } => AssignTarget::Property {
                object: self.expression(*object),
                property: *property,
                receiver_is_this: matches!(
                    self.module.arena().get_expression(*object).map(|e| &e.kind),
                    Some(ExpressionKind::This)
                ),
            },
            ExpressionKind::Index { object, index } => AssignTarget::Index {
                object: self.expression(*object),
                index: self.expression(*index),
            },
            _ => panic!("invalid assignment target"),
        }
    }

    fn read_target(&mut self, target: &AssignTarget, span: Span) -> Register {
        let dst = self.register();
        match target {
            AssignTarget::Name { name, binding } => {
                self.chunk.emit(
                    Instruction::LoadName {
                        dst,
                        name: *name,
                        binding: *binding,
                    },
                    span,
                );
            }
            AssignTarget::Property {
                object,
                property,
                receiver_is_this,
            } => {
                self.chunk.emit(
                    Instruction::ReadProperty {
                        dst,
                        object: *object,
                        property: *property,
                        receiver_is_this: *receiver_is_this,
                        receiver_name: None,
                    },
                    span,
                );
            }
            AssignTarget::Index { object, index } => {
                self.chunk.emit(
                    Instruction::ReadIndex {
                        dst,
                        object: *object,
                        index: *index,
                    },
                    span,
                );
            }
        }
        dst
    }

    fn store_target(&mut self, target: AssignTarget, source: Register, span: Span) {
        match target {
            AssignTarget::Name { name, binding } => {
                self.chunk.emit(
                    Instruction::StoreName {
                        name,
                        binding,
                        is_const: false,
                        source,
                    },
                    span,
                );
            }
            AssignTarget::Property {
                object,
                property,
                receiver_is_this,
            } => {
                self.chunk.emit(
                    Instruction::StoreProperty {
                        object,
                        property,
                        source,
                        receiver_is_this,
                    },
                    span,
                );
            }
            AssignTarget::Index { object, index } => {
                self.chunk.emit(
                    Instruction::StoreIndex {
                        object,
                        index,
                        source,
                    },
                    span,
                );
            }
        }
    }

    fn handler(&self, handler: &TryHandler) -> BytecodeHandler {
        BytecodeHandler {
            error_type: handler.error_type,
            error_text: handler.error_text,
            body: Arc::new(Compiler::statement_chunk(
                self.module,
                self.hir,
                handler.body,
            )),
        }
    }

    fn patch_jump(&mut self, address: usize, target: usize) {
        let Instruction::Jump(current) = &mut self.chunk.code[address] else {
            panic!("expected jump");
        };
        *current = target;
    }

    fn patch_jump_if_false(&mut self, address: usize, target: usize) {
        let Instruction::JumpIfFalse {
            target: current, ..
        } = &mut self.chunk.code[address]
        else {
            panic!("expected conditional jump");
        };
        *current = target;
    }

    fn block_needs_scope(&self, statements: &[StmtId]) -> bool {
        statements.iter().any(|statement| {
            self.module
                .arena()
                .get_statement(*statement)
                .is_some_and(|node| match node.kind {
                    StatementKind::Assign { .. } => {
                        !matches!(self.hir.stores.get(statement), Some(Binding::LocalSlot(_)))
                    }
                    StatementKind::FunctionDefinition(_)
                    | StatementKind::ClassDefinition(_)
                    | StatementKind::NativeLibraryDefinition(_) => true,
                    _ => false,
                })
        })
    }
}
