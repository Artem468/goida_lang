use std::cell::RefCell;
use std::collections::HashMap;
use crate::ast::prelude::{ClassDefinition, ErrorData, Span, Visibility};
use crate::interpreter::prelude::{BuiltinFn, RuntimeError, SharedInterner, Value};
use std::rc::Rc;
use std::sync::Arc;
use crate::ast::program::MethodType;

pub fn setup_text_class(interner: &SharedInterner) -> Rc<ClassDefinition> {
    let mut methods = HashMap::new();
    let name = interner.write().expect("interner lock poisoned").get_or_intern("Строка");

    let string_constructor = MethodType::Native(BuiltinFn(Arc::new(|_interp, args, _span| {
        if let Some(Value::Object(instance)) = args.get(0) {
            let content = match args.get(1) {
                Some(Value::Text(s)) => s.clone(),
                Some(Value::Number(n)) => n.to_string(),
                Some(Value::Float(f)) => f.to_string(),
                Some(Value::Boolean(b)) => b.to_string(),
                _ => String::new(),
            };

            let data_sym = _interp.interner.write().unwrap().get_or_intern("__data");
            instance.borrow_mut().field_values.insert(data_sym, Value::Text(content));
        }
        Ok(Value::Empty)
    })));

    // len() -> Number
    methods.insert(interner.write().expect("interner lock poisoned").get_or_intern("длина"), (Visibility::Public, MethodType::Native(BuiltinFn(Arc::new(|_interp, args, span| {
        if let Some(Value::Text(s)) = args.get(0) {
            Ok(Value::Number(s.chars().count() as i64))
        } else {
            Err(RuntimeError::TypeError(ErrorData::new(span, "Ожидалась строка".into())))
        }
    })))));

    // split(separator: Text) -> List
    methods.insert(interner.write().expect("interner lock poisoned").get_or_intern("разделить"), (Visibility::Public, MethodType::Native(BuiltinFn(Arc::new(|_interp, args, span| {
        if let (Some(Value::Text(s)), Some(Value::Text(sep))) = (args.get(0), args.get(1)) {
            let parts: Vec<Value> = s.split(sep)
                .map(|part| Value::Text(part.to_string()))
                .collect();
            Ok(Value::List(Rc::new(RefCell::new(parts))))
        } else {
            Err(RuntimeError::TypeError(ErrorData::new(span, "Использование: str.split(separator)".into())))
        }
    })))));

    // upper() -> Text
    methods.insert(interner.write().expect("interner lock poisoned").get_or_intern("верхний"), (Visibility::Public, MethodType::Native(BuiltinFn(Arc::new(|_interp, args, span| {
        if let Some(Value::Text(s)) = args.get(0) {
            Ok(Value::Text(s.to_uppercase()))
        } else {
            Err(RuntimeError::TypeError(ErrorData::new(span, "Ожидалась строка".into())))
        }
    })))));

    // lower() -> Text
    methods.insert(interner.write().expect("interner lock poisoned").get_or_intern("нижний"), (Visibility::Public, MethodType::Native(BuiltinFn(Arc::new(|_interp, args, span| {
        if let Some(Value::Text(s)) = args.get(0) {
            Ok(Value::Text(s.to_lowercase()))
        } else {
            Err(RuntimeError::TypeError(ErrorData::new(span, "Ожидалась строка".into())))
        }
    })))));

    // contains(substring: Text) -> Boolean
    methods.insert(interner.write().expect("interner lock poisoned").get_or_intern("содержит"), (Visibility::Public, MethodType::Native(BuiltinFn(Arc::new(|_interp, args, span| {
        if let (Some(Value::Text(s)), Some(Value::Text(sub))) = (args.get(0), args.get(1)) {
            Ok(Value::Boolean(s.contains(sub)))
        } else {
            Err(RuntimeError::TypeError(ErrorData::new(span, "Использование: str.contains(substring)".into())))
        }
    })))));

    // replace(old: Text, new: Text) -> Text
    methods.insert(interner.write().expect("interner lock poisoned").get_or_intern("заменить"), (Visibility::Public, MethodType::Native(BuiltinFn(Arc::new(|_interp, args, span| {
        if let (Some(Value::Text(s)), Some(Value::Text(old)), Some(Value::Text(new))) = (args.get(0), args.get(1), args.get(2)) {
            Ok(Value::Text(s.replace(old, new)))
        } else {
            Err(RuntimeError::TypeError(ErrorData::new(span, "Использование: str.replace(old, new)".into())))
        }
    })))));

    Rc::new(ClassDefinition {
        name,
        fields: HashMap::new(),
        methods,
        constructor: Some(string_constructor),
        span: Span::default(),
    })
}