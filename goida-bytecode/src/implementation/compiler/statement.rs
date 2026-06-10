use goida_syntax::prelude::TryHandler;
use crate::BytecodeHandler;

impl<'a> ChunkCompiler<'a> {
    fn statement(&mut self, id: StmtId) {
        let node = self.hir.arena.statement(id).expect("valid statement");
        let span = node.span;
        match &node.kind {
            HirStatementKind::Expression(expr) => {
                let result = self.expression(*expr);
                self.release(result);
            }
            HirStatementKind::Assign {
                name,
                binding,
                is_const,
                value,
                ..
            } => {
                let source = self.expression(*value);
                self.chunk.emit(
                    Instruction::StoreName {
                        name: *name,
                        binding: *binding,
                        is_const: *is_const,
                        source,
                    },
                    span,
                );
                self.release(source);
            }
            HirStatementKind::CompoundAssign { target, op, value } => {
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
                self.release(left);
                self.release(right);
                self.store_target(target, result, span);
                self.release(result);
            }
            HirStatementKind::IndexAssign {
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
                self.release(object);
                self.release(index);
                self.release(source);
            }
            HirStatementKind::If {
                condition,
                then_body,
                else_body,
            } => self.if_statement(*condition, *then_body, *else_body, span),
            HirStatementKind::While { condition, body } => {
                self.while_statement(*condition, *body, span)
            }
            HirStatementKind::For {
                variable,
                binding,
                init,
                condition,
                update,
                body,
            } => {
                if matches!(binding, Binding::LocalSlot(_)) {
                    let initial = self.expression(*init);
                    self.chunk.emit(
                        Instruction::StoreName {
                            name: *variable,
                            binding: *binding,
                            is_const: false,
                            source: initial,
                        },
                        span,
                    );
                    self.release(initial);
                    self.while_with_update(*condition, *body, *update, span);
                    return;
                }
                let mut nested = ChunkCompiler::new(self.module, self.hir);
                let initial = nested.expression(*init);
                nested.chunk.emit(
                    Instruction::StoreName {
                        name: *variable,
                        binding: *binding,
                        is_const: false,
                        source: initial,
                    },
                    span,
                );
                nested.release(initial);
                nested.while_with_update(*condition, *body, *update, span);
                self.chunk
                    .emit(Instruction::Scope(Arc::new(nested.finish(None))), span);
            }
            HirStatementKind::ForEach {
                variable,
                iterable,
                body,
                ..
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
                self.release(iterable);
            }
            HirStatementKind::Thread { body } => {
                let body = Arc::new(Compiler::statement_chunk(self.module, self.hir, *body));
                self.chunk.emit(Instruction::Thread(body), span);
            }
            HirStatementKind::Try { body, handlers } => {
                let body = Arc::new(Compiler::statement_chunk(self.module, self.hir, *body));
                let handlers = handlers
                    .iter()
                    .map(|handler| self.handler(handler))
                    .collect();
                self.chunk.emit(Instruction::Try { body, handlers }, span);
            }
            HirStatementKind::Raise {
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
                if let Some(message) = message {
                    self.release(message);
                }
            }
            HirStatementKind::Block(statements) => {
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
            HirStatementKind::Return(value) => {
                let value = value.map(|value| self.expression(value));
                self.chunk.emit(Instruction::Return(value), span);
            }
            HirStatementKind::FunctionDefinition(function) => {
                self.chunk
                    .emit(Instruction::DefineFunction(function.clone()), span);
            }
            HirStatementKind::NativeLibraryDefinition(definition) => {
                self.chunk
                    .emit(Instruction::LoadNativeLibrary(definition.clone()), span);
            }
            HirStatementKind::ClassDefinition(class) => {
                self.chunk
                    .emit(Instruction::DefineClass(class.clone()), span);
            }
            HirStatementKind::PropertyAssign {
                object,
                property,
                value,
            } => {
                let receiver_is_this = matches!(
                    self.hir.arena.expression(*object).map(|e| &e.kind),
                    Some(HirExpressionKind::This)
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
                self.release(object);
                self.release(source);
            }
            HirStatementKind::Import(_) | HirStatementKind::Empty => {}
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
        self.release(condition);
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
        self.release(condition);
        self.statement(body);
        if let Some(update) = update.into() {
            self.statement(update);
        }
        self.chunk.emit(Instruction::Jump(loop_start), span);
        let end = self.chunk.code.len();
        self.patch_jump_if_false(exit, end);
    }

    fn assign_target(&mut self, id: ExprId) -> AssignTarget {
        let node = self.hir.arena.expression(id).expect("valid target");
        match &node.kind {
            HirExpressionKind::Identifier { name, binding, .. } => AssignTarget::Name {
                name: *name,
                binding: *binding,
            },
            HirExpressionKind::PropertyAccess { object, property } => AssignTarget::Property {
                object: self.expression(*object),
                property: *property,
                receiver_is_this: matches!(
                    self.hir.arena.expression(*object).map(|e| &e.kind),
                    Some(HirExpressionKind::This)
                ),
            },
            HirExpressionKind::Index { object, index } => AssignTarget::Index {
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
                self.release(object);
                self.release(index);
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
            self.hir
                .arena
                .statement(*statement)
                .is_some_and(|node| match node.kind {
                    HirStatementKind::Assign { binding, .. } => {
                        !matches!(binding, Binding::LocalSlot(_))
                    }
                    HirStatementKind::FunctionDefinition(_)
                    | HirStatementKind::ClassDefinition(_)
                    | HirStatementKind::NativeLibraryDefinition(_) => true,
                    _ => false,
                })
        })
    }
}
