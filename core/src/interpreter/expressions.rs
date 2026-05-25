use crate::ast::prelude::{
    BinaryOperator, ClassDefinition, ErrorData, ExprId, ExpressionKind, LiteralValue, Span,
    UnaryOperator, Visibility,
};
use crate::ast::program::FieldData;
use crate::interpreter::structs::{CallArgValue, Interpreter, Module, RuntimeError, Value};
use crate::shared::SharedMut;
use crate::traits::prelude::{
    CoreOperations, ExpressionEvaluator, InterpreterClasses, InterpreterFunctions, ValueOperations,
};
use crate::{bail_runtime, runtime_error};
use std::sync::Arc;
use string_interner::DefaultSymbol as Symbol;

impl ExpressionEvaluator for Interpreter {
    fn evaluate_expression(
        &mut self,
        expr_id: ExprId,
        current_module_id: Symbol,
    ) -> Result<Value, RuntimeError> {
        let expr_kind = {
            let module = self.modules.get(&current_module_id).ok_or_else(|| {
                let module_name = self.resolve_symbol(current_module_id).unwrap();
                runtime_error!(
                    InvalidOperation,
                    Span::default(),
                    "Модуль {module_name} не найден"
                )
            })?;
            module.arena.get_expression(expr_id).unwrap().clone()
        };
        let result = match expr_kind.kind {
            ExpressionKind::Literal(literal) => match literal {
                LiteralValue::Number(n) => Ok(Value::Number(n)),
                LiteralValue::Float(f) => Ok(Value::Float(f)),
                LiteralValue::Text(symbol) => {
                    let text = self.resolve_symbol(symbol).unwrap();
                    Ok(Value::Text(text))
                }
                LiteralValue::Boolean(b) => Ok(Value::Boolean(b)),
                LiteralValue::Unit => Ok(Value::Empty),
            },

            ExpressionKind::Identifier(symbol) => {
                if let Some(val) = self.environment.read(|env| env.get(&symbol)) {
                    return self.resolve_runtime_value(val, expr_kind.span);
                }

                let current_module = self.modules.get(&current_module_id).ok_or_else(|| {
                    runtime_error!(InvalidOperation, expr_kind.span, "Текущий модуль не найден")
                })?;

                if let Some(val) = current_module.globals.get(&symbol) {
                    return self.resolve_runtime_value(val.clone(), expr_kind.span);
                }

                if let Some(builtin) = self.builtins.get(&symbol) {
                    return Ok(Value::Builtin(builtin.clone()));
                }

                let name_str = self.resolve_symbol(symbol).unwrap();
                if let Some(dot_pos) = name_str.find('.') {
                    let mod_name = &name_str[..dot_pos];
                    let var_name = &name_str[dot_pos + 1..];

                    let mod_sym = self.interner.write(|i| i.get_or_intern(mod_name));
                    let var_sym = self.interner.write(|i| i.get_or_intern(var_name));

                    let target_module_symbol =
                        self.resolve_import_alias_symbol(current_module, mod_sym);

                    return if let Some(target_module) =
                        target_module_symbol.and_then(|sym| self.modules.get(&sym))
                    {
                        let value =
                            target_module
                                .globals
                                .get(&var_sym)
                                .cloned()
                                .ok_or_else(|| {
                                    runtime_error!(
                                        UndefinedVariable,
                                        expr_kind.span,
                                        "{}",
                                        name_str.clone()
                                    )
                                })?;
                        self.resolve_runtime_value(value, expr_kind.span)
                    } else {
                        bail_runtime!(
                            InvalidOperation,
                            expr_kind.span,
                            "Модуль '{}' не найден",
                            mod_name
                        )
                    };
                }

                if let Some(module_symbol) =
                    self.resolve_import_alias_symbol(current_module, symbol)
                {
                    return Ok(Value::Module(module_symbol));
                }
                bail_runtime!(UndefinedVariable, expr_kind.span, "{}", name_str)
            }

            ExpressionKind::Binary { op, left, right } => {
                let left_val = self.evaluate_expression(left, current_module_id)?;

                match op {
                    BinaryOperator::And => {
                        if !left_val.is_truthy() {
                            return Ok(Value::Boolean(false));
                        }
                        let right_val = self.evaluate_expression(right, current_module_id)?;
                        Ok(Value::Boolean(right_val.is_truthy()))
                    }

                    BinaryOperator::Or => {
                        if left_val.is_truthy() {
                            return Ok(Value::Boolean(true));
                        }
                        let right_val = self.evaluate_expression(right, current_module_id)?;
                        Ok(Value::Boolean(right_val.is_truthy()))
                    }

                    _ => {
                        let right_val = self.evaluate_expression(right, current_module_id)?;
                        match op {
                            BinaryOperator::Add => {
                                self.add_values(left_val, right_val, expr_kind.span)
                            }
                            BinaryOperator::Sub => {
                                self.subtract_values(left_val, right_val, expr_kind.span)
                            }
                            BinaryOperator::Mul => {
                                self.multiply_values(left_val, right_val, expr_kind.span)
                            }
                            BinaryOperator::Div => {
                                self.divide_values(left_val, right_val, expr_kind.span)
                            }
                            BinaryOperator::Mod => {
                                self.modulo_values(left_val, right_val, expr_kind.span)
                            }
                            BinaryOperator::Eq => Ok(Value::Boolean(left_val == right_val)),
                            BinaryOperator::Ne => Ok(Value::Boolean(left_val != right_val)),
                            BinaryOperator::Gt => {
                                self.compare_greater(left_val, right_val, expr_kind.span)
                            }
                            BinaryOperator::Lt => {
                                self.compare_less(left_val, right_val, expr_kind.span)
                            }
                            BinaryOperator::Ge => {
                                self.compare_greater_equal(left_val, right_val, expr_kind.span)
                            }
                            BinaryOperator::Le => {
                                self.compare_less_equal(left_val, right_val, expr_kind.span)
                            }
                            _ => unreachable!(),
                        }
                    }
                }
            }

            ExpressionKind::Unary { op, operand } => {
                let value = self.evaluate_expression(operand, current_module_id)?;

                match op {
                    UnaryOperator::Negative => match value {
                        Value::Number(n) => Ok(Value::Number(-n)),
                        _ => bail_runtime!(
                            TypeMismatch,
                            expr_kind.span,
                            "Унарный минус применим только к числам"
                        ),
                    },
                    UnaryOperator::Not => Ok(Value::Boolean(!value.is_truthy())),
                }
            }

            ExpressionKind::FunctionCall { function, args } => {
                let func_expr = {
                    let module = self.modules.get(&current_module_id).ok_or_else(|| {
                        let module_name = self.resolve_symbol(current_module_id).unwrap();
                        runtime_error!(
                            InvalidOperation,
                            expr_kind.span,
                            "Модуль {module_name} не найден"
                        )
                    })?;
                    module.arena.get_expression(function).unwrap().clone()
                };

                let mut arguments = Vec::new();
                for arg in args {
                    let value = self.evaluate_expression(arg.value, current_module_id)?;
                    arguments.push(CallArgValue {
                        name: arg.name,
                        value,
                    });
                }

                match &func_expr.kind {
                    ExpressionKind::Identifier(symbol) => self.call_function_by_name(
                        *symbol,
                        arguments,
                        current_module_id,
                        func_expr.span,
                    ),
                    _ => {
                        let func_value = self.evaluate_expression(function, current_module_id)?;
                        match func_value {
                            Value::Function(f_def) => self.call_function(
                                f_def.clone(),
                                arguments,
                                current_module_id,
                                func_expr.span,
                            ),
                            Value::Builtin(builtin_func) => {
                                builtin_func(self, arguments, func_expr.span)
                            }
                            _ => bail_runtime!(
                                InvalidOperation,
                                expr_kind.span,
                                "Выражение не является вызываемой функцией"
                            ),
                        }
                    }
                }
            }

            ExpressionKind::Index { object, index } => {
                let obj_val = self.evaluate_expression(object, current_module_id)?;
                let idx_val = self.evaluate_expression(index, current_module_id)?;

                match obj_val {
                    Value::List(list) => list.read(|vec| {
                        let idx = idx_val.resolve_index(vec.len(), expr_kind.span)?;

                        vec.get(idx).cloned().ok_or_else(|| {
                            runtime_error!(
                                InvalidOperation,
                                expr_kind.span,
                                "Индекс {} вне границ списка (длина {})",
                                idx,
                                vec.len()
                            )
                        })
                    }),

                    Value::Array(arr) => {
                        let idx = idx_val.resolve_index(arr.len(), expr_kind.span)?;

                        arr.get(idx).cloned().ok_or_else(|| {
                            runtime_error!(
                                InvalidOperation,
                                expr_kind.span,
                                "Индекс {} вне границ списка (длина {})",
                                idx,
                                arr.len()
                            )
                        })
                    }

                    Value::Dict(dict) => dict.read(|d| {
                        let key = idx_val.to_string();
                        d.get(&key).cloned().ok_or_else(|| {
                            runtime_error!(
                                InvalidOperation,
                                expr_kind.span,
                                "Ключ '{}' не найден в словаре",
                                key
                            )
                        })
                    }),

                    _ => bail_runtime!(
                        TypeError,
                        expr_kind.span,
                        "Операция [] доступна только для итерируемых коллекций"
                    ),
                }
            }

            ExpressionKind::PropertyAccess { object, property } => {
                let receiver_expr = {
                    let module = self.modules.get(&current_module_id).ok_or_else(|| {
                        let module_name = self.resolve_symbol(current_module_id).unwrap();
                        runtime_error!(
                            InvalidOperation,
                            expr_kind.span,
                            "Модуль {module_name} не найден"
                        )
                    })?;
                    module.arena.get_expression(object).unwrap().clone()
                };
                let obj_result = self.evaluate_expression(object, current_module_id);
                match obj_result {
                    Ok(Value::Module(module_symbol)) => {
                        if let Some((_, value)) =
                            self.resolve_module_member_value(module_symbol, property)
                        {
                            self.resolve_runtime_value(value, expr_kind.span)
                        } else {
                            bail_runtime!(
                                InvalidOperation,
                                expr_kind.span,
                                "Модуль '{}' не найден",
                                self.resolve_symbol(module_symbol).unwrap()
                            )
                        }
                    }
                    Ok(Value::Object(instance_ref)) => {
                        let (field_data, is_accessible) = instance_ref.read(|instance| {
                            let is_external = !matches!(receiver_expr.kind, ExpressionKind::This);
                            let is_inside_method = self.method_depth > 0;
                            let final_is_external = is_external && !is_inside_method;

                            let accessible =
                                instance.is_field_accessible(&property, final_is_external);

                            let data = if let Some(val) = instance.field_values.get(&property) {
                                Some(Ok(val.clone()))
                            } else {
                                instance.get_field(&property).cloned().map(Err)
                            };

                            (data, accessible)
                        });

                        if !is_accessible {
                            return bail_runtime!(
                                InvalidOperation,
                                expr_kind.span,
                                "Поле '{}' недоступно для чтения",
                                self.resolve_symbol(property).unwrap()
                            );
                        }
                        match field_data {
                            Some(Ok(value)) => Ok(value),
                            Some(Err(Some(expr))) => {
                                Ok(self.evaluate_expression(expr, current_module_id)?)
                            }
                            _ => Ok(Value::Empty),
                        }
                    }
                    Ok(Value::Class(class_def)) => {
                        let field_info = class_def.read(|c| {
                            c.fields
                                .get(&property)
                                .map(|(vis, is_static, data)| {
                                    (vis.clone(), *is_static, data.clone(), c.name)
                                })
                                .or_else(|| {
                                    Some((
                                        Visibility::Public,
                                        false,
                                        FieldData::Expression(None),
                                        c.name,
                                    ))
                                })
                        });

                        match field_info {
                            Some((visibility, is_static, data, class_name_sym)) => {
                                if !class_def.read(|c| c.fields.contains_key(&property)) {
                                    let p_name = self.resolve_symbol(property).unwrap_or_default();
                                    let c_name =
                                        self.resolve_symbol(class_name_sym).unwrap_or_default();
                                    return bail_runtime!(
                                        UndefinedVariable,
                                        expr_kind.span,
                                        "Статическое поле '{}' не найдено в классе '{}'",
                                        p_name,
                                        c_name
                                    );
                                }

                                if !is_static {
                                    let p_name = self.resolve_symbol(property).unwrap_or_default();
                                    let c_name =
                                        self.resolve_symbol(class_name_sym).unwrap_or_default();
                                    return bail_runtime!(
                                        InvalidOperation,
                                        expr_kind.span,
                                        "Поле '{}' класса '{}' не является статичным",
                                        p_name,
                                        c_name
                                    );
                                }

                                let is_external =
                                    !matches!(receiver_expr.kind, ExpressionKind::This);
                                if is_external && matches!(visibility, Visibility::Private) {
                                    let p_name = self.resolve_symbol(property).unwrap_or_default();
                                    return bail_runtime!(
                                        InvalidOperation,
                                        receiver_expr.span,
                                        "Поле '{}' является приватным",
                                        p_name
                                    );
                                }

                                match data {
                                    FieldData::Value(val_shared) => {
                                        Ok(val_shared.read(|v| v.clone()))
                                    }
                                    FieldData::Expression(_) => {
                                        let p_name =
                                            self.resolve_symbol(property).unwrap_or_default();
                                        bail_runtime!(
                                            InvalidOperation,
                                            expr_kind.span,
                                            "Статическое поле '{}' еще не инициализировано",
                                            p_name
                                        )
                                    }
                                }
                            }
                            None => unreachable!(),
                        }
                    }

                    _ => {
                        if let ExpressionKind::Identifier(symbol) = &receiver_expr.kind {
                            bail_runtime!(
                                InvalidOperation,
                                expr_kind.span,
                                "Модуль '{}' не найден",
                                self.resolve_symbol(*symbol).unwrap()
                            )
                        } else {
                            bail_runtime!(
                                InvalidOperation,
                                expr_kind.span,
                                "Попытка доступа к свойству не-объектного типа"
                            )
                        }
                    }
                }
            }

            ExpressionKind::MethodCall {
                object,
                method,
                args,
            } => {
                self.preserving_environment(|interpreter| {
                    let obj_expr = {
                        let module = interpreter.modules.get(&current_module_id).ok_or_else(|| {
                            let module_name = interpreter.resolve_symbol(current_module_id).unwrap();
                            runtime_error!(
                                InvalidOperation,
                                expr_kind.span,
                                "Модуль {module_name} не найден"
                            )
                        })?;
                        module.arena.get_expression(object).unwrap().clone()
                    };

                    let mut arguments = Vec::new();
                    for arg in args {
                        let value = interpreter.evaluate_expression(arg.value, current_module_id)?;
                        arguments.push(CallArgValue {
                            name: arg.name,
                            value,
                        });
                    }

                    let target_value = interpreter.evaluate_expression(object, current_module_id)?;

                    if let Some(class_def) = interpreter.get_class_for_value(&target_value) {
                        let method_info = class_def.read(|c| {
                            c.methods.get(&method).map(|(vis, is_static, m_type)| {
                                (vis.clone(), *is_static, m_type.clone())
                            })
                        });

                        if let Some((visibility, is_static, method_type)) = method_info {
                            let is_calling_on_class = matches!(target_value, Value::Class(_));

                            if is_calling_on_class && !is_static {
                                return bail_runtime!(
                                    InvalidOperation,
                                    expr_kind.span,
                                    "Нельзя вызвать обычный метод у класса. Создайте экземпляр через 'новый'"
                                );
                            }

                            let this_val = if is_static {
                                Value::Empty
                            } else {
                                target_value
                            };
                            let is_external = !matches!(obj_expr.kind, ExpressionKind::This);

                            if is_external && matches!(visibility, Visibility::Private) {
                                let m_name =
                                    interpreter.resolve_symbol(method).unwrap_or_default();
                                return bail_runtime!(
                                    InvalidOperation,
                                    obj_expr.span,
                                    "Метод '{}' является приватным",
                                    m_name
                                );
                            }

                            let method_module =
                                method_type.get_module().unwrap_or(current_module_id);

                            let method_name = interpreter.resolve_symbol(method).unwrap_or_default();
                            return interpreter.call_method(
                                method_type,
                                arguments,
                                this_val,
                                method_module,
                                obj_expr.span,
                            ).map_err(|mut err| {
                                err.add_stack_frame(format!("метод {}", method_name), obj_expr.span);
                                err
                            });
                        }
                    }

                    if let Value::Module(mod_symbol) = target_value {
                        if let Some((definition_module_id, value)) =
                            interpreter.resolve_module_member_value(mod_symbol, method)
                        {
                            return match value {
                                Value::Function(function) => interpreter.call_function(
                                    function.clone(),
                                    arguments,
                                    definition_module_id,
                                    obj_expr.span,
                                ),
                                Value::Builtin(builtin) => {
                                    builtin(interpreter, arguments, obj_expr.span)
                                }
                                Value::Class(class_def) => interpreter.instantiate_class(
                                    class_def.clone(),
                                    definition_module_id,
                                    arguments,
                                    obj_expr.span,
                                ),
                                _ => {
                                    let m_name = interpreter.resolve_symbol(method).unwrap();
                                    let mod_name = interpreter.resolve_symbol(mod_symbol).unwrap();
                                    bail_runtime!(
                                        UndefinedFunction,
                                        expr_kind.span,
                                        "Функция '{}' не найдена в модуле '{}'",
                                        m_name,
                                        mod_name
                                    )
                                }
                            };
                        }
                    }

                    let m_name = interpreter.resolve_symbol(method).unwrap();
                    bail_runtime!(
                        UndefinedMethod,
                        expr_kind.span,
                        "Не удалось вызвать '{}': цель не является объектом или модулем",
                        m_name
                    )
                })
            }

            ExpressionKind::ObjectCreation { class_name, args } => {
                let mut arguments = Vec::new();
                for arg in args {
                    let value = self.evaluate_expression(arg.value, current_module_id)?;
                    arguments.push(CallArgValue {
                        name: arg.name,
                        value,
                    });
                }

                let (class_rc, definition_module) =
                    self.resolve_class_for_creation(class_name, current_module_id, expr_kind.span)?;
                self.instantiate_class(class_rc, definition_module, arguments, expr_kind.span)
            }

            ExpressionKind::Lambda { params, body } => {
                let name = self.intern_string("<лямбда>");
                Ok(Value::Function(Arc::new(crate::ast::prelude::FunctionDefinition {
                    name,
                    params,
                    return_type: None,
                    body,
                    span: expr_kind.span,
                    module: Some(current_module_id),
                })))
            }

            ExpressionKind::This => bail_runtime!(
                InvalidOperation,
                expr_kind.span,
                "'это' не является ключевым словом среды выполнения, передайте получатель в качестве явного параметра метода"
            ),

        };
        result
    }
}

impl Interpreter {
    fn resolve_module_path(&self, current_module: &Module, parts: &[&str]) -> Option<Symbol> {
        let (first, rest) = parts.split_first()?;
        let first_symbol = self.intern_string(first);
        let mut module_id = self.resolve_import_alias_symbol(current_module, first_symbol)?;

        for part in rest {
            let part_symbol = self.intern_string(part);
            let (_, value) = self.resolve_module_member_value(module_id, part_symbol)?;
            match value {
                Value::Module(next_module_id) => module_id = next_module_id,
                _ => return None,
            }
        }

        Some(module_id)
    }

    fn resolve_class_for_creation(
        &self,
        class_name: Symbol,
        current_module_id: Symbol,
        span: Span,
    ) -> Result<(SharedMut<ClassDefinition>, Symbol), RuntimeError> {
        if let Some(Value::Class(cls)) = self.environment.read(|env| env.get(&class_name)) {
            return Ok((cls.clone(), current_module_id));
        }

        let name_str = self.resolve_symbol(class_name).unwrap();
        let parts = name_str.split('.').collect::<Vec<_>>();

        if parts.len() > 1 {
            let class_simple_name = parts.last().copied().unwrap_or_default();
            let module_parts = &parts[..parts.len() - 1];
            let module_name = module_parts.join(".");
            let current_mod = self.modules.get(&current_module_id).unwrap();
            let target_module_id = self
                .resolve_module_path(current_mod, module_parts)
                .ok_or_else(|| {
                    runtime_error!(InvalidOperation, span, "Модуль '{}' не найден", module_name)
                })?;
            let target_module = self.modules.get(&target_module_id).ok_or_else(|| {
                runtime_error!(InvalidOperation, span, "Модуль '{}' не найден", module_name)
            })?;
            let class_sym = self.intern_string(class_simple_name);
            let class = target_module.classes.get(&class_sym).ok_or_else(|| {
                runtime_error!(
                    UndefinedVariable,
                    span,
                    "Класс '{}' не найден в модуле '{}'",
                    class_simple_name,
                    module_name
                )
            })?;

            return Ok((class.clone(), target_module.name));
        }

        let current_mod = self.modules.get(&current_module_id).unwrap();
        let found = current_mod
            .classes
            .get(&class_name)
            .map(|class| (class.clone(), current_module_id));
        let final_found = found.or_else(|| {
            self.std_classes
                .get(&class_name)
                .map(|c| (c.clone(), current_module_id))
        });

        final_found.ok_or_else(|| {
            runtime_error!(UndefinedVariable, span, "Класс '{}' не найден", name_str)
        })
    }

    pub(crate) fn instantiate_class(
        &mut self,
        class_rc: SharedMut<ClassDefinition>,
        definition_module: Symbol,
        arguments: Vec<CallArgValue>,
        span: Span,
    ) -> Result<Value, RuntimeError> {
        let class_rc = self.set_class_module(class_rc, definition_module);
        let instance = ClassDefinition::create_instance(class_rc.clone());
        let instance_ref = SharedMut::new(instance);

        let constructor_opt = class_rc.read(|c| c.constructor.clone());

        if let Some(constructor) = constructor_opt {
            let constructor_module = constructor.get_module().unwrap_or(definition_module);
            self.call_method(
                constructor,
                arguments,
                Value::Object(instance_ref.clone()),
                constructor_module,
                span,
            )?;
        }

        let data_key = self.interner.write(|i| i.get_or_intern("__data"));
        let internal_value = instance_ref.write(|instance| instance.field_values.remove(&data_key));

        match internal_value {
            Some(val) => Ok(val),
            None => Ok(Value::Object(instance_ref)),
        }
    }
}
