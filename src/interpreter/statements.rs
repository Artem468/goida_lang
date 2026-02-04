use crate::ast::prelude::{ErrorData, ExpressionKind, Span, StatementKind, StmtId};
use crate::interpreter::prelude::{Environment, Interpreter, RuntimeError, Value};
use crate::traits::prelude::{CoreOperations, ExpressionEvaluator, StatementExecutor};
use string_interner::DefaultSymbol;

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
                let val = self.evaluate_expression(value, current_module_id)?;

                if let Err(_) = self.environment.set(name, val.clone(), stmt_kind.span) {
                    self.environment.define(name, val);
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
                let parent_env = self.environment.clone();
                self.environment = Environment::with_parent(parent_env);

                let init_val = self.evaluate_expression(init, current_module_id)?;
                self.environment.define(variable.clone(), init_val);

                loop {
                    if !self
                        .evaluate_expression(condition, current_module_id)?
                        .is_truthy()
                    {
                        break;
                    }
                    self.execute_statement(body, current_module_id)?;

                    let update_val = self.evaluate_expression(update, current_module_id)?;
                    self.environment.define(variable.clone(), update_val);
                }

                if let Some(parent) = self.environment.parent.take() {
                    self.environment = *parent;
                }
                Ok(())
            }

            StatementKind::Block(statements) => {
                let parent_env = self.environment.clone();
                self.environment = Environment::with_parent(parent_env);

                for s_id in statements {
                    self.execute_statement(s_id, current_module_id)?;
                }

                if let Some(parent) = self.environment.parent.take() {
                    self.environment = *parent;
                }
                Ok(())
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
                let is_inside_method = self.environment.get(&this_sym).is_some();
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

                        // Устанавливаем значение
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

            StatementKind::ClassDefinition(_) | StatementKind::FunctionDefinition(_) => Ok(()),
            _ => Ok(()),
        }
    }
}
