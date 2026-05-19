use crate::ast::prelude::{ErrorData, ExpressionKind, Span, StatementKind, StmtId};
use crate::interpreter::prelude::{Interpreter, RuntimeError, Value};
use crate::shared::SharedMut;
use crate::traits::prelude::{CoreOperations, ExpressionEvaluator, StatementExecutor};
use crate::{bail_runtime, runtime_error};
use std::sync::Arc;
use std::thread;
use string_interner::DefaultSymbol;

impl StatementExecutor for Interpreter {
    fn execute_statement(
        &mut self,
        stmt_id: StmtId,
        current_module_id: DefaultSymbol,
    ) -> Result<(), RuntimeError> {
        let stmt_kind = {
            let module = self.modules.get(&current_module_id).ok_or_else(|| {
                let module_name = self.resolve_symbol(current_module_id).unwrap();
                runtime_error!(
                    InvalidOperation,
                    Span::default(),
                    "Модуль {module_name} не найден"
                )
            })?;
            module.arena.get_statement(stmt_id).unwrap().clone()
        };

        match stmt_kind.kind {
            StatementKind::Expression(expr_id) => {
                self.evaluate_expression(expr_id, current_module_id)?;
                Ok(())
            }

            StatementKind::Assign { name, value, .. } => {
                let target_env = self.environment.clone();
                let val = self.evaluate_expression(value, current_module_id)?;

                if self.try_assign_native_identifier(
                    name,
                    val.clone(),
                    current_module_id,
                    stmt_kind.span,
                )? {
                    return Ok(());
                }

                self.environment = target_env;
                // Debug: log whether we set or define the variable and which environment is root
                let _var_name = self.resolve_symbol(name).unwrap_or_default();
                let _is_root_env = self.environment.read(|env| env.parent.is_none());
                // Try to set the variable in the nearest environment where it exists;
                // if not found, define it in the current environment. This preserves
                // expected behavior for block-scoped code that assigns to outer
                // variables while still creating new locals when the variable is new.
                // We need nuanced assignment semantics:
                // - Search for the variable in the chain up to the nearest function environment
                //   (inclusive). If found, update that environment.
                // - If not found but a containing function environment exists, define the
                //   variable in that function environment (so nested functions don't
                //   accidentally update outer function variables with the same name).
                // - If no function environment exists in the chain, fall back to the
                //   original set-then-define semantics.
                let mut search_env = self.environment.clone();
                let mut found_env: Option<SharedMut<crate::interpreter::structs::Environment>> =
                    None;
                let mut function_env: Option<SharedMut<crate::interpreter::structs::Environment>> =
                    None;

                loop {
                    if search_env.read(|env| env.variables.contains_key(&name)) {
                        found_env = Some(search_env.clone());
                        break;
                    }
                    if search_env.read(|env| env.is_function) {
                        function_env = Some(search_env.clone());
                        break;
                    }
                    let parent_opt = search_env.read(|env| env.parent.clone());
                    if let Some(parent) = parent_opt {
                        search_env = parent;
                    } else {
                        break;
                    }
                }

                if let Some(target_env) = found_env {
                    target_env.write(|env| {
                        env.variables.insert(name, val.clone());
                    });
                } else if let Some(fn_env) = function_env {
                    fn_env.write(|env| {
                        env.define(name, val.clone());
                    });
                } else {
                    let set_result = self
                        .environment
                        .write(|env| env.set(name, val.clone(), stmt_kind.span));

                    if set_result.is_err() {
                        self.environment.write(|env| env.define(name, val.clone()));
                    }
                }

                if self.environment.read(|env| env.parent.is_none()) {
                    if let Some(module) = self.modules.get_mut(&current_module_id) {
                        module.globals.insert(name, val.clone());
                    }
                }

                Ok(())
            }

            StatementKind::If {
                condition,
                then_body,
                else_body,
            } => {
                let condition_value = self.evaluate_expression(condition, current_module_id)?;
                if condition_value.is_truthy() {
                    self.execute_statement(then_body, current_module_id)
                } else if let Some(else_stmt_id) = else_body {
                    self.execute_statement(else_stmt_id, current_module_id)
                } else {
                    Ok(())
                }
            }

            StatementKind::While { condition, body } => {
                while self
                    .evaluate_expression(condition, current_module_id)?
                    .is_truthy()
                {
                    self.execute_statement(body, current_module_id)?;
                }
                Ok(())
            }

            StatementKind::For {
                variable,
                init,
                condition,
                update,
                body,
            } => {
                let init_val = self.evaluate_expression(init, current_module_id)?;

                self.scoped_child_environment(
                    |local_env| local_env.define(variable, init_val),
                    |interpreter| {
                        loop {
                            let cond_val =
                                interpreter.evaluate_expression(condition, current_module_id)?;
                            if !cond_val.is_truthy() {
                                break;
                            }
                            interpreter.execute_statement(body, current_module_id)?;

                            let update_val =
                                interpreter.evaluate_expression(update, current_module_id)?;
                            interpreter
                                .environment
                                .write(|env| env.define(variable, update_val));
                        }
                        Ok(())
                    },
                )
            }

            StatementKind::Thread { body } => {
                let mut thread_interpreter = self.fork_for_thread();
                let span = stmt_kind.span;
                let handle = thread::spawn(move || {
                    let result = match thread_interpreter.execute_thread_body(body, current_module_id)
                    {
                        Err(RuntimeError::Return(..)) => Ok(()),
                        result => result,
                    };

                    result?;
                    thread_interpreter.join_background_threads(current_module_id, span)
                });
                self.background_threads
                    .push(crate::interpreter::structs::RuntimeThread::new(handle));
                Ok(())
            }

            StatementKind::Try { body, handlers } => {
                match self.execute_statement(body, current_module_id) {
                    Ok(()) => Ok(()),
                    Err(err @ RuntimeError::Return(..)) => Err(err),
                    Err(err) => {
                        let error_class = err.error_class_name();
                        let error_message = err.error_message();
                        for handler in handlers {
                            if handler.error_type.is_none()
                                || self.runtime_error_matches(
                                    &error_class,
                                    handler.error_type.unwrap(),
                                    current_module_id,
                                )
                            {
                                return self.scoped_child_environment(
                                    |local_env| {
                                        if let Some(error_text) = handler.error_text {
                                            local_env.define(
                                                error_text,
                                                Value::Text(error_message.clone()),
                                            );
                                        }
                                    },
                                    |interpreter| {
                                        interpreter
                                            .execute_statement(handler.body, current_module_id)
                                    },
                                );
                            }
                        }
                        Err(err)
                    }
                }
            }

            StatementKind::Raise {
                error_type,
                message,
            } => {
                let class_name = self.resolve_symbol(error_type).unwrap_or_default();
                let message = if let Some(message_expr) = message {
                    self.evaluate_expression(message_expr, current_module_id)?
                        .to_string()
                } else {
                    class_name.clone()
                };

                Err(RuntimeError::Raised(
                    ErrorData::new(stmt_kind.span, message),
                    class_name,
                ))
            }

            StatementKind::Block(statements) => self.scoped_child_environment(
                |_| {},
                |interpreter| {
                    for s_id in statements {
                        interpreter.execute_statement(s_id, current_module_id)?;
                    }
                    Ok(())
                },
            ),

            StatementKind::Return(expr) => {
                let value = if let Some(e) = expr {
                    self.evaluate_expression(e, current_module_id)?
                } else {
                    Value::Empty
                };
                bail_runtime!(Return, stmt_kind.span, "{}", value.to_string() => value)
            }

            StatementKind::PropertyAssign {
                object,
                property,
                value,
            } => {
                let property_name = self.resolve_symbol(property).unwrap();
                let obj_expr = {
                    let module = self.modules.get(&current_module_id).ok_or_else(|| {
                        let module_name = self.resolve_symbol(current_module_id).unwrap();
                        runtime_error!(
                            InvalidOperation,
                            stmt_kind.span,
                            "Модуль {module_name} не найден"
                        )
                    })?;
                    module.arena.get_expression(object).unwrap().clone()
                };
                let is_external = !matches!(obj_expr.kind, ExpressionKind::This);

                let is_inside_method = self.method_depth > 0;
                let is_external = is_external && !is_inside_method;

                let obj_value = self.evaluate_expression(object, current_module_id)?;
                let value_result = self.evaluate_expression(value, current_module_id)?;

                if let Value::Object(instance_ref) = obj_value {
                    instance_ref.write(|instance| {
                        if !instance.is_field_accessible(&property, is_external) {
                            return bail_runtime!(
                                InvalidOperation,
                                obj_expr.span,
                                "Поле '{}' недоступно",
                                property_name
                            );
                        }
                        instance.set_field_value(property, value_result);
                        Ok(())
                    })
                } else {
                    bail_runtime!(TypeMismatch, obj_expr.span, "Ожидался объект")
                }
            }

            StatementKind::IndexAssign {
                object,
                index,
                value,
            } => {
                let target_obj = self.evaluate_expression(object, current_module_id)?;

                let idx_val = self.evaluate_expression(index, current_module_id)?;
                let val_to_set = self.evaluate_expression(value, current_module_id)?;

                match target_obj {
                    Value::List(list) => list.write(|vec| {
                        let i = idx_val.resolve_index(vec.len(), stmt_kind.span)?;
                        vec[i] = val_to_set;
                        Ok(())
                    }),
                    Value::Dict(dict) => dict.write(|d| {
                        d.insert(idx_val.to_string(), val_to_set);
                        Ok(())
                    }),
                    _ => bail_runtime!(
                        TypeError,
                        stmt_kind.span,
                        "Нельзя присвоить по индексу для этого типа"
                    ),
                }
            }

            StatementKind::FunctionDefinition(def) => {
                let name = def.name;
                let value = Value::Function(Arc::from(def.clone()));
                self.environment.write(|env| env.define(name, value));
                Ok(())
            }

            StatementKind::NativeLibraryDefinition(definition) => {
                self.load_native_library_definition(definition, current_module_id)
            }

            StatementKind::ClassDefinition(cls) => {
                let def = self
                    .modules
                    .get(&current_module_id)
                    .and_then(|module| module.classes.get(&cls.name))
                    .cloned()
                    .unwrap_or_else(|| SharedMut::new(cls.clone()));
                let value = Value::Class(def.clone());
                self.environment.write(|env| env.define(cls.name, value));
                Ok(())
            }
            _ => Ok(()),
        }
    }
}

impl Interpreter {
    fn execute_thread_body(
        &mut self,
        body: StmtId,
        current_module_id: DefaultSymbol,
    ) -> Result<(), RuntimeError> {
        let stmt_kind = {
            let module = self.modules.get(&current_module_id).ok_or_else(|| {
                let module_name = self.resolve_symbol(current_module_id).unwrap();
                runtime_error!(
                    InvalidOperation,
                    Span::default(),
                    "РњРѕРґСѓР»СЊ {module_name} РЅРµ РЅР°Р№РґРµРЅ"
                )
            })?;
            module.arena.get_statement(body).unwrap().clone()
        };

        if let StatementKind::Block(statements) = stmt_kind.kind {
            for stmt_id in statements {
                self.execute_statement(stmt_id, current_module_id)?;
            }
            Ok(())
        } else {
            self.execute_statement(body, current_module_id)
        }
    }

    pub(crate) fn join_thread_handle(
        &self,
        thread_value: &crate::interpreter::structs::RuntimeThread,
        span: Span,
    ) -> Result<Value, RuntimeError> {
        let handle = thread_value
            .handle
            .lock()
            .map_err(|_| runtime_error!(InvalidOperation, span, "Блокировка потока повреждена"))?
            .take();

        if let Some(handle) = handle {
            match handle.join() {
                Ok(Ok(())) => Ok(Value::Empty),
                Ok(Err(err)) => Err(err),
                Err(_) => bail_runtime!(Panic, span, "Поток завершился паникой"),
            }
        } else {
            Ok(Value::Empty)
        }
    }

    pub(crate) fn join_background_threads(
        &mut self,
        _current_module_id: DefaultSymbol,
        span: Span,
    ) -> Result<(), RuntimeError> {
        let threads = std::mem::take(&mut self.background_threads);
        for thread_value in threads {
            self.join_thread_handle(&thread_value, span)?;
        }
        Ok(())
    }
}
