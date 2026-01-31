use crate::ast::prelude::{ClassDefinition, ErrorData, ExprId, FunctionDefinition, Span, Visibility};
use crate::interpreter::prelude::{ClassInstance, Environment, Interpreter, RuntimeError, Value};
use crate::traits::prelude::{CoreOperations, InterpreterClasses, StatementExecutor};
use std::collections::HashMap;
use std::rc::Rc;
use string_interner::DefaultSymbol as Symbol;

impl InterpreterClasses for Interpreter {
    /// Регистрируем класс в текущем модуле
    fn register_class(
        &mut self,
        class_def: Rc<ClassDefinition>,
        current_module_id: Symbol,
    ) -> Result<(), RuntimeError> {
        if let Some(module) = self.modules.get_mut(&current_module_id) {
            module.classes.insert(class_def.name, class_def.clone());
        }

        Ok(())
    }

    /// Вызываем метод
    fn call_method(
        &mut self,
        method: FunctionDefinition,
        arguments: Vec<Value>,
        this_obj: Value,
        current_module_id: Symbol,
        span: Span
    ) -> Result<Value, RuntimeError> {
        let method_module = method.module.unwrap_or(current_module_id);

        let previous_env = std::mem::replace(&mut self.environment, Environment::new());
        let mut local_env = Environment::with_parent(previous_env.clone());

        let this_sym = self.intern_string("this");
        local_env.define(this_sym, this_obj);

        if arguments.len() != method.params.len() {
            self.environment = local_env;
            let _ = std::mem::replace(&mut self.environment, previous_env);
            return Err(RuntimeError::InvalidOperation(
                ErrorData::new(
                    span.into(),
                    "Неверное кол-во аргументов".into(),
            )));
        }

        for (param, arg_value) in method.params.iter().zip(arguments.iter()) {
            local_env.define(param.name, arg_value.clone());
        }

        self.environment = local_env;

        let mut result = Value::Empty;
        match self.execute_statement(method.body, method_module) {
            Ok(()) => {}
            Err(RuntimeError::Return(_, val)) => result = val,
            Err(e) => {
                self.environment = previous_env;
                return Err(e);
            }
        }

        self.environment = previous_env;
        Ok(result)
    }

    fn set_class_module(
        &self,
        class_def: Rc<ClassDefinition>,
        module: Symbol,
    ) -> Rc<ClassDefinition> {
        let mut methods = HashMap::new();
        for (method_name, (visibility, method_def)) in &class_def.methods {
            let mut updated_method = method_def.clone();
            updated_method.module = Some(module);
            methods.insert(*method_name, (visibility.clone(), updated_method));
        }

        Rc::new(ClassDefinition {
            name: class_def.name,
            fields: class_def.fields.clone(),
            methods,
            constructor: class_def.constructor.as_ref().map(|constructor| {
                let mut updated_constructor = constructor.clone();
                updated_constructor.module = Some(module);
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

        for (field_name, (_, default_value)) in &class_ref.fields {
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
        if let Some((visibility, _)) = self.class_ref.fields.get(field_name) {
            match visibility {
                Visibility::Public => true,
                Visibility::Private => !is_external_access,
            }
        } else {
            false
        }
    }

    /// Проверить доступность метода (приватный или публичный доступ)
    pub fn is_method_accessible(&self, method_name: &Symbol, is_external_access: bool) -> bool {
        if let Some((visibility, _)) = self.class_ref.methods.get(method_name) {
            match visibility {
                Visibility::Public => true,
                Visibility::Private => !is_external_access,
            }
        } else {
            false
        }
    }

    /// Получить метод по имени
    pub fn get_method(&self, method_name: &Symbol) -> Option<&FunctionDefinition> {
        self.class_ref
            .methods
            .get(method_name)
            .map(|(_, func)| func)
    }

    /// Получить конструктор класса
    pub fn get_constructor(&self) -> Option<&FunctionDefinition> {
        self.class_ref.constructor.as_ref()
    }
}

impl ClassDefinition {
    /// Создать новый класс
    pub fn new(name: Symbol) -> Self {
        Self {
            name,
            fields: HashMap::new(),
            methods: HashMap::new(),
            constructor: None,
            span: Default::default(),
        }
    }

    /// Добавить поле в класс
    pub fn add_field(
        &mut self,
        name: Symbol,
        visibility: Visibility,
        default_value: Option<ExprId>,
    ) {
        self.fields.insert(name, (visibility, default_value));
    }

    /// Добавить метод в класс
    pub fn add_method(&mut self, name: Symbol, visibility: Visibility, method: FunctionDefinition) {
        self.methods.insert(name, (visibility, method));
    }

    /// Установить конструктор
    pub fn set_constructor(&mut self, constructor: FunctionDefinition) {
        self.constructor = Some(constructor);
    }

    /// Создать экземпляр класса
    pub fn create_instance(self: &Rc<Self>) -> ClassInstance {
        ClassInstance::new(self.name, self.clone())
    }
}
