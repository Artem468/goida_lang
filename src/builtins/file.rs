use crate::ast::prelude::{ClassDefinition, ErrorData, Span, Visibility};
use crate::ast::program::MethodType;
use crate::interpreter::prelude::{BuiltinFn, RuntimeError, SharedInterner, Value};
use std::collections::HashMap;
use std::fs;
use std::io::Write;
use std::rc::Rc;
use std::sync::Arc;

pub fn setup_file_class(interner_ref: &SharedInterner) -> Rc<ClassDefinition> {
    let mut methods = HashMap::new();
    let name = interner_ref.write().unwrap().get_or_intern("Файл");

    let file_constructor = MethodType::Native(BuiltinFn(Arc::new(|interp, args, span| {
        if let (Some(Value::Object(instance)), Some(Value::Text(path))) = (args.get(0), args.get(1))
        {
            let path_sym = interp.interner.write().unwrap().get_or_intern("путь");
            instance
                .borrow_mut()
                .field_values
                .insert(path_sym, Value::Text(path.clone()));
            Ok(Value::Empty)
        } else {
            Err(RuntimeError::TypeError(ErrorData::new(
                span,
                "Использование: новый Файл(путь)".into(),
            )))
        }
    })));

    let get_path = |args: &Vec<Value>| -> Option<String> {
        if let Some(Value::Object(instance)) = args.get(0) {
            for (_, val) in &instance.borrow().field_values {
                if let Value::Text(p) = val {
                    return Some(p.clone());
                }
            }
        }
        None
    };

    // --- .существует() -> Bool ---
    methods.insert(
        interner_ref.write().unwrap().get_or_intern("существует"),
        (
            Visibility::Public,
            MethodType::Native(BuiltinFn(Arc::new(move |_, args, _| {
                let path = get_path(&args).unwrap_or_default();
                Ok(Value::Boolean(std::path::Path::new(&path).exists()))
            }))),
        ),
    );

    // --- .читать() -> Text ---
    methods.insert(
        interner_ref.write().unwrap().get_or_intern("читать"),
        (
            Visibility::Public,
            MethodType::Native(BuiltinFn(Arc::new(move |_, args, span| {
                let path = get_path(&args).ok_or(RuntimeError::IOError(ErrorData::new(
                    span,
                    "Путь не найден".into(),
                )))?;
                let content = fs::read_to_string(path)
                    .map_err(|e| RuntimeError::IOError(ErrorData::new(span, e.to_string())))?;
                Ok(Value::Text(content))
            }))),
        ),
    );

    // --- .записать(текст) ---
    methods.insert(
        interner_ref.write().unwrap().get_or_intern("записать"),
        (
            Visibility::Public,
            MethodType::Native(BuiltinFn(Arc::new(move |_, args, span| {
                let path = get_path(&args).ok_or(RuntimeError::IOError(ErrorData::new(
                    span,
                    "Путь не найден".into(),
                )))?;
                let text = if let Some(t) = args.get(1) {
                    t.to_string()
                } else {
                    "".into()
                };

                if let Some(parent) = std::path::Path::new(&path).parent() {
                    fs::create_dir_all(parent)
                        .map_err(|e| RuntimeError::IOError(ErrorData::new(span, e.to_string())))?;
                }

                fs::write(path, text)
                    .map_err(|e| RuntimeError::IOError(ErrorData::new(span, e.to_string())))?;
                Ok(Value::Empty)
            }))),
        ),
    );

    // --- .дописать(текст) ---
    methods.insert(
        interner_ref.write().unwrap().get_or_intern("дописать"),
        (
            Visibility::Public,
            MethodType::Native(BuiltinFn(Arc::new(move |_, args, span| {
                let path = get_path(&args).ok_or(RuntimeError::IOError(ErrorData::new(
                    span,
                    "Путь не найден".into(),
                )))?;
                let text = if let Some(t) = args.get(1) {
                    t.to_string()
                } else {
                    "".into()
                };

                if let Some(parent) = std::path::Path::new(&path).parent() {
                    fs::create_dir_all(parent)
                        .map_err(|e| RuntimeError::IOError(ErrorData::new(span, e.to_string())))?;
                }

                let mut file = fs::OpenOptions::new()
                    .append(true)
                    .create(true)
                    .open(path)
                    .map_err(|e| RuntimeError::IOError(ErrorData::new(span, e.to_string())))?;
                file.write_all(text.as_bytes())
                    .map_err(|e| RuntimeError::IOError(ErrorData::new(span, e.to_string())))?;
                Ok(Value::Empty)
            }))),
        ),
    );

    // --- .удалить() ---
    methods.insert(
        interner_ref.write().unwrap().get_or_intern("удалить"),
        (
            Visibility::Public,
            MethodType::Native(BuiltinFn(Arc::new(move |_, args, span| {
                let path = get_path(&args).ok_or(RuntimeError::IOError(ErrorData::new(
                    span,
                    "Путь не найден".into(),
                )))?;
                fs::remove_file(path)
                    .map_err(|e| RuntimeError::IOError(ErrorData::new(span, e.to_string())))?;
                Ok(Value::Empty)
            }))),
        ),
    );

    Rc::new(ClassDefinition {
        name,
        fields: HashMap::new(),
        methods,
        constructor: Some(file_constructor),
        span: Span::default(),
    })
}
