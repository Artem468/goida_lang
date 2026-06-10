use crate::RegisterArg;
use goida_hir::HirCallArg;
use goida_syntax::prelude::BinaryOperator;

impl<'a> ChunkCompiler<'a> {
    fn expression(&mut self, id: ExprId) -> Register {
        let node = self.hir.arena.expression(id).expect("valid expression");
        let span = node.span;
        match &node.kind {
            HirExpressionKind::Literal(value) => {
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
            HirExpressionKind::Identifier { name, binding, .. } => {
                let dst = self.register();
                self.chunk.emit(
                    Instruction::LoadName {
                        dst,
                        name: *name,
                        binding: *binding,
                    },
                    span,
                );
                dst
            }
            HirExpressionKind::Binary { op, left, right }
                if matches!(op, BinaryOperator::And | BinaryOperator::Or) =>
            {
                self.short_circuit(*op, *left, *right, span)
            }
            HirExpressionKind::Binary { op, left, right } => {
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
                self.release(left);
                self.release(right);
                dst
            }
            HirExpressionKind::Unary { op, operand } => {
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
                self.release(operand);
                dst
            }
            HirExpressionKind::FunctionCall { function, args } => {
                let args = self.args(args);
                let dst = self.register();
                match self.hir.arena.expression(*function).map(|e| &e.kind) {
                    Some(HirExpressionKind::Identifier { name, binding, .. })
                        if !matches!(binding, Binding::LocalSlot(_) | Binding::UpvalueSlot(_)) =>
                    {
                        self.release_args(&args);
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
                        self.release(callable);
                        self.release_args(&args);
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
            HirExpressionKind::Index { object, index } => {
                let object = self.expression(*object);
                let index = self.expression(*index);
                let dst = self.register();
                self.chunk
                    .emit(Instruction::ReadIndex { dst, object, index }, span);
                self.release(object);
                self.release(index);
                dst
            }
            HirExpressionKind::PropertyAccess { object, property } => {
                let receiver = self.hir.arena.expression(*object).map(|e| &e.kind);
                let receiver_is_this = matches!(receiver, Some(HirExpressionKind::This));
                let receiver_name = match receiver {
                    Some(HirExpressionKind::Identifier { name, .. }) => Some(*name),
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
                self.release(object);
                dst
            }
            HirExpressionKind::MethodCall {
                object,
                resolution,
                args,
            } => {
                let receiver_is_this = matches!(
                    self.hir.arena.expression(*object).map(|e| &e.kind),
                    Some(HirExpressionKind::This)
                );
                let object = self.expression(*object);
                let args = self.args(args);
                let dst = self.register();
                self.release(object);
                self.release_args(&args);
                self.chunk.emit(
                    Instruction::CallMethod {
                        dst,
                        object,
                        resolution: *resolution,
                        args,
                        receiver_is_this,
                    },
                    span,
                );
                dst
            }
            HirExpressionKind::ObjectCreation { class_name, args } => {
                let args = self.args(args);
                let dst = self.register();
                self.release_args(&args);
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
            HirExpressionKind::Lambda { params, body } => {
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
            HirExpressionKind::This => {
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
        self.release(left);
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
                self.release(right);
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
                self.release(right);
                let end = self.chunk.code.len();
                self.patch_jump(skip_right, end);
            }
            _ => unreachable!(),
        }
        dst
    }

    fn args(&mut self, args: &[HirCallArg]) -> Vec<RegisterArg> {
        args.iter()
            .map(|arg| RegisterArg {
                name: arg.name,
                register: self.expression(arg.value),
            })
            .collect()
    }
}
