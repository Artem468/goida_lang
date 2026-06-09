impl<'a> Vm<'a> {
    fn execute_chunk(&mut self, chunk: &Chunk) -> Result<Vec<Value>, RuntimeError> {
        let mut registers = vec![Value::Empty; chunk.register_count as usize];
        let mut ip = 0usize;
        while ip < chunk.code.len() {
            let span = chunk.spans.get(ip).copied().unwrap_or_default();
            let instruction = &chunk.code[ip];
            ip += 1;
            match instruction {
                Instruction::LoadLiteral { dst, value } => {
                    let value = match value {
                        LiteralValue::Number(value) => Value::Number(*value),
                        LiteralValue::Float(value) => Value::Float(*value),
                        LiteralValue::Text(value) => {
                            Value::Text(self.interpreter.resolve_symbol(*value).unwrap_or_default())
                        }
                        LiteralValue::Boolean(value) => Value::Boolean(*value),
                        LiteralValue::Unit => Value::Empty,
                    };
                    Self::set(&mut registers, *dst, value);
                }
                Instruction::LoadName { dst, name, binding } => {
                    let value = match binding {
                        Binding::LocalSlot(slot) => {
                            let slot = *slot as usize;
                            if let Some(Some(value)) = self.locals.get(slot) {
                                value.clone()
                            } else {
                                let value = self.load_identifier(*name, span)?;
                                self.set_local(slot, value.clone());
                                value
                            }
                        }
                        _ => self.load_identifier(*name, span)?,
                    };
                    Self::set(&mut registers, *dst, value);
                }
                Instruction::Unary { dst, op, operand } => {
                    let value = Self::get(&registers, *operand);
                    let value = match (op, value) {
                        (UnaryOperator::Negative, Value::Number(value)) => Value::Number(-value),
                        (UnaryOperator::Not, value) => Value::Boolean(!value.is_truthy()),
                        _ => {
                            return bail_runtime!(
                                TypeMismatch,
                                span,
                                "Unary minus can only be applied to numbers"
                            )
                        }
                    };
                    Self::set(&mut registers, *dst, value);
                }
                Instruction::Binary {
                    dst,
                    op,
                    left,
                    right,
                } => {
                    let value = self.binary(
                        *op,
                        Self::get(&registers, *left),
                        Self::get(&registers, *right),
                        span,
                    )?;
                    Self::set(&mut registers, *dst, value);
                }
                Instruction::ToBoolean { dst, source } => {
                    let value = Value::Boolean(Self::get(&registers, *source).is_truthy());
                    Self::set(&mut registers, *dst, value);
                }
                Instruction::CallDirect { dst, name, args } => {
                    let args = Self::args(&registers, args);
                    let value =
                        self.interpreter
                            .call_function_by_name(*name, args, self.module, span)?;
                    Self::set(&mut registers, *dst, value);
                }
                Instruction::Call {
                    dst,
                    callable,
                    args,
                } => {
                    let args = Self::args(&registers, args);
                    let value = match Self::get(&registers, *callable) {
                        Value::Function(function) => {
                            self.interpreter
                                .call_function(function, args, self.module, span)?
                        }
                        Value::Builtin(function) => function(self.interpreter, args, span)?,
                        _ => return bail_runtime!(InvalidOperation, span, "Value is not callable"),
                    };
                    Self::set(&mut registers, *dst, value);
                }
                Instruction::ReadIndex { dst, object, index } => {
                    let value = self.read_index(
                        Self::get(&registers, *object),
                        Self::get(&registers, *index),
                        span,
                    )?;
                    Self::set(&mut registers, *dst, value);
                }
                Instruction::ReadProperty {
                    dst,
                    object,
                    property,
                    receiver_is_this,
                    receiver_name,
                } => {
                    let value = self.read_property(
                        Ok(Self::get(&registers, *object)),
                        *property,
                        *receiver_is_this,
                        *receiver_name,
                        span,
                    )?;
                    Self::set(&mut registers, *dst, value);
                }
                Instruction::CallMethod {
                    dst,
                    object,
                    resolution,
                    args,
                    receiver_is_this,
                } => {
                    let method = match resolution {
                        MethodResolution::Static(method) | MethodResolution::Dynamic(method) => {
                            *method
                        }
                    };
                    let args = Self::args(&registers, args);
                    let value = self.call_method(
                        Self::get(&registers, *object),
                        method,
                        args,
                        *receiver_is_this,
                        span,
                    )?;
                    Self::set(&mut registers, *dst, value);
                }
                Instruction::NewObject {
                    dst,
                    class_name,
                    args,
                } => {
                    let args = Self::args(&registers, args);
                    let (class, module) = self.interpreter.resolve_class_for_creation(
                        *class_name,
                        self.module,
                        span,
                    )?;
                    let value = self
                        .interpreter
                        .instantiate_class(class, module, args, span)?;
                    Self::set(&mut registers, *dst, value);
                }
                Instruction::MakeLambda { dst, function } => {
                    let mut function = function.clone();
                    function.name = self.interpreter.intern_string("<lambda>");
                    function.module = Some(self.module);
                    Self::set(&mut registers, *dst, Value::Function(Arc::new(function)));
                }
                Instruction::InvalidThis { .. } => {
                    return bail_runtime!(
                        InvalidOperation,
                        span,
                        "'this' must be passed as an explicit method parameter"
                    );
                }
                Instruction::StoreName {
                    name,
                    binding,
                    is_const,
                    source,
                } => {
                    let value = Self::get(&registers, *source);
                    if let Binding::LocalSlot(slot) = binding {
                        if self.local_constants.contains(slot) {
                            return bail_runtime!(
                                InvalidOperation,
                                span,
                                "Cannot assign to a constant"
                            );
                        }
                        self.set_local(*slot as usize, value);
                        if *is_const {
                            self.local_constants.insert(*slot);
                        }
                    } else if *is_const {
                        self.interpreter
                            .define_constant(*name, value, self.module, span)?;
                    } else {
                        self.interpreter
                            .assign_identifier(*name, value, self.module, span)?;
                    }
                }
                Instruction::StoreIndex {
                    object,
                    index,
                    source,
                } => self.assign_index(
                    Self::get(&registers, *object),
                    Self::get(&registers, *index),
                    Self::get(&registers, *source),
                    span,
                )?,
                Instruction::StoreProperty {
                    object,
                    property,
                    source,
                    receiver_is_this,
                } => self.assign_property(
                    Self::get(&registers, *object),
                    *property,
                    Self::get(&registers, *source),
                    *receiver_is_this,
                    span,
                )?,
                Instruction::Jump(target) => ip = *target,
                Instruction::JumpIfFalse { condition, target } => {
                    if !Self::get(&registers, *condition).is_truthy() {
                        ip = *target;
                    }
                }
                Instruction::Scope(body) => {
                    let module = self.module;
                    self.interpreter.scoped_child_environment(
                        |_| {},
                        |interpreter| Vm::new(interpreter, module).run(body),
                    )?;
                }
                Instruction::ForEach {
                    variable,
                    iterable,
                    body,
                } => {
                    let values = self
                        .interpreter
                        .iterable_values(Self::get(&registers, *iterable), span)?;
                    let module = self.module;
                    self.interpreter.scoped_child_environment(
                        |_| {},
                        |interpreter| {
                            for value in values {
                                interpreter
                                    .environment
                                    .write(|environment| environment.define(*variable, value));
                                Vm::new(interpreter, module).run(body)?;
                            }
                            Ok(())
                        },
                    )?;
                }
                Instruction::Thread(body) => {
                    let mut interpreter = self.interpreter.fork_for_thread();
                    let module = self.module;
                    let body = body.clone();
                    let handle = thread::spawn(move || {
                        let result = match Vm::new(&mut interpreter, module).run(&body) {
                            Err(RuntimeError::Return(..)) => Ok(()),
                            result => result,
                        };
                        result?;
                        interpreter.join_background_threads(module, span)
                    });
                    self.interpreter
                        .background_threads
                        .push(RuntimeThread::new(handle));
                }
                Instruction::Try { body, handlers } => match self.run_chunk(body) {
                    Ok(()) => {}
                    Err(error @ RuntimeError::Return(..)) => return Err(error),
                    Err(error) => {
                        let error_class = error.error_class_name();
                        let error_message = error.error_message();
                        let mut handled = false;
                        for handler in handlers {
                            if handler.error_type.is_none()
                                || self.interpreter.runtime_error_matches(
                                    &error_class,
                                    handler.error_type.unwrap(),
                                    self.module,
                                )
                            {
                                let module = self.module;
                                self.interpreter.scoped_child_environment(
                                    |environment| {
                                        if let Some(name) = handler.error_text {
                                            environment
                                                .define(name, Value::Text(error_message.clone()));
                                        }
                                    },
                                    |interpreter| Vm::new(interpreter, module).run(&handler.body),
                                )?;
                                handled = true;
                                break;
                            }
                        }
                        if !handled {
                            return Err(error);
                        }
                    }
                },
                Instruction::Raise {
                    error_type,
                    message,
                } => {
                    let class_name = self
                        .interpreter
                        .resolve_symbol(*error_type)
                        .unwrap_or_default();
                    let message = message
                        .map(|message| Self::get(&registers, message).to_string())
                        .unwrap_or_else(|| class_name.clone());
                    return Err(RuntimeError::Raised(
                        ErrorData::new(span, message),
                        class_name,
                    ));
                }
                Instruction::Return(value) => {
                    let value = value
                        .map(|value| Self::get(&registers, value))
                        .unwrap_or(Value::Empty);
                    return bail_runtime!(                         Return,                         span,                         "{}",                         self.interpreter.format_value(&value) => value                     );
                }
                Instruction::DefineFunction(function) => {
                    self.interpreter.environment.write(|environment| {
                        environment
                            .define(function.name, Value::Function(Arc::new(function.clone())));
                    });
                }
                Instruction::LoadNativeLibrary(definition) => self
                    .interpreter
                    .load_native_library_definition(definition.clone(), self.module)?,
                Instruction::DefineClass(class) => {
                    let definition = self
                        .interpreter
                        .modules
                        .get(&self.module)
                        .and_then(|module| module.classes.get(&class.name))
                        .cloned()
                        .unwrap_or_else(|| {
                            SharedMut::new(RuntimeClassDefinition::from_syntax(class))
                        });
                    self.interpreter.environment.write(|environment| {
                        environment.define(class.name, Value::Class(definition))
                    });
                }
                Instruction::Halt => break,
            }
        }
        Ok(registers)
    }
}
