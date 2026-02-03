use std::cell::RefCell;
use std::collections::HashMap;
use crate::ast::prelude::{ClassDefinition, ErrorData, Span, Visibility};
use crate::interpreter::prelude::{BuiltinFn, RuntimeError, SharedInterner, Value};
use std::rc::Rc;
use std::sync::Arc;
use crate::ast::program::MethodType;
use crate::traits::prelude::CoreOperations;

pub fn setup_list_class(interner: &SharedInterner) -> Rc<ClassDefinition> {
    let mut methods = HashMap::new();
    let name = interner.write().expect("interner lock poisoned").get_or_intern("Список");

    let list_constructor = MethodType::Native(BuiltinFn(Arc::new(|_interp, args, _span| {
        // args[0] - это временный ClassInstance (this)
        // args[1..] - это элементы: новый Список(1, 2, 3)
        if let Some(Value::Object(instance)) = args.get(0) {
            let items = args[1..].to_vec();
            let internal_list = Value::List(Rc::new(RefCell::new(items)));

            let data_sym = _interp.intern_string("__data");
            instance.borrow_mut().field_values.insert(data_sym, internal_list);
        }
        Ok(Value::Empty)
    })));

    // append(value) - Добавить в конец
    methods.insert(interner.write().expect("interner lock poisoned").get_or_intern("добавить"), (Visibility::Public, MethodType::Native(BuiltinFn(Arc::new(|_interp, args, span| {
        if let (Some(Value::List(list)), Some(val)) = (args.get(0), args.get(1)) {
            list.borrow_mut().push(val.clone());
            Ok(Value::Empty)
        } else {
            Err(RuntimeError::TypeError(ErrorData::new(span, "Использование: list.append(value)".into())))
        }
    })))));

    // set(index: Number, value: Any) -> Empty
    methods.insert(interner.write().expect("interner lock poisoned").get_or_intern("задать"), (Visibility::Public, MethodType::Native(BuiltinFn(Arc::new(|_interp, args, span| {
        if let (Some(Value::List(list)), Some(Value::Number(idx)), Some(new_val)) = (args.get(0), args.get(1), args.get(2)) {
            let mut vec = list.borrow_mut();
            let i = *idx as usize;
            if i < vec.len() {
                vec[i] = new_val.clone();
                Ok(Value::Empty)
            } else {
                Err(RuntimeError::InvalidOperation(ErrorData::new(span, "Индекс вне границ списка".into())))
            }
        } else {
            Err(RuntimeError::TypeError(ErrorData::new(span, "Использование: list.set(number, value)".into())))
        }
    })))));

    // len() - Получить длину
    methods.insert(interner.write().expect("interner lock poisoned").get_or_intern("длина"), (Visibility::Public, MethodType::Native(BuiltinFn(Arc::new(|_interp, args, span| {
        if let Some(Value::List(list)) = args.get(0) {
            let length = list.borrow().len();
            Ok(Value::Number(length as i64))
        } else {
            Err(RuntimeError::TypeError(ErrorData::new(span, "Ожидался List".into())))
        }
    })))));

    // pop(index?) - Удалить и вернуть элемент (последний или по индексу)
    methods.insert(interner.write().expect("interner lock poisoned").get_or_intern("удалить"), (Visibility::Public, MethodType::Native(BuiltinFn(Arc::new(|_interp, args, span| {
        if let Some(Value::List(list)) = args.get(0) {
            let mut vec = list.borrow_mut();
            if vec.is_empty() {
                return Err(RuntimeError::InvalidOperation(ErrorData::new(span, "pop у пустого списка".into())));
            }
            let val = if let Some(Value::Number(idx)) = args.get(1) {
                if *idx < 0 || *idx >= vec.len() as i64 {
                    return Err(RuntimeError::InvalidOperation(ErrorData::new(span, "Индекс вне границ".into())));
                }
                vec.remove(*idx as usize)
            } else {
                vec.pop().unwrap()
            };
            Ok(val)
        } else {
            Err(RuntimeError::TypeError(ErrorData::new(span, "Ожидался List".into())))
        }
    })))));
    // clear() - Очистить список
    methods.insert(interner.write().expect("interner lock poisoned").get_or_intern("отчистить"), (Visibility::Public, MethodType::Native(BuiltinFn(Arc::new(|_interp, args, span| {
        if let Some(Value::List(list)) = args.get(0) {
            list.borrow_mut().clear();
            Ok(Value::Empty)
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
        constructor: Some(list_constructor),
        span: Span::default(),
    })
}