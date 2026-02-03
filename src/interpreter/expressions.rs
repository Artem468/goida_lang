use crate::ast::prelude::{
    BinaryOperator, ErrorData, ExprId, ExpressionKind, LiteralValue, Span, UnaryOperator,
    Visibility,
};
use crate::interpreter::structs::{Interpreter, RuntimeError, Value};
use crate::traits::prelude::{
    CoreOperations, ExpressionEvaluator, InterpreterClasses, InterpreterFunctions, ValueOperations,
};
use std::cell::RefCell;
use std::rc::Rc;
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

                    let mod_sym = self.interner.write().unwrap().get_or_intern(mod_name);
                    let var_sym = self.interner.write().unwrap().get_or_intern(var_name);

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
                    BinaryOperator::Assign => Err(RuntimeError::InvalidOperation(ErrorData::new(
                        expr_kind.span,
                        "Оператор присваивания не поддерживается в выражениях".to_string(),
                    ))),
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

            ExpressionKind::Index {
                object: _,
                index: _,
            } => Err(RuntimeError::InvalidOperation(ErrorData::new(
                expr_kind.span,
                "Индексный доступ отключён".to_string(),
            ))),

            ExpressionKind::PropertyAccess { object, property } => {
                let obj_expr = {
                    let module = self.modules.get(&current_module_id).ok_or_else(|| {
                        RuntimeError::InvalidOperation(ErrorData::new(
                            expr_kind.span,
                            "Модуль не найден".into(),
                        ))
                    })?;
                    module.arena.get_expression(expr_id).unwrap().kind.clone()
                };
                if let ExpressionKind::Identifier(module_symbol) = obj_expr {
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
                        let instance = instance_ref.borrow();

                        let is_external = !matches!(obj_expr, ExpressionKind::This);

                        let this_sym = self.intern_string("this");
                        let is_inside_method = self.environment.get(&this_sym).is_some();
                        let is_external = is_external && !is_inside_method;

                        if !instance.is_field_accessible(&property, is_external) {
                            return Err(RuntimeError::InvalidOperation(ErrorData::new(
                                expr_kind.span,
                                format!(
                                    "Поле '{}' недоступно для чтения",
                                    self.resolve_symbol(property).unwrap()
                                ),
                            )));
                        }

                        if let Some(opt_expr) = instance.get_field(&property) {
                            if let Some(computed_value) = instance.field_values.get(&property) {
                                Ok(computed_value.clone())
                            } else {
                                match opt_expr {
                                    Some(expr) => {
                                        Ok(self.evaluate_expression(*expr, current_module_id)?)
                                    }
                                    None => Ok(Value::Empty),
                                }
                            }
                        } else {
                            Ok(Value::Empty)
                        }
                    }
                    _ => {
                        if let ExpressionKind::Identifier(symbol) = obj_expr {
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
                    if let Some((visibility, is_static, method_type)) = class_def.methods.get(&method) {
                        let is_calling_on_class = matches!(target_value, Value::Class(_));

                        if is_calling_on_class && !*is_static {
                            return Err(RuntimeError::InvalidOperation(ErrorData::new(
                                expr_kind.span,
                                "Нельзя вызвать обычный метод у класса. Создайте экземпляр через 'новый'".into()
                            )));
                        }

                        let this_val = if *is_static {
                            Value::Empty
                        } else {
                            target_value
                        };

                        let is_external = !matches!(obj_expr.kind, ExpressionKind::This);

                        if is_external && matches!(visibility, Visibility::Private) {
                            let m_name = self.resolve_symbol(method).unwrap();
                            return Err(RuntimeError::InvalidOperation(ErrorData::new(
                                obj_expr.span,
                                format!("Метод '{}' является приватным", m_name),
                            )));
                        }

                        let method_type = method_type.clone();
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
                let (class_rc, definition_module) = if let Some(Value::Class(cls)) = self.environment.get(&class_name) {
                    (cls.clone(), current_module_id)
                } else {
                    let name_str = self.resolve_symbol(class_name).unwrap();

                    if let Some(dot_pos) = name_str.find('.') {
                        let mod_name = &name_str[..dot_pos];
                        let class_simple_name = &name_str[dot_pos + 1..];
                        let mod_sym = self.intern_string(mod_name);
                        let class_sym = self.intern_string(class_simple_name);

                        let target_module = self.modules.get(&mod_sym).ok_or_else(|| {
                            RuntimeError::InvalidOperation(ErrorData::new(expr_kind.span, format!("Модуль '{}' не найден", mod_name)))
                        })?;

                        let class = target_module.classes.get(&class_sym).ok_or_else(|| {
                            RuntimeError::UndefinedVariable(ErrorData::new(expr_kind.span, format!("Класс '{}' не найден в модуле '{}'", class_simple_name, mod_name)))
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
                                if res.is_some() { break; }
                            }
                            res
                        };

                        let final_found = found.or_else(|| {
                            self.std_classes.get(&class_name).map(|c| (c.clone(), current_module_id))
                        });

                        final_found.ok_or_else(|| {
                            RuntimeError::UndefinedVariable(ErrorData::new(expr_kind.span, format!("Класс '{}' не найден", name_str)))
                        })?
                    }
                };

                let instance = class_rc.create_instance();
                let instance_ref = Rc::new(RefCell::new(instance));

                if let Some(constructor) = class_rc.constructor.clone() {
                    let constructor_module = constructor.get_module().unwrap_or(definition_module);

                    self.call_method(
                        constructor,
                        arguments,
                        Value::Object(instance_ref.clone()),
                        constructor_module,
                        expr_kind.span,
                    )?;
                }

                let data_key = self.interner.write().unwrap().get_or_intern("__data");
                let mut instance_borrow = instance_ref.borrow_mut();

                if let Some(internal_value) = instance_borrow.field_values.remove(&data_key) {
                    Ok(internal_value)
                } else {
                    drop(instance_borrow);
                    Ok(Value::Object(instance_ref))
                }
            }

            ExpressionKind::This => {
                let this_sym = self.interner.write().unwrap().get_or_intern("this");

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
