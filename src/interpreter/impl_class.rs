use std::collections::HashMap;
use std::rc::Rc;
use crate::interpreter::structs::{Class, ClassInstance, Value};
use crate::ast::FieldVisibility;

impl ClassInstance {
    /// Создать новый экземпляр класса
    pub fn new(class_name: String, class_ref: Rc<Class>) -> Self {
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
                FieldVisibility::Public => true,   
                FieldVisibility::Private => !is_external_access, 
            }
        } else {
            false 
        }
    }
    
    /// Проверить доступность метода (приватный или публичный доступ)
    pub fn is_method_accessible(&self, method_name: &str, is_external_access: bool) -> bool {
        if let Some((visibility, _)) = self.class_ref.methods.get(method_name) {
            match visibility {
                FieldVisibility::Public => true,   
                FieldVisibility::Private => !is_external_access, 
            }
        } else {
            false 
        }
    }
    
    /// Получить метод по имени
    pub fn get_method(&self, method_name: &str) -> Option<&crate::ast::Function> {
        self.class_ref.methods.get(method_name).map(|(_, func)| func)
    }
    
    /// Получить конструктор класса
    pub fn get_constructor(&self) -> Option<&crate::ast::Function> {
        self.class_ref.constructor.as_ref()
    }
}

impl Class {
    /// Создать новый класс
    pub fn new(name: String) -> Self {
        Self {
            name,
            fields: HashMap::new(),
            methods: HashMap::new(),
            constructor: None,
        }
    }
    
    /// Добавить поле в класс
    pub fn add_field(&mut self, name: String, visibility: FieldVisibility, default_value: Option<Value>) {
        self.fields.insert(name, (visibility, default_value));
    }
    
    /// Добавить метод в класс
    pub fn add_method(&mut self, name: String, visibility: FieldVisibility, method: crate::ast::Function) {
        self.methods.insert(name, (visibility, method));
    }
    
    /// Установить конструктор
    pub fn set_constructor(&mut self, constructor: crate::ast::Function) {
        self.constructor = Some(constructor);
    }
    
    /// Создать экземпляр класса
    pub fn create_instance(self: &Rc<Self>) -> ClassInstance {
        ClassInstance::new(self.name.clone(), self.clone())
    }
}
