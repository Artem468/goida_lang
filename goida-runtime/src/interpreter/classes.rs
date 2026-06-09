use crate::ast::prelude::{ClassDefinition, ErrorData, ExprId, Span, Visibility};
use crate::ast::program::{FieldData, MethodType};
use crate::interpreter::prelude::{
    CallArgValue, ClassInstance, Interpreter, RuntimeClassDefinition, RuntimeError,
    RuntimeFieldData, RuntimeMethodType, Value,
};
use crate::shared::SharedMut;
use crate::traits::prelude::InterpreterClasses;
use crate::vm::Vm;
use std::collections::HashMap;
use std::sync::Arc;
use string_interner::DefaultSymbol as Symbol;

impl InterpreterClasses for Interpreter {
    /// Вызываем метод
    fn call_method(
        &mut self,
        method: RuntimeMethodType,
        arguments: Vec<CallArgValue>,
        this_obj: Value,
        current_module_id: Symbol,
        span: Span,
    ) -> Result<Value, RuntimeError> {
        match method {
            RuntimeMethodType::User(func) => {
                let method_module = func.module.unwrap_or(current_module_id);
                let method_name = self
                    .modules
                    .get(&method_module)
                    .and_then(|m| m.arena.resolve_symbol(&self.interner, func.name))
                    .unwrap_or_else(|| "неизвестно".to_string());
                let mut arguments = arguments;
                if !matches!(this_obj, Value::Empty) {
                    arguments.insert(
                        0,
                        CallArgValue {
                            name: None,
                            value: this_obj,
                        },
                    );
                }

                let final_arguments =
                    self.bind_call_arguments(&func, arguments, method_module, span, "Метод")?;

                let chunk = self
                    .modules
                    .get(&method_module)
                    .and_then(|module| module.bytecode.bodies.get(&func.body))
                    .cloned();
                let execution_result = self.scoped_method_context(|interpreter| {
                    interpreter.scoped_child_environment(
                        |local_env| {
                            for (param, arg_value) in func.params.iter().zip(final_arguments.iter())
                            {
                                local_env.define(param.name, arg_value.clone());
                            }
                        },
                        |interpreter| {
                            let chunk = chunk.as_ref().ok_or_else(|| {
                                crate::runtime_error!(
                                    InvalidOperation,
                                    span,
                                    "Compiled method body is missing"
                                )
                            })?;
                            Vm::new(interpreter, method_module).run(chunk)
                        },
                    )
                });

                match execution_result {
                    Ok(()) => Ok(Value::Empty),
                    Err(RuntimeError::Return(_, val)) => Ok(val),
                    Err(mut e) => {
                        e.add_stack_frame(format!("метод {}", method_name), span);
                        Err(e)
                    }
                }
            }

            RuntimeMethodType::Native(builtin) => {
                let has_receiver = !matches!(this_obj, Value::Empty);
                let mut final_args =
                    Vec::with_capacity(arguments.len() + usize::from(has_receiver));
                if has_receiver {
                    final_args.push(CallArgValue {
                        name: None,
                        value: this_obj,
                    });
                }
                final_args.extend(arguments);
                builtin(self, final_args, span)
            }
        }
    }

    fn set_class_module(
        &self,
        class_def: SharedMut<RuntimeClassDefinition>,
        module: Symbol,
    ) -> SharedMut<RuntimeClassDefinition> {
        let mut methods = HashMap::new();
        class_def.read(|i| {
            for (method_name, (visibility, is_static, method_type)) in &i.methods {
                let updated_method = match method_type {
                    RuntimeMethodType::User(func_def) => {
                        let mut updated_func = func_def.clone();
                        Arc::make_mut(&mut updated_func).module = Some(module);
                        RuntimeMethodType::User(updated_func)
                    }
                    RuntimeMethodType::Native(builtin) => {
                        RuntimeMethodType::Native(builtin.clone())
                    }
                };

                methods.insert(
                    *method_name,
                    (visibility.clone(), *is_static, updated_method),
                );
            }
        });

        let new_class_def = class_def.read(|c| RuntimeClassDefinition {
            name: c.name,
            base_class: c.base_class,
            fields: c.fields.clone(),
            methods,
            constructor: c.constructor.as_ref().map(|constructor| match constructor {
                RuntimeMethodType::User(func_def) => {
                    let mut updated_func = func_def.clone();
                    Arc::make_mut(&mut updated_func).module = Some(module);
                    RuntimeMethodType::User(updated_func)
                }
                RuntimeMethodType::Native(builtin) => RuntimeMethodType::Native(builtin.clone()),
            }),
            span: c.span,
        });

        SharedMut::new(new_class_def)
    }
}

impl ClassInstance {
    /// Создать новый экземпляр класса
    pub fn new(class_name: Symbol, class_ref: SharedMut<RuntimeClassDefinition>) -> Self {
        let mut fields = HashMap::new();
        let mut field_values = HashMap::new();

        class_ref.read(|class_def| {
            for (name, (_, is_static, data)) in &class_def.fields {
                if *is_static {
                    continue;
                }

                match data {
                    RuntimeFieldData::Expression(opt_expr) => {
                        fields.insert(*name, *opt_expr);
                    }
                    RuntimeFieldData::Value(val_lock) => {
                        let value = val_lock.read(|v| v.clone());
                        field_values.insert(*name, value);
                    }
                }
            }
        });

        Self {
            class_name,
            fields,
            field_values,
            class_ref,
        }
    }

    /// Получить значение поля
    pub fn get_field(&self, field_name: &Symbol) -> Option<&Option<ExprId>> {
        self.fields.get(field_name)
    }

    /// Установить значение поля
    pub fn set_field(&mut self, field_name: Symbol, expr_id: ExprId) {
        self.fields.insert(field_name, Some(expr_id));
    }

    /// Установить значение поля с вычисленным Value
    pub fn set_field_value(&mut self, field_name: Symbol, value: Value) {
        self.field_values.insert(field_name, value);
    }

    /// Проверить доступность поля (приватный или публичный доступ)
    pub fn is_field_accessible(&self, field_name: &Symbol, is_external_access: bool) -> bool {
        // 1. Сначала проверяем статическое определение в классе (там права доступа)
        let access_from_class = self.class_ref.read(|class| {
            class.fields.get(field_name).map(|(vis, _, _)| match vis {
                Visibility::Public => true,
                Visibility::Private => !is_external_access,
            })
        });

        if let Some(allowed) = access_from_class {
            return allowed;
        }

        // 2. Если в классе поле не описано, проверяем, существует ли оно в инстансе
        // (Это позволяет динамически добавлять поля в конструкторе)
        self.field_values.contains_key(field_name)
    }

    /// Получить метод по имени
    pub fn get_method(&self, method_name: &Symbol) -> Option<RuntimeMethodType> {
        self.class_ref.read(|class| {
            class
                .methods
                .get(method_name)
                .map(|(_, _, func)| func.clone())
        })
    }

    /// Получить конструктор класса
    pub fn get_constructor(&self) -> Option<RuntimeMethodType> {
        self.class_ref.read(|class| class.constructor.clone())
    }
}

impl RuntimeClassDefinition {
    pub fn from_syntax(class: &ClassDefinition) -> Self {
        Self {
            name: class.name,
            base_class: class.base_class,
            fields: class
                .fields
                .iter()
                .map(|(name, (visibility, is_static, data))| {
                    let data = match data {
                        FieldData::Expression(value) => RuntimeFieldData::Expression(*value),
                        FieldData::Value(()) => {
                            unreachable!("syntax classes cannot contain values")
                        }
                    };
                    (*name, (visibility.clone(), *is_static, data))
                })
                .collect(),
            methods: class
                .methods
                .iter()
                .map(|(name, (visibility, is_static, method))| {
                    let method = match method {
                        MethodType::User(function) => RuntimeMethodType::User(function.clone()),
                        MethodType::Native(_) => {
                            unreachable!("syntax classes cannot contain native methods")
                        }
                    };
                    (*name, (visibility.clone(), *is_static, method))
                })
                .collect(),
            constructor: class.constructor.as_ref().map(|method| match method {
                MethodType::User(function) => RuntimeMethodType::User(function.clone()),
                MethodType::Native(_) => {
                    unreachable!("syntax classes cannot contain native constructors")
                }
            }),
            span: class.span,
        }
    }

    /// Создать новый класс
    pub fn new(name: Symbol, span: Span) -> Self {
        Self {
            name,
            base_class: None,
            fields: HashMap::new(),
            methods: HashMap::new(),
            constructor: None,
            span,
        }
    }

    pub fn new_with_base(name: Symbol, base_class: Option<Symbol>, span: Span) -> Self {
        Self {
            name,
            base_class,
            fields: HashMap::new(),
            methods: HashMap::new(),
            constructor: None,
            span,
        }
    }

    /// Добавить поле в класс
    pub fn add_field<T: Into<RuntimeFieldData>>(
        &mut self,
        name: Symbol,
        visibility: Visibility,
        is_static: bool,
        default_value: T,
    ) {
        self.fields
            .insert(name, (visibility, is_static, default_value.into()));
    }

    /// Добавить метод в класс
    pub fn add_method<F: Into<RuntimeMethodType>>(
        &mut self,
        name: Symbol,
        visibility: Visibility,
        is_static: bool,
        method: F,
    ) {
        self.methods
            .insert(name, (visibility, is_static, method.into()));
    }

    /// Установить конструктор
    pub fn set_constructor<F: Into<RuntimeMethodType>>(&mut self, constructor: F) {
        self.constructor = Some(constructor.into());
    }

    /// Создать экземпляр класса
    pub fn create_instance(this: SharedMut<Self>) -> ClassInstance {
        let name = this.read(|i| i.name);
        ClassInstance::new(name, this)
    }

    pub fn to_syntax(&self) -> ClassDefinition {
        ClassDefinition {
            name: self.name,
            base_class: self.base_class,
            fields: self
                .fields
                .iter()
                .filter_map(|(name, (visibility, is_static, data))| match data {
                    RuntimeFieldData::Expression(value) => Some((
                        *name,
                        (
                            visibility.clone(),
                            *is_static,
                            FieldData::Expression(*value),
                        ),
                    )),
                    RuntimeFieldData::Value(_) => None,
                })
                .collect(),
            methods: self
                .methods
                .iter()
                .filter_map(|(name, (visibility, is_static, method))| match method {
                    RuntimeMethodType::User(function) => Some((
                        *name,
                        (
                            visibility.clone(),
                            *is_static,
                            MethodType::User(function.clone()),
                        ),
                    )),
                    RuntimeMethodType::Native(_) => None,
                })
                .collect(),
            constructor: self.constructor.as_ref().and_then(|method| match method {
                RuntimeMethodType::User(function) => Some(MethodType::User(function.clone())),
                RuntimeMethodType::Native(_) => None,
            }),
            span: self.span,
        }
    }
}
