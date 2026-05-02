use crate::ast::prelude::{ErrorData, ExpressionKind, Span, StatementKind, StmtId};
use crate::interpreter::prelude::{Interpreter, RuntimeError, Value};
use crate::shared::SharedMut;
use crate::traits::prelude::{CoreOperations, ExpressionEvaluator, StatementExecutor};
use std::sync::Arc;
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
                RuntimeError::InvalidOperation(ErrorData::new(
                    Span::default(),
                    format!("Модуль {module_name} не найден"),
                ))
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
                self.environment.write(|env| {
                    if env.set(name, val.clone(), stmt_kind.span).is_err() {
                        env.define(name, val.clone());
                    }
                });

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
                Err(RuntimeError::Return(
                    ErrorData::new(stmt_kind.span, value.to_string()),
                    value,
                ))
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
                        RuntimeError::InvalidOperation(ErrorData::new(
                            stmt_kind.span,
                            format!("Модуль {module_name} не найден"),
                        ))
                    })?;
                    module.arena.get_expression(object).unwrap().clone()
                };
                let is_external = !matches!(obj_expr.kind, ExpressionKind::This);

                let this_sym = self.intern_string("this");
                let is_inside_method = self.environment.read(|env| env.get(&this_sym).is_some());
                let is_external = is_external && !is_inside_method;

                let obj_value = self.evaluate_expression(object, current_module_id)?;
                let value_result = self.evaluate_expression(value, current_module_id)?;

                if let Value::Object(instance_ref) = obj_value {
                    instance_ref.write(|instance| {
                        if !instance.is_field_accessible(&property, is_external) {
                            return Err(RuntimeError::InvalidOperation(ErrorData::new(
                                obj_expr.span,
                                format!("Поле '{}' недоступно", property_name),
                            )));
                        }

                        instance.set_field_value(property, value_result);
                        Ok(())
                    })
                } else {
                    Err(RuntimeError::TypeMismatch(ErrorData::new(
                        obj_expr.span,
                        "Ожидался объект".into(),
                    )))
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
                    _ => Err(RuntimeError::TypeError(ErrorData::new(
                        stmt_kind.span,
                        "Нельзя присвоить по индексу для этого типа".into(),
                    ))),
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
