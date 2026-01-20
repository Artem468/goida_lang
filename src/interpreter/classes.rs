use crate::ast::prelude::{ClassDefinition, FunctionDefinition, Program, Visibility};
use crate::interpreter::structs::{
    Class, ClassInstance, Environment, Interpreter, RuntimeError, Value,
};
use crate::interpreter::traits::{ExpressionEvaluator, InterpreterClasses, StatementExecutor};
use std::collections::HashMap;
use std::rc::Rc;
use string_interner::DefaultSymbol as Symbol;

impl InterpreterClasses for Interpreter {
    /// Регистрируем класс в интерпретаторе
    fn register_class(
        &mut self,
        class_def: &ClassDefinition,
        program: &Program,
    ) -> Result<(), RuntimeError> {
        let class_name = program
            .arena
            .resolve_symbol(class_def.name)
            .unwrap()
            .to_string();
        let mut class = Class::new(class_def.name);

        for field in &class_def.fields {
            let field_name = program
                .arena
                .resolve_symbol(field.name)
                .unwrap()
                .to_string();
            let default_value = if let Some(default_expr) = field.default_value {
                Some(self.evaluate_expression(default_expr, program)?)
            } else {
                None
            };
            class.add_field(field_name, field.visibility.clone(), default_value);
        }

        for method in &class_def.methods {
            let method_name = program
                .arena
                .resolve_symbol(method.name)
                .unwrap()
                .to_string();

            let function = FunctionDefinition {
                name: method.name,
                params: method.params.clone(),
                return_type: method.return_type,
                body: method.body,
                span: method.span,
                module: Some(program.name),
            };

            if method.is_constructor {
                class.set_constructor(function);
            } else {
                class.add_method(method_name, method.visibility.clone(), function);
            }
        }

        self.classes.insert(class_name, Rc::new(class));
        Ok(())
    }

    /// Вызываем метод с контекстом объекта
    fn call_method(
        &mut self,
        method: FunctionDefinition,
        arguments: Vec<Value>,
        this_obj: Value,
        program: &Program,
    ) -> Result<Value, RuntimeError> {
        let prev_module = self.current_module.clone();
        let module_name = method.module.map(|module_symbol| {
            program
                .arena
                .resolve_symbol(module_symbol)
                .unwrap()
                .to_string()
        });
        self.current_module = module_name;

        let parent_env = self.environment.clone();
        self.environment = Environment::with_parent(parent_env);

        self.environment.define("this".to_string(), this_obj);

        if arguments.len() != method.params.len() {
            return Err(RuntimeError::InvalidOperation(format!(
                "Метод {} ожидает {} аргументов, получено {}",
                program.arena.resolve_symbol(method.name).unwrap(),
                method.params.len(),
                arguments.len()
            )));
        }

        for (param, arg_value) in method.params.iter().zip(arguments.iter()) {
            let param_name = program
                .arena
                .resolve_symbol(param.name)
                .unwrap()
                .to_string();
            self.environment.define(param_name, arg_value.clone());
        }

        let mut result = Value::Empty;
        match self.execute_statement(method.body, program) {
            Ok(()) => {}
            Err(RuntimeError::Return(val)) => {
                result = val;
            }
            Err(e) => {
                self.environment = self.environment.clone().pop();
                self.current_module = prev_module;
                return Err(e);
            }
        }

        self.environment = self.environment.clone().pop();
        self.current_module = prev_module;
        Ok(result)
    }
}

impl ClassInstance {
    /// Создать новый экземпляр класса
    pub fn new(class_name: Symbol, class_ref: Rc<Class>) -> Self {
        let mut fields = HashMap::new();

        for (field_name, (_, default_value)) in &class_ref.fields {
            if let Some(default) = default_value {
                fields.insert(field_name.clone(), default.clone());
            } else {
                fields.insert(field_name.clone(), Value::Empty);
            }
        }

        Self {
            class_name,
            fields,
            class_ref,
        }
    }

    /// Получить значение поля
    pub fn get_field(&self, field_name: &str) -> Option<&Value> {
        self.fields.get(field_name)
    }

    /// Установить значение поля
    pub fn set_field(&mut self, field_name: String, value: Value) {
        self.fields.insert(field_name, value);
    }

    /// Проверить доступность поля (приватный или публичный доступ)
    pub fn is_field_accessible(&self, field_name: &str, is_external_access: bool) -> bool {
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
    pub fn is_method_accessible(&self, method_name: &str, is_external_access: bool) -> bool {
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
    pub fn get_method(&self, method_name: &str) -> Option<&FunctionDefinition> {
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

impl Class {
    /// Создать новый класс
    pub fn new(name: Symbol) -> Self {
        Self {
            name,
            fields: HashMap::new(),
            methods: HashMap::new(),
            constructor: None,
        }
    }

    /// Добавить поле в класс
    pub fn add_field(
        &mut self,
        name: String,
        visibility: Visibility,
        default_value: Option<Value>,
    ) {
        self.fields.insert(name, (visibility, default_value));
    }

    /// Добавить метод в класс
    pub fn add_method(&mut self, name: String, visibility: Visibility, method: FunctionDefinition) {
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
