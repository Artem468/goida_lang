use crate::ast::prelude::{ErrorData, Span};
use crate::builtins::iterator::values_from_iterable;
use crate::builtins::registry::*;
use crate::interpreter::prelude::RuntimeClassDefinition;
use crate::interpreter::prelude::{
    CallArgListExt, Interpreter, RuntimeError, RuntimeIterator, SharedInterner, Value,
};
use crate::shared::SharedMut;
use crate::{
    bail_runtime, define_builtin, define_constructor, define_method, expect_args, runtime_error,
};
use std::ffi::CStr;
use std::os::raw::c_char;
use string_interner::DefaultSymbol as Symbol;

const MAX_NATIVE_STRING_BYTES: usize = 16 * 1024 * 1024;

pub fn setup_text_class(interner: &SharedInterner) -> (Symbol, SharedMut<RuntimeClassDefinition>) {
    let name = interner.write(|i| i.get_or_intern(class::STRING.names.canonical));

    let mut class_def = RuntimeClassDefinition::new(name, Span::default());

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
        expect_args!(arguments, 1, span, function::STRING.canonical);
        let n: String = arguments[0].value.clone().try_into()?;
        Ok(Value::Text(n))
    });

    define_builtin!(interpreter, interner, function::STRING_FROM_POINTER.canonical => (_, arguments, span){
        match arguments.as_slice() {
            [pointer] => {
                let address = native_pointer_address(&pointer.value, span)?;
                copy_utf8_from_c_string(address, span)
            }
            [pointer, byte_length] => {
                let address = native_pointer_address(&pointer.value, span)?;
                let Value::Number(byte_length) = byte_length.value else {
                    return bail_runtime!(TypeError, span, "string_from_pointer ожидает длину байт");
                };
                copy_utf8_from_pointer(address, byte_length, span)
            }
            _ => bail_runtime!(
                InvalidOperation,
                span,
                "string_from_pointer ожидает 1 или 2 аргумента, получено {}",
                arguments.len()
            ),
        }
    });
}

fn native_pointer_address(value: &Value, span: Span) -> Result<usize, RuntimeError> {
    match value {
        Value::Pointer(address) => Ok(*address),
        Value::Empty => Ok(0),
        _ => bail_runtime!(
            TypeError,
            span,
            "string_from_pointer ожидает нативный указатель"
        ),
    }
}

fn copy_utf8_from_c_string(address: usize, span: Span) -> Result<Value, RuntimeError> {
    if address == 0 {
        return Ok(Value::Text(String::new()));
    }

    // SAFETY: the trusted native library must return a readable NUL-terminated
    // string that remains alive for the duration of this call.
    let bytes = unsafe { CStr::from_ptr(address as *const c_char) }.to_bytes();
    if bytes.len() > MAX_NATIVE_STRING_BYTES {
        return bail_runtime!(
            InvalidOperation,
            span,
            "Native string exceeds the {} byte limit",
            MAX_NATIVE_STRING_BYTES
        );
    }
    copy_utf8_bytes(bytes, span)
}

fn copy_utf8_from_pointer(
    address: usize,
    byte_length: i64,
    span: Span,
) -> Result<Value, RuntimeError> {
    if byte_length < 0 {
        return bail_runtime!(
            InvalidOperation,
            span,
            "Native string length cannot be negative"
        );
    }

    let byte_length = byte_length as usize;
    if byte_length > MAX_NATIVE_STRING_BYTES {
        return bail_runtime!(
            InvalidOperation,
            span,
            "Native string exceeds the {} byte limit",
            MAX_NATIVE_STRING_BYTES
        );
    }
    if byte_length == 0 {
        return Ok(Value::Text(String::new()));
    }
    if address == 0 {
        return bail_runtime!(InvalidOperation, span, "Native string pointer is null");
    }

    if address.checked_add(byte_length).is_none() {
        return bail_runtime!(
            InvalidOperation,
            span,
            "Native string address range overflows"
        );
    }

    // SAFETY: the native library contract must guarantee that this range remains
    // readable for the duration of the call. The bytes are copied immediately.
    let bytes = unsafe { std::slice::from_raw_parts(address as *const u8, byte_length) };
    copy_utf8_bytes(bytes, span)
}

fn copy_utf8_bytes(bytes: &[u8], span: Span) -> Result<Value, RuntimeError> {
    let text = std::str::from_utf8(bytes).map_err(|err| {
        runtime_error!(TypeError, span, "Native string is not valid UTF-8: {err}")
    })?;
    Ok(Value::Text(text.to_owned()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::interpreter::prelude::{CallArgValue, Interpreter};
    use crate::traits::runtime::CoreOperations;

    #[test]
    fn builtin_rejects_numeric_address_without_dereferencing_it() {
        let interner = goida_model::new_interner();
        let mut interpreter = Interpreter::new(interner.clone());
        setup_text_func(&mut interpreter, &interner);
        let symbol = interner.write(|i| i.get_or_intern("string_from_pointer"));
        let builtin = interpreter
            .builtins
            .get(&symbol)
            .expect("installed builtin");

        let result = (builtin.0)(
            &interpreter,
            vec![
                CallArgValue {
                    name: None,
                    value: Value::Number(1),
                },
                CallArgValue {
                    name: None,
                    value: Value::Number(1),
                },
            ],
            Span::default(),
        );

        assert!(matches!(result, Err(RuntimeError::TypeError(_))));
    }

    #[test]
    fn copies_utf8_from_pointer_with_explicit_length() {
        let text = "native text";
        let address = text.as_ptr() as usize;

        assert!(matches!(
            copy_utf8_from_pointer(address, text.len() as i64, Span::default()),
            Ok(Value::Text(value)) if value == text
        ));
    }

    #[test]
    fn copies_utf8_from_nul_terminated_pointer() {
        let text = b"native c string\0";
        let address = text.as_ptr() as usize;

        assert!(matches!(
            copy_utf8_from_c_string(address, Span::default()),
            Ok(Value::Text(value)) if value == "native c string"
        ));
    }

    #[test]
    fn rejects_invalid_native_string_ranges() {
        assert!(matches!(
            copy_utf8_from_pointer(0, 0, Span::default()),
            Ok(Value::Text(value)) if value.is_empty()
        ));
        assert!(matches!(
            copy_utf8_from_pointer(0, 1, Span::default()),
            Err(RuntimeError::InvalidOperation(_))
        ));
        assert!(matches!(
            copy_utf8_from_pointer(1, -1, Span::default()),
            Err(RuntimeError::InvalidOperation(_))
        ));
        assert!(matches!(
            copy_utf8_from_pointer(1, MAX_NATIVE_STRING_BYTES as i64 + 1, Span::default()),
            Err(RuntimeError::InvalidOperation(_))
        ));
    }

    #[test]
    fn rejects_non_utf8_native_string() {
        let bytes = [0xff];
        let address = bytes.as_ptr() as usize;

        assert!(matches!(
            copy_utf8_from_pointer(address, bytes.len() as i64, Span::default()),
            Err(RuntimeError::TypeError(_))
        ));
    }
}
