use crate::ast::prelude::{BinaryOperator, ExprId, ExpressionKind, LiteralValue, Program, UnaryOperator};
use crate::interpreter::structs::{Interpreter, RuntimeError, Value};
use crate::interpreter::traits::{ExpressionEvaluator, InterpreterClasses, InterpreterFunctions, InterpreterUtils, ValueOperations};

impl ExpressionEvaluator for Interpreter {
    fn evaluate_expression(
        &mut self,
        expr_id: ExprId,
        program: &Program,
    ) -> Result<Value, RuntimeError> {
        let expr = program.arena.get_expression(expr_id).unwrap();
        match &expr.kind {
            ExpressionKind::Literal(literal) => match literal {
                LiteralValue::Number(n) => Ok(Value::Number(*n)),
                LiteralValue::Float(f) => Ok(Value::Float(*f)),
                LiteralValue::Text(symbol) => {
                    let text = program.arena.resolve_symbol(*symbol).unwrap().to_string();
                    Ok(Value::Text(text))
                }
                LiteralValue::Boolean(b) => Ok(Value::Boolean(*b)),
                LiteralValue::Unit => Ok(Value::Empty),
            },

            ExpressionKind::Identifier(symbol) => {
                let name = program.arena.resolve_symbol(*symbol).unwrap().to_string();
                if let Some(dot_pos) = name.find('.') {
                    let module_name = &name[..dot_pos];
                    let var_name = &name[dot_pos + 1..];

                    if let Some(module_env) = self.modules.get(module_name) {
                        module_env.environment.get(var_name).ok_or_else(|| {
                            RuntimeError::UndefinedVariable(format!("{}.{}", module_name, var_name))
                        })
                    } else {
                        Err(RuntimeError::InvalidOperation(format!(
                            "Модуль '{}' не найден",
                            module_name
                        )))
                    }
                } else {
                    self.environment
                        .get(&name)
                        .ok_or_else(|| RuntimeError::UndefinedVariable(name))
                }
            }

            ExpressionKind::Binary { op, left, right } => {
                let left_val = self.evaluate_expression(*left, program)?;
                let right_val = self.evaluate_expression(*right, program)?;

                match op {
                    BinaryOperator::Add => self.add_values(left_val, right_val),
                    BinaryOperator::Sub => self.subtract_values(left_val, right_val),
                    BinaryOperator::Mul => self.multiply_values(left_val, right_val),
                    BinaryOperator::Div => self.divide_values(left_val, right_val),
                    BinaryOperator::Mod => self.modulo_values(left_val, right_val),
                    BinaryOperator::Eq => Ok(Value::Boolean(left_val == right_val)),
                    BinaryOperator::Ne => Ok(Value::Boolean(left_val != right_val)),
                    BinaryOperator::Gt => self.compare_greater(left_val, right_val),
                    BinaryOperator::Lt => self.compare_less(left_val, right_val),
                    BinaryOperator::Ge => self.compare_greater_equal(left_val, right_val),
                    BinaryOperator::Le => self.compare_less_equal(left_val, right_val),
                    BinaryOperator::And => Ok(Value::Boolean(
                        left_val.is_truthy() && right_val.is_truthy(),
                    )),
                    BinaryOperator::Or => Ok(Value::Boolean(
                        left_val.is_truthy() || right_val.is_truthy(),
                    )),
                    BinaryOperator::Assign => Err(RuntimeError::InvalidOperation(
                        "Оператор присваивания не поддерживается в выражениях".to_string(),
                    )),
                }
            }

            ExpressionKind::Unary { op, operand } => {
                let value = self.evaluate_expression(*operand, program)?;

                match op {
                    UnaryOperator::Negative => match value {
                        Value::Number(n) => Ok(Value::Number(-n)),
                        _ => Err(RuntimeError::TypeMismatch(
                            "Унарный минус применим только к числам".to_string(),
                        )),
                    },
                    UnaryOperator::Not => Ok(Value::Boolean(!value.is_truthy())),
                }
            }

            ExpressionKind::FunctionCall { function, args } => {
                let func_expr = program.arena.get_expression(*function).unwrap();
                let func_name = match &func_expr.kind {
                    ExpressionKind::Identifier(symbol) => {
                        program.arena.resolve_symbol(*symbol).unwrap().to_string()
                    }
                    _ => {
                        return Err(RuntimeError::InvalidOperation(
                            "Вызов функции возможен только по имени".to_string(),
                        ))
                    }
                };

                let mut arguments = Vec::new();
                for &arg_id in args {
                    arguments.push(self.evaluate_expression(arg_id, program)?);
                }

                self.call_function_by_name(&func_name, arguments, program)
            }

            ExpressionKind::Index {
                object: _,
                index: _,
            } => Err(RuntimeError::InvalidOperation(
                "Индексный доступ отключён".to_string(),
            )),

            ExpressionKind::Input(arg_id) => {
                let data = self.evaluate_expression(*arg_id, program)?;
                self.input_function(data)
            }

            ExpressionKind::PropertyAccess { object, property } => {
                let obj_expr = program.arena.get_expression(*object).unwrap();

                if let ExpressionKind::Identifier(module_symbol) = obj_expr.kind {
                    let module_name = program.arena.resolve_symbol(module_symbol).unwrap();
                    let property_name =
                        program.arena.resolve_symbol(*property).unwrap().to_string();

                    if let Some(module_env) = self.modules.get(module_name) {
                        return module_env.environment.get(&property_name).ok_or_else(|| {
                            RuntimeError::UndefinedVariable(format!(
                                "{}.{}",
                                module_name, property_name
                            ))
                        });
                    }
                }

                let obj_result = self.evaluate_expression(*object, program);

                match obj_result {
                    Ok(Value::Object(instance_ref)) => {
                        let property_name =
                            program.arena.resolve_symbol(*property).unwrap().to_string();
                        let instance = instance_ref.borrow();

                        let is_external = !matches!(obj_expr.kind, ExpressionKind::This);

                        if !instance.is_field_accessible(&property_name, is_external) {
                            return Err(RuntimeError::InvalidOperation(format!(
                                "Поле '{}' недоступно для чтения",
                                property_name
                            )));
                        }

                        if let Some(value) = instance.get_field(&property_name) {
                            Ok(value.clone())
                        } else {
                            Ok(Value::Empty)
                        }
                    }
                    _ => {
                        if let ExpressionKind::Identifier(symbol) = obj_expr.kind {
                            let name = program.arena.resolve_symbol(symbol).unwrap();
                            Err(RuntimeError::InvalidOperation(format!(
                                "Модуль '{}' не найден",
                                name
                            )))
                        } else {
                            Err(RuntimeError::InvalidOperation(
                                "Попытка доступа к свойству не-объектного типа".to_string(),
                            ))
                        }
                    }
                }
            }

            ExpressionKind::MethodCall {
                object,
                method,
                args,
            } => {
                let obj_expr = program.arena.get_expression(*object).unwrap();
                let method_name = program.arena.resolve_symbol(*method).unwrap().to_string();

                let obj_result = self.evaluate_expression(*object, program);

                match obj_result {
                    Ok(Value::Object(instance_ref)) => {
                        let instance = instance_ref.borrow();

                        let is_external = !matches!(obj_expr.kind, ExpressionKind::This);

                        if !instance.is_method_accessible(&method_name, is_external) {
                            return Err(RuntimeError::InvalidOperation(format!(
                                "Метод '{}' недоступен для вызова",
                                method_name
                            )));
                        }

                        if let Some(method) = instance.get_method(&method_name) {
                            let method = method.clone();

                            let method_program = {
                                let mut found_program = program.clone();
                                for (_module_name, module) in &self.modules {
                                    let class_name = program.arena.resolve_symbol(instance.class_name).unwrap();
                                    if module.classes.contains_key(class_name) {
                                        found_program = module.program.clone();
                                        break;
                                    }
                                }
                                found_program
                            };

                            drop(instance);

                            let mut arguments = Vec::new();
                            for &arg_id in args {
                                arguments.push(self.evaluate_expression(arg_id, program)?);
                            }

                            self.call_method(
                                method,
                                arguments,
                                Value::Object(instance_ref),
                                &method_program,
                            )
                        } else {
                            let class_name = program.arena.resolve_symbol(instance.class_name).unwrap();
                            Err(RuntimeError::UndefinedFunction(format!(
                                "Метод '{}' не найден в классе '{}'",
                                method_name, class_name
                            )))
                        }
                    }
                    Ok(_) => {
                        if let ExpressionKind::Identifier(module_symbol) = obj_expr.kind {
                            let module_name = program.arena.resolve_symbol(module_symbol).unwrap();

                            if let Some(module_env) = self.modules.get(module_name).cloned() {
                                if let Some(function) = module_env.functions.get(&method_name) {
                                    let mut arguments = Vec::new();
                                    for &arg_id in args {
                                        arguments.push(self.evaluate_expression(arg_id, program)?);
                                    }

                                    return self.call_function(
                                        function.clone(),
                                        arguments,
                                        &module_env.program,
                                    );
                                } else {
                                    return Err(RuntimeError::UndefinedFunction(format!(
                                        "Функция '{}.{}' не найдена в модуле",
                                        module_name, method_name
                                    )));
                                }
                            }
                        }

                        Err(RuntimeError::TypeMismatch(
                            "Попытка вызова метода не-объектного типа".to_string(),
                        ))
                    }
                    Err(_) => {
                        if let ExpressionKind::Identifier(module_symbol) = obj_expr.kind {
                            let module_name = program.arena.resolve_symbol(module_symbol).unwrap();

                            if let Some(module_env) = self.modules.get(module_name).cloned() {
                                if let Some(function) = module_env.functions.get(&method_name) {
                                    let mut arguments = Vec::new();
                                    for &arg_id in args {
                                        arguments.push(self.evaluate_expression(arg_id, program)?);
                                    }

                                    return self.call_function(
                                        function.clone(),
                                        arguments,
                                        &module_env.program,
                                    );
                                } else {
                                    return Err(RuntimeError::UndefinedFunction(format!(
                                        "Функция '{}.{}' не найдена в модуле",
                                        module_name, method_name
                                    )));
                                }
                            }

                            Err(RuntimeError::InvalidOperation(format!(
                                "Модуль '{}' не найден",
                                module_name
                            )))
                        } else {
                            Err(RuntimeError::TypeMismatch(
                                "Попытка вызова метода не-объектного типа".to_string(),
                            ))
                        }
                    }
                }
            }

            ExpressionKind::ObjectCreation { class_name, args } => {
                let class_name_str = program
                    .arena
                    .resolve_symbol(*class_name)
                    .unwrap()
                    .to_string();

                let class = if class_name_str.contains('.') {
                    let parts: Vec<&str> = class_name_str.split('.').collect();
                    if parts.len() == 2 {
                        let module_name = parts[0];
                        let class_simple_name = parts[1];

                        if let Some(module) = self.modules.get(module_name) {
                            module.classes.get(class_simple_name).cloned()
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                } else {
                    self.classes.get(&class_name_str).cloned()
                };

                if let Some(class) = class {
                    let mut arguments = Vec::new();
                    for &arg_id in args {
                        arguments.push(self.evaluate_expression(arg_id, program)?);
                    }

                    let instance = class.create_instance();
                    let instance_ref = std::rc::Rc::new(std::cell::RefCell::new(instance));

                    if let Some(constructor) = {
                        let temp_borrow = instance_ref.borrow();
                        temp_borrow.get_constructor().cloned()
                    } {
                        let constructor_program = if class_name_str.contains('.') {
                            let parts: Vec<&str> = class_name_str.split('.').collect();
                            let module_name = parts[0];
                            self.modules.get(module_name).unwrap().program.clone()
                        } else {
                            program.clone()
                        };

                        self.call_method(
                            constructor,
                            arguments,
                            Value::Object(instance_ref.clone()),
                            &constructor_program,
                        )?;
                    }

                    Ok(Value::Object(instance_ref))
                } else {
                    Err(RuntimeError::UndefinedVariable(format!(
                        "Класс '{}' не найден",
                        class_name_str
                    )))
                }
            }

            ExpressionKind::This => self.environment.get("this").ok_or_else(|| {
                RuntimeError::InvalidOperation(
                    "'это' можно использовать только внутри методов класса".to_string(),
                )
            }),
        }
    }
}
