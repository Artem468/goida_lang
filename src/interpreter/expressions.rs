use crate::ast::prelude::{
    BinaryOperator, ClassDefinition, ErrorData, ExprId, ExpressionKind, LiteralValue, Span,
    UnaryOperator, Visibility,
};
use crate::ast::program::FieldData;
use crate::interpreter::structs::{Interpreter, RuntimeError, Value};
use crate::shared::SharedMut;
use crate::traits::prelude::{
    CoreOperations, ExpressionEvaluator, InterpreterClasses, InterpreterFunctions, ValueOperations,
};
use string_interner::DefaultSymbol as Symbol;

impl ExpressionEvaluator for Interpreter {
    fn evaluate_expression(
        &mut self,
        expr_id: ExprId,
        current_module_id: Symbol,
    ) -> Result<Value, RuntimeError> {
        let expr_kind = {
            let module = self.modules.get(&current_module_id).ok_or_else(|| {
                RuntimeError::InvalidOperation(ErrorData::new(
                    Span::default(),
                    "Модуль не найден".into(),
                ))
            })?;
            module.arena.get_expression(expr_id).unwrap().clone()
        };

        match expr_kind.kind {
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
                if let Some(val) = self.environment.get(&symbol) {
                    return Ok(val);
                }
                println!("DEBUG: Ищу идентификатор с ID: {:?}", symbol);

                let current_module = self.modules.get(&current_module_id).ok_or_else(|| {
                    RuntimeError::InvalidOperation(ErrorData::new(
                        expr_kind.span,
                        "Текущий модуль не найден".into(),
                    ))
                })?;

                if let Some(val) = current_module.globals.get(&symbol) {
                    return Ok(val.clone());
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

                    return if let Some(target_module) = self.modules.get(&mod_sym) {
                        target_module.globals.get(&var_sym).cloned().ok_or_else(|| {
                            RuntimeError::UndefinedVariable(ErrorData::new(
                                expr_kind.span,
                                name_str.clone(),
                            ))
                        })
                    } else {
                        Err(RuntimeError::InvalidOperation(ErrorData::new(
                            expr_kind.span,
                            format!("Модуль '{}' не найден", mod_name),
                        )))
                    };
                }

                for import in &current_module.imports {
                    for &imp_mod_sym in &import.files {
                        if let Some(m) = self.modules.get(&imp_mod_sym) {
                            if let Some(val) = m.globals.get(&symbol) {
                                return Ok(val.clone());
                            }
                            if let Some(recursive_val) = self.find_in_module_imports(&m, &symbol) {
                                return Ok(recursive_val);
                            }
                        }
                    }
                }

                for import in &current_module.imports {
                    for &imp_mod_sym in &import.files {
                        if let Some(mod_name) = self.resolve_symbol(imp_mod_sym) {
                            let this_name = self.resolve_symbol(symbol).unwrap_or_default();
                            if mod_name == this_name {
                                return Ok(Value::Module(imp_mod_sym));
                            }
                        }
                    }
                }

                Err(RuntimeError::UndefinedVariable(ErrorData::new(
                    expr_kind.span,
                    name_str,
                )))
            }

            ExpressionKind::Binary { op, left, right } => {
                let left_val = self.evaluate_expression(left, current_module_id)?;
                let right_val = self.evaluate_expression(right, current_module_id)?;

                match op {
                    BinaryOperator::Add => self.add_values(left_val, right_val, expr_kind.span),
                    BinaryOperator::Sub => {
                        self.subtract_values(left_val, right_val, expr_kind.span)
                    }
                    BinaryOperator::Mul => {
                        self.multiply_values(left_val, right_val, expr_kind.span)
                    }
                    BinaryOperator::Div => self.divide_values(left_val, right_val, expr_kind.span),
                    BinaryOperator::Mod => self.modulo_values(left_val, right_val, expr_kind.span),
                    BinaryOperator::Eq => Ok(Value::Boolean(left_val == right_val)),
                    BinaryOperator::Ne => Ok(Value::Boolean(left_val != right_val)),
                    BinaryOperator::Gt => self.compare_greater(left_val, right_val, expr_kind.span),
                    BinaryOperator::Lt => self.compare_less(left_val, right_val, expr_kind.span),
                    BinaryOperator::Ge => {
                        self.compare_greater_equal(left_val, right_val, expr_kind.span)
                    }
                    BinaryOperator::Le => {
                        self.compare_less_equal(left_val, right_val, expr_kind.span)
                    }
                    BinaryOperator::And => Ok(Value::Boolean(
                        left_val.is_truthy() && right_val.is_truthy(),
                    )),
                    BinaryOperator::Or => Ok(Value::Boolean(
                        left_val.is_truthy() || right_val.is_truthy(),
                    )),
                }
            }

            ExpressionKind::Unary { op, operand } => {
                let value = self.evaluate_expression(operand, current_module_id)?;

                match op {
                    UnaryOperator::Negative => match value {
                        Value::Number(n) => Ok(Value::Number(-n)),
                        _ => Err(RuntimeError::TypeMismatch(ErrorData::new(
                            expr_kind.span,
                            "Унарный минус применим только к числам".to_string(),
                        ))),
                    },
                    UnaryOperator::Not => Ok(Value::Boolean(!value.is_truthy())),
                }
            }

            ExpressionKind::FunctionCall { function, args } => {
                let func_expr = {
                    let module = self.modules.get(&current_module_id).ok_or_else(|| {
                        RuntimeError::InvalidOperation(ErrorData::new(
                            expr_kind.span,
                            "Модуль не найден".into(),
                        ))
                    })?;
                    module.arena.get_expression(expr_id).unwrap().clone()
                };

                let mut arguments = Vec::new();
                for arg_id in args {
                    arguments.push(self.evaluate_expression(arg_id, current_module_id)?);
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
                                (*f_def).clone(),
                                arguments,
                                current_module_id,
                                func_expr.span,
                            ),
                            Value::Builtin(builtin_func) => {
                                builtin_func(self, arguments, func_expr.span)
                            }
                            _ => Err(RuntimeError::InvalidOperation(ErrorData::new(
                                expr_kind.span.into(),
                                "Выражение не является вызываемой функцией".to_string(),
                            ))),
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
                            RuntimeError::InvalidOperation(ErrorData::new(
                                expr_kind.span,
                                format!("Индекс {} вне границ списка (длина {})", idx, vec.len()),
                            ))
                        })
                    }),

                    Value::Array(arr) => {
                        let idx = idx_val.resolve_index(arr.len(), expr_kind.span)?;

                        arr.get(idx).cloned().ok_or_else(|| {
                            RuntimeError::InvalidOperation(ErrorData::new(
                                expr_kind.span,
                                format!("Индекс {} вне границ списка (длина {})", idx, arr.len()),
                            ))
                        })
                    },

                    Value::Dict(dict) => dict.read(|d| {
                        let key = idx_val.to_string();
                        d.get(&key).cloned().ok_or_else(|| {
                            RuntimeError::InvalidOperation(ErrorData::new(
                                expr_kind.span,
                                format!("Ключ '{}' не найден в словаре", key),
                            ))
                        })
                    }),

                    _ => Err(RuntimeError::TypeError(ErrorData::new(
                        expr_kind.span,
                        "Операция [] доступна только для итерируемых коллекций".into(),
                    ))),
                }
            },

            ExpressionKind::PropertyAccess { object, property } => {
                let obj_expr = {
                    let module = self.modules.get(&current_module_id).ok_or_else(|| {
                        RuntimeError::InvalidOperation(ErrorData::new(
                            expr_kind.span,
                            "Модуль не найден".into(),
                        ))
                    })?;
                    module.arena.get_expression(expr_id).unwrap().clone()
                };
                if let ExpressionKind::Identifier(module_symbol) = obj_expr.kind {
                    if let Some(module_env) = self.modules.get(&module_symbol) {
                        return module_env
                            .globals
                            .get(&property)
                            .ok_or_else(|| {
                                RuntimeError::UndefinedVariable(ErrorData::new(
                                    expr_kind.span,
                                    format!(
                                        "{}.{}",
                                        self.resolve_symbol(module_symbol).unwrap(),
                                        self.resolve_symbol(property).unwrap()
                                    ),
                                ))
                            })
                            .cloned();
                    }
                }

                let obj_result = self.evaluate_expression(object, current_module_id);

                match obj_result {
                    Ok(Value::Module(module_symbol)) => {
                        if let Some(module_env) = self.modules.get(&module_symbol) {
                            module_env
                                .globals
                                .get(&property)
                                .ok_or_else(|| {
                                    RuntimeError::UndefinedVariable(ErrorData::new(
                                        expr_kind.span,
                                        format!(
                                            "{}.{}",
                                            self.resolve_symbol(module_symbol).unwrap(),
                                            self.resolve_symbol(property).unwrap()
                                        ),
                                    ))
                                })
                                .cloned()
                        } else {
                            Err(RuntimeError::InvalidOperation(ErrorData::new(
                                expr_kind.span,
                                format!(
                                    "Модуль '{}' не найден",
                                    self.resolve_symbol(module_symbol).unwrap()
                                ),
                            )))
                        }
                    }
                    Ok(Value::Object(instance_ref)) => {
                        let (field_data, is_accessible) = instance_ref.read(|instance| {
                            let is_external = !matches!(obj_expr.kind, ExpressionKind::This);
                            let this_sym = self.intern_string("this");
                            let is_inside_method = self.environment.get(&this_sym).is_some();
                            let final_is_external = is_external && !is_inside_method;

                            let accessible =
                                instance.is_field_accessible(&property, final_is_external);

                            let data = if let Some(val) = instance.field_values.get(&property) {
                                Some(Ok(val.clone()))
                            } else {
                                instance
                                    .get_field(&property)
                                    .cloned()
                                    .map(|opt_expr| Err(opt_expr))
                            };

                            (data, accessible)
                        });

                        if !is_accessible {
                            return Err(RuntimeError::InvalidOperation(ErrorData::new(
                                expr_kind.span,
                                format!(
                                    "Поле '{}' недоступно для чтения",
                                    self.resolve_symbol(property).unwrap()
                                ),
                            )));
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
                                    return Err(RuntimeError::UndefinedVariable(ErrorData::new(
                                        expr_kind.span,
                                        format!(
                                            "Статическое поле '{}' не найдено в классе '{}'",
                                            p_name, c_name
                                        ),
                                    )));
                                }

                                if !is_static {
                                    let p_name = self.resolve_symbol(property).unwrap_or_default();
                                    let c_name =
                                        self.resolve_symbol(class_name_sym).unwrap_or_default();
                                    return Err(RuntimeError::InvalidOperation(ErrorData::new(
                                        expr_kind.span,
                                        format!(
                                            "Поле '{}' класса '{}' не является статичным",
                                            p_name, c_name
                                        ),
                                    )));
                                }

                                let is_external = !matches!(obj_expr.kind, ExpressionKind::This);
                                if is_external && matches!(visibility, Visibility::Private) {
                                    let p_name = self.resolve_symbol(property).unwrap_or_default();
                                    return Err(RuntimeError::InvalidOperation(ErrorData::new(
                                        obj_expr.span,
                                        format!("Поле '{}' является приватным", p_name),
                                    )));
                                }

                                match data {
                                    FieldData::Value(val_shared) => {
                                        Ok(val_shared.read(|v| v.clone()))
                                    }
                                    FieldData::Expression(_) => {
                                        let p_name =
                                            self.resolve_symbol(property).unwrap_or_default();
                                        Err(RuntimeError::InvalidOperation(ErrorData::new(
                                            expr_kind.span,
                                            format!(
                                                "Статическое поле '{}' еще не инициализировано",
                                                p_name
                                            ),
                                        )))
                                    }
                                }
                            }
                            None => unreachable!(),
                        }
                    }

                    _ => {
                        if let ExpressionKind::Identifier(symbol) = obj_expr.kind {
                            Err(RuntimeError::InvalidOperation(ErrorData::new(
                                expr_kind.span,
                                format!(
                                    "Модуль '{}' не найден",
                                    self.resolve_symbol(symbol).unwrap()
                                ),
                            )))
                        } else {
                            Err(RuntimeError::InvalidOperation(ErrorData::new(
                                expr_kind.span,
                                "Попытка доступа к свойству не-объектного типа".to_string(),
                            )))
                        }
                    }
                }
            }

            ExpressionKind::MethodCall {
                object,
                method,
                args,
            } => {
                let obj_expr = {
                    let module = self.modules.get(&current_module_id).ok_or_else(|| {
                        RuntimeError::InvalidOperation(ErrorData::new(
                            expr_kind.span,
                            "Модуль не найден".into(),
                        ))
                    })?;
                    module.arena.get_expression(object).unwrap().clone()
                };

                let mut arguments = Vec::new();
                for arg_id in args {
                    arguments.push(self.evaluate_expression(arg_id, current_module_id)?);
                }

                let target_value = self.evaluate_expression(object, current_module_id)?;

                if let Some(class_def) = self.get_class_for_value(&target_value) {
                    let method_info = class_def.read(|c| {
                        c.methods.get(&method).map(|(vis, is_static, m_type)| {
                            (vis.clone(), *is_static, m_type.clone())
                        })
                    });

                    if let Some((visibility, is_static, method_type)) = method_info {
                        let is_calling_on_class = matches!(target_value, Value::Class(_));

                        if is_calling_on_class && !is_static {
                            return Err(RuntimeError::InvalidOperation(ErrorData::new(
                                expr_kind.span,
                                "Нельзя вызвать обычный метод у класса. Создайте экземпляр через 'новый'".into()
                            )));
                        }

                        let this_val = if is_static {
                            Value::Empty
                        } else {
                            target_value
                        };
                        let is_external = !matches!(obj_expr.kind, ExpressionKind::This);

                        if is_external && matches!(visibility, Visibility::Private) {
                            let m_name = self.resolve_symbol(method).unwrap_or_default();
                            return Err(RuntimeError::InvalidOperation(ErrorData::new(
                                obj_expr.span,
                                format!("Метод '{}' является приватным", m_name),
                            )));
                        }

                        let method_module = method_type.get_module().unwrap_or(current_module_id);

                        return self.call_method(
                            method_type,
                            arguments,
                            this_val,
                            method_module,
                            obj_expr.span,
                        );
                    }
                }

                match target_value {
                    Value::Module(mod_symbol) => {
                        if let Some(target_module) = self.modules.get(&mod_symbol) {
                            if let Some(function) = target_module.functions.get(&method) {
                                return self.call_function(
                                    function.clone(),
                                    arguments,
                                    mod_symbol,
                                    obj_expr.span,
                                );
                            } else {
                                let m_name = self.resolve_symbol(method).unwrap();
                                let mod_name = self.resolve_symbol(mod_symbol).unwrap();
                                return Err(RuntimeError::UndefinedFunction(ErrorData::new(
                                    expr_kind.span,
                                    format!(
                                        "Функция '{}' не найдена в модуле '{}'",
                                        m_name, mod_name
                                    ),
                                )));
                            }
                        }
                    }
                    _ => {
                        if let ExpressionKind::Identifier(mod_symbol) = obj_expr.kind {
                            if let Some(target_module) = self.modules.get(&mod_symbol) {
                                if let Some(function) = target_module.functions.get(&method) {
                                    return self.call_function(
                                        function.clone(),
                                        arguments,
                                        mod_symbol,
                                        obj_expr.span,
                                    );
                                }
                            }
                        }
                    }
                }

                let m_name = self.resolve_symbol(method).unwrap();
                Err(RuntimeError::UndefinedMethod(ErrorData::new(
                    expr_kind.span,
                    format!(
                        "Не удалось вызвать '{}': цель не является объектом или модулем",
                        m_name
                    ),
                )))
            }

            ExpressionKind::ObjectCreation { class_name, args } => {
                let mut arguments = Vec::new();
                for arg_id in args {
                    arguments.push(self.evaluate_expression(arg_id, current_module_id)?);
                }
                let (class_rc, definition_module) =
                    if let Some(Value::Class(cls)) = self.environment.get(&class_name) {
                        (cls.clone(), current_module_id)
                    } else {
                        let name_str = self.resolve_symbol(class_name).unwrap();

                        if let Some(dot_pos) = name_str.find('.') {
                            let mod_name = &name_str[..dot_pos];
                            let class_simple_name = &name_str[dot_pos + 1..];
                            let mod_sym = self.intern_string(mod_name);
                            let class_sym = self.intern_string(class_simple_name);

                            let target_module = self.modules.get(&mod_sym).ok_or_else(|| {
                                RuntimeError::InvalidOperation(ErrorData::new(
                                    expr_kind.span,
                                    format!("Модуль '{}' не найден", mod_name),
                                ))
                            })?;

                            let class = target_module.classes.get(&class_sym).ok_or_else(|| {
                                RuntimeError::UndefinedVariable(ErrorData::new(
                                    expr_kind.span,
                                    format!(
                                        "Класс '{}' не найден в модуле '{}'",
                                        class_simple_name, mod_name
                                    ),
                                ))
                            })?;

                            (class.clone(), mod_sym)
                        } else {
                            let current_mod = self.modules.get(&current_module_id).unwrap();

                            let found = if let Some(class) = current_mod.classes.get(&class_name) {
                                Some((class.clone(), current_module_id))
                            } else {
                                let mut res = None;
                                for import in &current_mod.imports {
                                    for &imp_mod_sym in &import.files {
                                        if let Some(m) = self.modules.get(&imp_mod_sym) {
                                            if let Some(c) = m.classes.get(&class_name) {
                                                res = Some((c.clone(), imp_mod_sym));
                                                break;
                                            }
                                        }
                                    }
                                    if res.is_some() {
                                        break;
                                    }
                                }
                                res
                            };

                            let final_found = found.or_else(|| {
                                self.std_classes
                                    .get(&class_name)
                                    .map(|c| (c.clone(), current_module_id))
                            });

                            final_found.ok_or_else(|| {
                                RuntimeError::UndefinedVariable(ErrorData::new(
                                    expr_kind.span,
                                    format!("Класс '{}' не найден", name_str),
                                ))
                            })?
                        }
                    };

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
                        expr_kind.span,
                    )?;
                }

                let data_key = self.interner.write(|i| i.get_or_intern("__data"));
                let internal_value =
                    instance_ref.write(|instance| instance.field_values.remove(&data_key));

                match internal_value {
                    Some(val) => Ok(val),
                    None => Ok(Value::Object(instance_ref)),
                }
            }

            ExpressionKind::This => {
                let this_sym = self.interner.write(|i| i.get_or_intern("this"));

                self.environment.get(&this_sym).ok_or_else(|| {
                    RuntimeError::InvalidOperation(ErrorData::new(
                        expr_kind.span.into(),
                        "'это' можно использовать только внутри методов класса".to_string(),
                    ))
                })
            }
        }
    }

    fn find_in_module_imports(
        &self,
        module: &crate::interpreter::structs::Module,
        symbol: &string_interner::DefaultSymbol,
    ) -> Option<Value> {
        for import in &module.imports {
            for &imp_mod_sym in &import.files {
                if let Some(m) = self.modules.get(&imp_mod_sym) {
                    if let Some(val) = m.globals.get(symbol) {
                        return Some(val.clone());
                    }
                    if let Some(recursive_val) = self.find_in_module_imports(&m, symbol) {
                        return Some(recursive_val);
                    }
                }
            }
        }
        None
    }
}
