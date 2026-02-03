use crate::ast::prelude::{ClassDefinition, ErrorData, Span, Visibility};
use crate::ast::program::MethodType;
use crate::interpreter::prelude::{BuiltinFn, RuntimeError, SharedInterner, Value};
use std::fs;
use std::io::Write;
use std::sync::{Arc, RwLock};
use string_interner::DefaultSymbol as Symbol;

pub fn setup_file_class(interner_ref: &SharedInterner) -> (Symbol, Arc<RwLock<ClassDefinition>>) {
    let name = interner_ref.write().unwrap().get_or_intern("Файл");

    let mut class_def = ClassDefinition::new(name, Span::default());

    class_def.set_constructor(BuiltinFn(Arc::new(|interp, args, span| {
        if let (Some(Value::Object(instance)), Some(Value::Text(path))) = (args.get(0), args.get(1))
        {
            let path_sym = interp.interner.write().unwrap().get_or_intern("путь");
            instance
                .write()
                .map_err(|_| {
                    RuntimeError::Panic(ErrorData::new(
                        Span::default(),
                        "Сбой блокировки в реализации файла".into(),
                    ))
                })?
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

    let get_path = |args: &Vec<Value>| -> Result<String, RuntimeError> {
        if let Some(Value::Object(instance)) = args.get(0) {
            for (_, val) in &instance
                .read()
                .map_err(|_| {
                    RuntimeError::Panic(ErrorData::new(
                        Span::default(),
                        "Сбой блокировки в реализации файла".into(),
                    ))
                })?
                .field_values
            {
                if let Value::Text(p) = val {
                    return Ok(p.clone());
                }
            }
        }
        Err(RuntimeError::InvalidOperation(ErrorData::new(Span::default(), "Путь не найден".into())))
    };

    // --- .существует() -> Bool ---
    class_def.add_method(
        interner_ref.write().unwrap().get_or_intern("существует"),
        Visibility::Public,
        false,
        BuiltinFn(Arc::new(move |_, args, _| {
            let path = get_path(&args).unwrap_or_default();
            Ok(Value::Boolean(std::path::Path::new(&path).exists()))
        })),
    );

    // --- .читать() -> Text ---
    class_def.add_method(
        interner_ref.write().unwrap().get_or_intern("читать"),
        Visibility::Public,
        false,
        BuiltinFn(Arc::new(move |_, args, span| {
            let path = get_path(&args)?;
            let content = fs::read_to_string(path)
                .map_err(|e| RuntimeError::IOError(ErrorData::new(span, e.to_string())))?;
            Ok(Value::Text(content))
        })),
    );

    // --- .записать(текст) ---
    class_def.add_method(
        interner_ref.write().unwrap().get_or_intern("записать"),
        Visibility::Public,
        false,
        BuiltinFn(Arc::new(move |_, args, span| {
            let path = get_path(&args)?;
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
        })),
    );

    // --- .дописать(текст) ---
    class_def.add_method(
        interner_ref.write().unwrap().get_or_intern("дописать"),
        Visibility::Public,
        false,
        MethodType::Native(Arc::from(BuiltinFn(Arc::new(move |_, args, span| {
            let path = get_path(&args)?;
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
        })))),
    );

    // --- .удалить() ---
    class_def.add_method(
        interner_ref.write().unwrap().get_or_intern("удалить"),
        Visibility::Public,
        false,
        BuiltinFn(Arc::new(move |_, args, span| {
            let path = get_path(&args)?;
            fs::remove_file(path)
                .map_err(|e| RuntimeError::IOError(ErrorData::new(span, e.to_string())))?;
            Ok(Value::Empty)
        })),
    );

    (name, Arc::new(RwLock::new(class_def)))
}
