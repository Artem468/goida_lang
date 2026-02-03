use crate::ast::prelude::{ClassDefinition, ErrorData, ExprId, Span, Visibility};
use crate::ast::program::MethodType;
use crate::interpreter::prelude::{ClassInstance, Environment, Interpreter, RuntimeError, Value};
use crate::traits::prelude::{CoreOperations, InterpreterClasses, StatementExecutor};
use std::collections::HashMap;
use std::rc::Rc;
use string_interner::DefaultSymbol as Symbol;

impl InterpreterClasses for Interpreter {
    /// Вызываем метод
    fn call_method(
        &mut self,
        method: MethodType,
        arguments: Vec<Value>,
        this_obj: Value,
        current_module_id: Symbol,
        span: Span
    ) -> Result<Value, RuntimeError> {
        match method {
            MethodType::User(func) => {
                let method_module = func.module.unwrap_or(current_module_id);

                let previous_env = std::mem::replace(&mut self.environment, Environment::new());
                let mut local_env = Environment::with_parent(previous_env.clone());

                let this_sym = self.intern_string("this");
                local_env.define(this_sym, this_obj);

                if arguments.len() != func.params.len() {
                    self.environment = previous_env;
                    return Err(RuntimeError::InvalidOperation(ErrorData::new(
                        span,
                        format!("Ожидалось {} аргументов, получено {}", func.params.len(), arguments.len()),
                    )));
                }

                for (param, arg_value) in func.params.iter().zip(arguments.iter()) {
                    local_env.define(param.name, arg_value.clone());
                }

                self.environment = local_env;

                let mut result = Ok(Value::Empty);
                match self.execute_statement(func.body, method_module) {
                    Ok(()) => {}
                    Err(RuntimeError::Return(_, val)) => result = Ok(val),
                    Err(e) => result = Err(e),
                }

                self.environment = previous_env;
                result
            }

            MethodType::Native(builtin) => {
                let mut final_args = vec![this_obj];
                final_args.extend(arguments);
                builtin(self, final_args, span)
            }
        }
    }

    fn set_class_module(
        &self,
        class_def: Rc<ClassDefinition>,
        module: Symbol,
    ) -> Rc<ClassDefinition> {
        let mut methods = HashMap::new();
        for (method_name, (visibility, is_static, method_type)) in &class_def.methods {
            let updated_method = match method_type {
                MethodType::User(func_def) => {
                    let mut updated_func = func_def.clone();
                    updated_func.module = Some(module);
                    MethodType::User(updated_func)
                }
                MethodType::Native(builtin) => MethodType::Native(builtin.clone()),
            };
            methods.insert(*method_name, (visibility.clone(), *is_static, updated_method));
        }

        Rc::new(ClassDefinition {
            name: class_def.name,
            fields: class_def.fields.clone(),
            methods,
            constructor: class_def.constructor.as_ref().map(|constructor| {
                let updated_constructor = match constructor {
                    MethodType::User(func_def) => {
                        let mut updated_func = func_def.clone();
                        updated_func.module = Some(module);
                        MethodType::User(updated_func)
                    }
                    MethodType::Native(builtin) => MethodType::Native(builtin.clone()),
                };
                updated_constructor
            }),
            span: class_def.span,
        })
    }
}

impl ClassInstance {
    /// Создать новый экземпляр класса
    pub fn new(class_name: Symbol, class_ref: Rc<ClassDefinition>) -> Self {
        let mut fields = HashMap::new();

        for (field_name, (_, _, default_value)) in &class_ref.fields {
            if let Some(default) = default_value {
                fields.insert(field_name.clone(), Some(default.clone()));
            } else {
                fields.insert(field_name.clone(), None);
            }
        }

        Self {
            class_name,
            fields,
            field_values: HashMap::new(),
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
        if let Some((visibility, _, _)) = self.class_ref.fields.get(field_name) {
            match visibility {
                Visibility::Public => true,
                Visibility::Private => !is_external_access,
            }
        } else {
            false
        }
    }

    /// Получить метод по имени
    pub fn get_method(&self, method_name: &Symbol) -> Option<&MethodType> {
        self.class_ref
            .methods
            .get(method_name)
            .map(|(_, _, func)| func)
    }

    /// Получить конструктор класса
    pub fn get_constructor(&self) -> Option<&MethodType> {
        self.class_ref.constructor.as_ref()
    }
}

impl ClassDefinition {
    /// Создать новый класс
    pub fn new(name: Symbol, span: Span) -> Self {
        Self {
            name,
            fields: HashMap::new(),
            methods: HashMap::new(),
            constructor: None,
            span,
        }
    }

    /// Добавить поле в класс
    pub fn add_field(
        &mut self,
        name: Symbol,
        visibility: Visibility,
        is_static: bool,
        default_value: Option<ExprId>,
    ) {
        self.fields.insert(name, (visibility, is_static, default_value));
    }

    /// Добавить метод в класс
    pub fn add_method<F: Into<MethodType>>(&mut self, name: Symbol, visibility: Visibility, is_static: bool, method: F) {
        self.methods.insert(name, (visibility, is_static, method.into()));
    }

    /// Установить конструктор
    pub fn set_constructor<F: Into<MethodType>>(&mut self, constructor: F) {
        self.constructor = Some(constructor.into());
    }

    /// Создать экземпляр класса
    pub fn create_instance(self: &Rc<Self>) -> ClassInstance {
        ClassInstance::new(self.name, self.clone())
    }
}
