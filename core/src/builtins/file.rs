use crate::ast::prelude::{ClassDefinition, ErrorData, Span};
use crate::interpreter::prelude::{
    CallArgListExt, CallArgValue, RuntimeError, SharedInterner, Value,
};
use crate::shared::SharedMut;
use crate::{bail_runtime, define_constructor, define_method, runtime_error};
use std::fs;
use std::path::Path;
use string_interner::DefaultSymbol as Symbol;

pub fn setup_file_class(interner_ref: &SharedInterner) -> (Symbol, SharedMut<ClassDefinition>) {
    let name = interner_ref.write(|i| i.get_or_intern("Файл"));

    let mut class_def = ClassDefinition::new(name, Span::default());

    define_constructor!(class_def, (interp, args, span) {
        if let (Some(Value::Object(instance)), Some(Value::Text(path))) = (
            CallArgListExt::first_value(&args),
            CallArgListExt::get_value(&args, 1),
        ) {
            let path_sym = interp.interner.write(|i| i.get_or_intern("путь"));
            instance.write(|i| i.field_values.insert(path_sym, Value::Text(path.clone())));
            Ok(Value::Empty)
        } else {
            bail_runtime!(
                TypeError,
                span,
                "Использование: новый Файл(путь)"
            )
        }
    });

    let get_path = |args: &Vec<CallArgValue>| -> Result<String, RuntimeError> {
        if let Some(Value::Object(instance)) = CallArgListExt::first_value(args) {
            return instance.read(|i| {
                for val in i.field_values.values() {
                    if let Value::Text(p) = val {
                        return Ok(p.clone());
                    }
                }
                bail_runtime!(InvalidOperation, Span::default(), "Путь не найден")
            });
        }
        bail_runtime!(InvalidOperation, Span::default(), "Путь не найден")
    };

    // --- .существует() -> Bool ---
    define_method!(class_def, interner_ref, "существует" => (_, args, _) {
        let path = get_path(&args).unwrap_or_default();
        Ok(Value::Boolean(Path::new(&path).exists()))
    });

    // --- .читать() -> Text ---
    define_method!(class_def, interner_ref, "читать" => (_, args, span) {
        let path = get_path(&args)?;
        let content = fs::read_to_string(path)
            .map_err(|e| runtime_error!(IOError, span, "{}", e.to_string()))?;
        Ok(Value::Text(content))
    });

    // --- .записать(текст) ---
    define_method!(class_def, interner_ref, "записать" => (_, args, span) {
        let path = get_path(&args)?;
        let text = if let Some(t) = CallArgListExt::get_value(&args, 1) {
            t.to_string()
        } else {
            "".into()
        };

        if let Some(parent) = Path::new(&path).parent() {
            fs::create_dir_all(parent)
                .map_err(|e| runtime_error!(IOError, span, "{}", e.to_string()))?;
        }

        fs::write(path, text)
            .map_err(|e| runtime_error!(IOError, span, "{}", e.to_string()))?;
        Ok(Value::Empty)
    });

    // --- .дописать(текст) ---
    define_method!(class_def, interner_ref, "дописать" => (_, args, span) {
        let path = get_path(&args)?;
        let text = if let Some(t) = CallArgListExt::get_value(&args, 1) {
            t.to_string()
        } else {
            "".into()
        };

        if let Some(parent) = Path::new(&path).parent() {
            fs::create_dir_all(parent)
                .map_err(|e| runtime_error!(IOError, span, "{}", e.to_string()))?;
        }

        let mut file = fs::OpenOptions::new()
            .append(true)
            .create(true)
            .open(path)
            .map_err(|e| runtime_error!(IOError, span, "{}", e.to_string()))?;

        use std::io::Write;
        file.write_all(text.as_bytes())
            .map_err(|e| runtime_error!(IOError, span, "{}", e.to_string()))?;

        Ok(Value::Empty)
    });

    // --- .удалить() ---
    define_method!(class_def, interner_ref, "удалить" => (_, args, span) {
        let path = get_path(&args)?;
        fs::remove_file(path)
            .map_err(|e| runtime_error!(IOError, span, "{}", e.to_string()))?;
        Ok(Value::Empty)
    });

    (name, SharedMut::new(class_def))
}
