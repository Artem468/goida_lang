use std::collections::HashMap;
use crate::ast::prelude::{ClassDefinition, ErrorData, Span, Visibility};
use crate::interpreter::prelude::{BuiltinFn, RuntimeError, SharedInterner, Value};
use std::rc::Rc;
use std::sync::Arc;
use crate::ast::program::MethodType;

pub fn setup_array_class(interner: &SharedInterner) -> Rc<ClassDefinition> {
    let mut methods = HashMap::new();
    let name = interner.write().expect("interner lock poisoned").get_or_intern("Массив");

    // len() - Получить длину
    methods.insert(interner.write().expect("interner lock poisoned").get_or_intern("длина"), (Visibility::Public, MethodType::Native(BuiltinFn(Arc::new(|_interp, args, span| {
        if let Some(Value::List(list)) = args.get(0) {
            let length = list.borrow().len();
            Ok(Value::Number(length as i64))
        } else {
            Err(RuntimeError::TypeError(ErrorData::new(span, "Ожидался List".into())))
        }
    })))));

    // join(separator) - Склеить в строку
    methods.insert(interner.write().expect("interner lock poisoned").get_or_intern("объединить"), (Visibility::Public, MethodType::Native(BuiltinFn(Arc::new(|_interp, args, span| {
        if let (Some(Value::List(list)), Some(Value::Text(sep))) = (args.get(0), args.get(1)) {
            let vec = list.borrow();
            let res = vec.iter()
                .map(|v| v.to_string())
                .collect::<Vec<_>>()
                .join(sep);
            Ok(Value::Text(res))
        } else {
            Err(RuntimeError::TypeError(ErrorData::new(span, "Использование: list.join(string)".into())))
        }
    })))));

    // get(index) - Безопасное получение (аналог list[i])
    methods.insert(interner.write().expect("interner lock poisoned").get_or_intern("получить"), (Visibility::Public, MethodType::Native(BuiltinFn(Arc::new(|_interp, args, span| {
        if let (Some(Value::List(list)), Some(Value::Number(idx))) = (args.get(0), args.get(1)) {
            let vec = list.borrow();
            vec.get(*idx as usize).cloned().ok_or_else(|| {
                RuntimeError::InvalidOperation(ErrorData::new(span, "Индекс вне границ".into()))
            })
        } else {
            Err(RuntimeError::TypeError(ErrorData::new(span, "Использование: list.get(number)".into())))
        }
    })))));

    Rc::new(ClassDefinition {
        name,
        fields: HashMap::new(),
        methods,
        constructor: None,
        span: Span::default(),
    })
}