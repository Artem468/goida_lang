use std::sync::Arc;
use crate::ast::prelude::{ErrorData, ExpressionKind, Span, StatementKind, StmtId};
use crate::interpreter::prelude::{Environment, Interpreter, RuntimeError, Value};
use crate::traits::prelude::{CoreOperations, ExpressionEvaluator, StatementExecutor};
use string_interner::DefaultSymbol;
use crate::shared::SharedMut;

impl StatementExecutor for Interpreter {
    fn execute_statement(
        &mut self,
        stmt_id: StmtId,
        current_module_id: DefaultSymbol,
    ) -> Result<(), RuntimeError> {
        let stmt_kind = {
            let module = self.modules.get(&current_module_id).ok_or_else(|| {
                RuntimeError::InvalidOperation(ErrorData::new(
                    Span::default(),
                    "Модуль не найден".into(),
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

                self.environment = target_env;
                self.environment.write(|env| {
                    if env.set(name, val.clone(), stmt_kind.span).is_err() {
                        env.define(name, val);
                    }
                });

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
                let previous_env = self.environment.clone();

                let mut local_env_inner = Environment::new();
                local_env_inner.parent = Some(previous_env.clone());

                let init_val = self.evaluate_expression(init, current_module_id)?;
                local_env_inner.define(variable.clone(), init_val);

                self.environment = SharedMut::new(local_env_inner);

                let result = (|| -> Result<(), RuntimeError> {
                    loop {
                        let cond_val = self.evaluate_expression(condition, current_module_id)?;
                        if !cond_val.is_truthy() {
                            break;
                        }
                        self.execute_statement(body, current_module_id)?;

                        let update_val = self.evaluate_expression(update, current_module_id)?;
                        self.environment.write(|env| env.define(variable.clone(), update_val));
                    }
                    Ok(())
                })();
                self.environment = previous_env;

                result
            }

            StatementKind::Block(statements) => {
                let previous_env = self.environment.clone();

                let mut local_env_inner = Environment::new();
                local_env_inner.parent = Some(previous_env.clone());

                self.environment = SharedMut::new(local_env_inner);

                let result = (|| {
                    for s_id in statements {
                        self.execute_statement(s_id, current_module_id)?;
                    }
                    Ok(())
                })();

                self.environment = previous_env;

                result
            }

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
                        RuntimeError::InvalidOperation(ErrorData::new(
                            stmt_kind.span,
                            "Модуль не найден".into(),
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
                    Value::List(list) => {
                        list.write(|vec| {
                            let i = idx_val.resolve_index(vec.len(), stmt_kind.span)?;
                            vec[i] = val_to_set;
                            Ok(())
                        })
                    }
                    Value::Dict(dict) => dict.write(|d| {
                        d.insert(idx_val.to_string(), val_to_set);
                        Ok(())
                    }),
                    _ => {
                        Err(RuntimeError::TypeError(ErrorData::new(
                            stmt_kind.span,
                            "Нельзя присвоить по индексу для этого типа".into(),
                        )))
                    }
                }
            }

            StatementKind::FunctionDefinition(def) => {
                let name = def.name;
                let value = Value::Function(Arc::from(def.clone()));
                self.environment.write(|env| env.define(name, value));
                Ok(())
            }

            StatementKind::ClassDefinition(cls) => {
                let def = SharedMut::new(cls.clone());
                let value = Value::Class(def.clone());
                self.environment.write(|env| env.define(cls.name, value));
                Ok(())
            }
            _ => Ok(()),
        }
    }
}
