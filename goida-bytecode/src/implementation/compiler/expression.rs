impl<'a> ChunkCompiler<'a> {
    fn expression(&mut self, id: ExprId) -> Register {
        let node = self
            .module
            .arena()
            .get_expression(id)
            .expect("valid expression");
        let span = node.span;
        match &node.kind {
            ExpressionKind::Literal(value) => {
                let dst = self.register();
                self.chunk.emit(
                    Instruction::LoadLiteral {
                        dst,
                        value: value.clone(),
                    },
                    span,
                );
                dst
            }
            ExpressionKind::Identifier(name) => {
                let dst = self.register();
                self.chunk.emit(
                    Instruction::LoadName {
                        dst,
                        name: *name,
                        binding: self
                            .hir
                            .names
                            .get(&id)
                            .copied()
                            .unwrap_or(Binding::Dynamic(*name)),
                    },
                    span,
                );
                dst
            }
            ExpressionKind::Binary { op, left, right }
                if matches!(op, BinaryOperator::And | BinaryOperator::Or) =>
            {
                self.short_circuit(*op, *left, *right, span)
            }
            ExpressionKind::Binary { op, left, right } => {
                let left = self.expression(*left);
                let right = self.expression(*right);
                let dst = self.register();
                self.chunk.emit(
                    Instruction::Binary {
                        dst,
                        op: *op,
                        left,
                        right,
                    },
                    span,
                );
                dst
            }
            ExpressionKind::Unary { op, operand } => {
                let operand = self.expression(*operand);
                let dst = self.register();
                self.chunk.emit(
                    Instruction::Unary {
                        dst,
                        op: *op,
                        operand,
                    },
                    span,
                );
                dst
            }
            ExpressionKind::FunctionCall { function, args } => {
                let args = self.args(args);
                let dst = self.register();
                match self.module.arena().get_expression(*function).map(|e| &e.kind) {
                    Some(ExpressionKind::Identifier(name))
                        if !matches!(
                            self.hir.names.get(function),
                            Some(Binding::LocalSlot(_) | Binding::UpvalueSlot(_))
                        ) =>
                    {
                        self.chunk.emit(
                            Instruction::CallDirect {
                                dst,
                                name: *name,
                                args,
                            },
                            span,
                        );
                    }
                    _ => {
                        let callable = self.expression(*function);
                        self.chunk.emit(
                            Instruction::Call {
                                dst,
                                callable,
                                args,
                            },
                            span,
                        );
                    }
                }
                dst
            }
            ExpressionKind::Index { object, index } => {
                let object = self.expression(*object);
                let index = self.expression(*index);
                let dst = self.register();
                self.chunk
                    .emit(Instruction::ReadIndex { dst, object, index }, span);
                dst
            }
            ExpressionKind::PropertyAccess { object, property } => {
                let receiver = self.module.arena().get_expression(*object).map(|e| &e.kind);
                let receiver_is_this = matches!(receiver, Some(ExpressionKind::This));
                let receiver_name = match receiver {
                    Some(ExpressionKind::Identifier(name)) => Some(*name),
                    _ => None,
                };
                let object = self.expression(*object);
                let dst = self.register();
                self.chunk.emit(
                    Instruction::ReadProperty {
                        dst,
                        object,
                        property: *property,
                        receiver_is_this,
                        receiver_name,
                    },
                    span,
                );
                dst
            }
            ExpressionKind::MethodCall {
                object,
                method,
                args,
            } => {
                let receiver_is_this = matches!(
                    self.module.arena().get_expression(*object).map(|e| &e.kind),
                    Some(ExpressionKind::This)
                );
                let object = self.expression(*object);
                let args = self.args(args);
                let dst = self.register();
                self.chunk.emit(
                    Instruction::CallMethod {
                        dst,
                        object,
                        resolution: self
                            .hir
                            .methods
                            .get(&id)
                            .copied()
                            .unwrap_or(MethodResolution::Dynamic(*method)),
                        args,
                        receiver_is_this,
                    },
                    span,
                );
                dst
            }
            ExpressionKind::ObjectCreation { class_name, args } => {
                let args = self.args(args);
                let dst = self.register();
                self.chunk.emit(
                    Instruction::NewObject {
                        dst,
                        class_name: *class_name,
                        args,
                    },
                    span,
                );
                dst
            }
            ExpressionKind::Lambda { params, body } => {
                let dst = self.register();
                self.chunk.emit(
                    Instruction::MakeLambda {
                        dst,
                        function: FunctionDefinition {
                            name: self.module.name(),
                            params: params.clone(),
                            return_type: None,
                            body: *body,
                            span,
                            module: Some(self.module.name()),
                        },
                    },
                    span,
                );
                dst
            }
            ExpressionKind::This => {
                let dst = self.register();
                self.chunk.emit(Instruction::InvalidThis { dst }, span);
                dst
            }
        }
    }

    fn short_circuit(
        &mut self,
        op: BinaryOperator,
        left_id: ExprId,
        right_id: ExprId,
        span: Span,
    ) -> Register {
        let left = self.expression(left_id);
        let dst = self.register();
        self.chunk
            .emit(Instruction::ToBoolean { dst, source: left }, span);
        match op {
            BinaryOperator::And => {
                let jump = self.chunk.emit(
                    Instruction::JumpIfFalse {
                        condition: dst,
                        target: usize::MAX,
                    },
                    span,
                );
                let right = self.expression(right_id);
                self.chunk
                    .emit(Instruction::ToBoolean { dst, source: right }, span);
                let end = self.chunk.code.len();
                self.patch_jump_if_false(jump, end);
            }
            BinaryOperator::Or => {
                let branch = self.chunk.emit(
                    Instruction::JumpIfFalse {
                        condition: dst,
                        target: usize::MAX,
                    },
                    span,
                );
                let skip_right = self.chunk.emit(Instruction::Jump(usize::MAX), span);
                let right_start = self.chunk.code.len();
                self.patch_jump_if_false(branch, right_start);
                let right = self.expression(right_id);
                self.chunk
                    .emit(Instruction::ToBoolean { dst, source: right }, span);
                let end = self.chunk.code.len();
                self.patch_jump(skip_right, end);
            }
            _ => unreachable!(),
        }
        dst
    }

    fn args(&mut self, args: &[CallArg]) -> Vec<RegisterArg> {
        args.iter()
            .map(|arg| RegisterArg {
                name: arg.name,
                register: self.expression(arg.value),
            })
            .collect()
    }
}
