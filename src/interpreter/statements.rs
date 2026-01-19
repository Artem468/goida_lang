use crate::ast::prelude::{ExpressionKind, Program, StatementKind, StmtId};
use crate::interpreter::structs::{Environment, Interpreter, RuntimeError, Value};
use crate::interpreter::traits::{ExpressionEvaluator, InterpreterUtils, StatementExecutor};

impl StatementExecutor for Interpreter {
    fn execute_statement(
        &mut self,
        stmt_id: StmtId,
        program: &Program,
    ) -> Result<(), RuntimeError> {
        let stmt = program.arena.get_statement(stmt_id).unwrap();
        match &stmt.kind {
            StatementKind::Expression(expr_id) => {
                self.evaluate_expression(*expr_id, program)?;
                Ok(())
            }

            StatementKind::Assign {
                name,
                type_hint: _,
                value,
            } => {
                let name_str = program.arena.resolve_symbol(*name).unwrap().to_string();
                let val = self.evaluate_expression(*value, program)?;
                self.environment.set(&name_str, val)?;
                Ok(())
            }

            StatementKind::IndexAssign {
                object: _,
                index: _,
                value: _,
            } => Err(RuntimeError::InvalidOperation(
                "Индексный доступ отключён".to_string(),
            )),

            StatementKind::If {
                condition,
                then_body,
                else_body,
            } => {
                let condition_value = self.evaluate_expression(*condition, program)?;
                if condition_value.is_truthy() {
                    self.execute_statement(*then_body, program)
                } else if let Some(else_stmt_id) = else_body {
                    self.execute_statement(*else_stmt_id, program)
                } else {
                    Ok(())
                }
            }

            StatementKind::While { condition, body } => {
                while self.evaluate_expression(*condition, program)?.is_truthy() {
                    self.execute_statement(*body, program)?;
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
                let variable_str = program.arena.resolve_symbol(*variable).unwrap().to_string();

                let parent_env = self.environment.clone();
                self.environment = Environment::with_parent(parent_env);

                let init_val = self.evaluate_expression(*init, program)?;
                self.environment.define(variable_str.clone(), init_val);

                loop {
                    let cond_val = self.evaluate_expression(*condition, program)?;
                    if !cond_val.is_truthy() {
                        break;
                    }

                    self.execute_statement(*body, program)?;

                    let update_val = self.evaluate_expression(*update, program)?;
                    self.environment.define(variable_str.clone(), update_val);
                }

                if let Some(parent) = self.environment.parent.take() {
                    self.environment = *parent;
                }
                Ok(())
            }

            StatementKind::Block(statements) => {
                let parent_env = self.environment.clone();
                self.environment = Environment::with_parent(parent_env);

                for &stmt_id in statements {
                    self.execute_statement(stmt_id, program)?;
                }

                if let Some(parent) = self.environment.parent.take() {
                    self.environment = *parent;
                }
                Ok(())
            }

            StatementKind::Return(expr) => {
                let value = if let Some(e) = expr {
                    self.evaluate_expression(*e, program)?
                } else {
                    Value::Empty
                };
                Err(RuntimeError::Return(value))
            }

            StatementKind::Print(expr_id) => {
                let value = self.evaluate_expression(*expr_id, program)?;
                println!("{}", value.to_string());
                Ok(())
            }

            StatementKind::Input(expr_id) => {
                let value = self.evaluate_expression(*expr_id, program)?;
                self.input_function(value)?;
                Ok(())
            }

            StatementKind::ClassDefinition(_) => Ok(()),
            StatementKind::FunctionDefinition(_) => Ok(()),

            StatementKind::PropertyAssign {
                object,
                property,
                value,
            } => {
                let property_name = program.arena.resolve_symbol(*property).unwrap().to_string();
                let obj_expr = program.arena.get_expression(*object).unwrap();
                let is_external = !matches!(obj_expr.kind, ExpressionKind::This);

                let obj_value = self.evaluate_expression(*object, program)?;
                let new_value = self.evaluate_expression(*value, program)?;

                if let Value::Object(instance_ref) = obj_value {
                    {
                        let instance = instance_ref.borrow();
                        if !instance.is_field_accessible(&property_name, is_external) {
                            return Err(RuntimeError::InvalidOperation(format!(
                                "Поле '{}' недоступно для записи",
                                property_name
                            )));
                        }
                    }

                    {
                        let mut instance_mut = instance_ref.borrow_mut();
                        instance_mut.set_field(property_name, new_value);
                    }

                    Ok(())
                } else {
                    Err(RuntimeError::TypeMismatch(format!(
                        "Попытка присвоения свойства не-объектному типу: {:?}",
                        obj_value
                    )))
                }
            }
        }
    }
}
