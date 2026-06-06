use crate::ast::prelude::{ClassDefinition, ErrorData, Span};
use crate::builtins::iterator::values_from_iterable;
use crate::builtins::registry::*;
use crate::interpreter::prelude::{
    CallArgListExt, Interpreter, RuntimeError, RuntimeIterator, SharedInterner, Value,
};
use crate::shared::SharedMut;
use crate::{
    bail_runtime, define_builtin, define_constructor, define_method, expect_args, runtime_error,
};
use std::ffi::{c_char, CStr};
use string_interner::DefaultSymbol as Symbol;

pub fn setup_text_class(interner: &SharedInterner) -> (Symbol, SharedMut<ClassDefinition>) {
    let name = interner.write(|i| i.get_or_intern(class::STRING.names.canonical));

    let mut class_def = ClassDefinition::new(name, Span::default());

    define_constructor!(class_def, (interp, args, _) {
        if let Some(Value::Object(instance)) = CallArgListExt::first_value(&args) {
            let content = match CallArgListExt::get_value(&args, 1) {
                Some(Value::Text(s)) => s.clone(),
                Some(Value::Number(n)) => n.to_string(),
                Some(Value::Float(f)) => f.to_string(),
                Some(Value::Boolean(b)) => b.to_string(),
                _ => String::new(),
            };

            let data_sym = interp.interner.write(|i| i.get_or_intern("__data"));
            instance.write(|i| i.field_values.insert(data_sym, Value::Text(content)));
        }
        Ok(Value::Empty)
    });

    // len() -> Number
    define_method!(class_def, interner, method::LEN.canonical => (_interp, args, span) {
        if let Some(Value::Text(s)) = CallArgListExt::first_value(&args) {
            Ok(Value::Number(s.chars().count() as i64))
        } else {
            bail_runtime!(
                TypeError,
                span,
                "Ожидалась строка"
            )
        }
    });

    // split(separator: Text) -> List
    define_method!(class_def, interner, method::SPLIT.canonical => (_interp, args, span) {
        if let (Some(Value::Text(s)), Some(Value::Text(sep))) = (
            CallArgListExt::first_value(&args),
            CallArgListExt::get_value(&args, 1),
        ) {
            let parts: Vec<Value> = s
                .split(sep)
                .map(|part| Value::Text(part.to_string()))
                .collect();
            Ok(Value::List(SharedMut::new(parts)))
        } else {
            bail_runtime!(
                TypeError,
                span,
                "Использование: str.split(separator)"
            )
        }
    });

    // upper() -> Text
    define_method!(class_def, interner, method::UPPER.canonical => (_interp, args, span) {
        if let Some(Value::Text(s)) = CallArgListExt::first_value(&args) {
            Ok(Value::Text(s.to_uppercase()))
        } else {
            bail_runtime!(
                TypeError,
                span,
                "Ожидалась строка"
            )
        }
    });

    // lower() -> Text
    define_method!(class_def, interner, method::LOWER.canonical => (_interp, args, span) {
        if let Some(Value::Text(s)) = CallArgListExt::first_value(&args) {
            Ok(Value::Text(s.to_lowercase()))
        } else {
            bail_runtime!(
                TypeError,
                span,
                "Ожидалась строка"
            )
        }
    });

    // contains(substring: Text) -> Boolean
    define_method!(class_def, interner, method::CONTAINS.canonical => (_interp, args, span) {
        if let (Some(Value::Text(s)), Some(Value::Text(sub))) = (
            CallArgListExt::first_value(&args),
            CallArgListExt::get_value(&args, 1),
        ) {
            Ok(Value::Boolean(s.contains(sub)))
        } else {
            bail_runtime!(
                TypeError,
                span,
                "Использование: str.contains(substring)"
            )
        }
    });

    // replace(old: Text, new: Text) -> Text
    define_method!(class_def, interner, method::REPLACE.canonical => (_interp, args, span) {
        if let (Some(Value::Text(s)), Some(Value::Text(old)), Some(Value::Text(new))) = (
            CallArgListExt::first_value(&args),
            CallArgListExt::get_value(&args, 1),
            CallArgListExt::get_value(&args, 2),
        ) {
            Ok(Value::Text(s.replace(old, new)))
        } else {
            bail_runtime!(
                TypeError,
                span,
                "Использование: str.replace(old, new)"
            )
        }
    });

    define_method!(class_def, interner, method::TRIM.canonical => (_interp, args, span) {
        if let Some(Value::Text(s)) = CallArgListExt::first_value(&args) {
            Ok(Value::Text(s.trim().to_string()))
        } else {
            bail_runtime!(TypeError, span, "Ожидалась строка")
        }
    });

    define_method!(class_def, interner, method::STARTS_WITH.canonical => (_interp, args, span) {
        if let (Some(Value::Text(s)), Some(Value::Text(prefix))) = (
            CallArgListExt::first_value(&args),
            CallArgListExt::get_value(&args, 1),
        ) {
            Ok(Value::Boolean(s.starts_with(prefix)))
        } else {
            bail_runtime!(TypeError, span, "Использование: str.начинается_с(prefix)")
        }
    });

    define_method!(class_def, interner, method::ENDS_WITH.canonical => (_interp, args, span) {
        if let (Some(Value::Text(s)), Some(Value::Text(suffix))) = (
            CallArgListExt::first_value(&args),
            CallArgListExt::get_value(&args, 1),
        ) {
            Ok(Value::Boolean(s.ends_with(suffix)))
        } else {
            bail_runtime!(TypeError, span, "Использование: str.заканчивается_на(suffix)")
        }
    });

    define_method!(class_def, interner, method::ITERATOR.canonical => (_, args, span) {
        let Some(value) = CallArgListExt::first_value(&args) else {
            return bail_runtime!(TypeError, span, "Ожидалась строка");
        };
        Ok(Value::Iterator(RuntimeIterator::new(values_from_iterable(value, span)?)))
    });

    (name, SharedMut::new(class_def))
}

pub fn setup_text_func(interpreter: &mut Interpreter, interner: &SharedInterner) {
    define_builtin!(interpreter, interner, function::STRING.canonical => (_, arguments, span) {
        expect_args!(arguments, 1, span, "строка");
        let n: String = arguments[0].value.clone().try_into()?;
        Ok(Value::Text(n))
    });

    define_builtin!(interpreter, interner, function::STRING_FROM_POINTER.canonical => (_, arguments, _){
        let ptr = arguments[0].value.as_i64().unwrap();
        let _v = unsafe { addr_to_string(ptr) };
        Ok(Value::Text(_v))
    });
}

pub unsafe fn addr_to_string(addr: i64) -> String {
    let ptr = addr as *const c_char;

    if ptr.is_null() {
        return String::from("");
    }

    let c_str = CStr::from_ptr(ptr);
    c_str.to_string_lossy().into_owned()
}
