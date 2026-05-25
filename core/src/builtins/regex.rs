use crate::ast::prelude::{ClassDefinition, ErrorData, Span};
use crate::interpreter::prelude::{
    CallArgListExt, ClassInstance, Interpreter, RuntimeError, SharedInterner, Value,
};
use crate::shared::SharedMut;
use crate::traits::prelude::CoreOperations;
use crate::{
    bail_runtime, define_builtin, define_constructor, define_method, expect_args, runtime_error,
};
use regex::Regex;
use std::any::Any;
use string_interner::DefaultSymbol as Symbol;

fn compile_regex(pattern: &str, span: Span) -> Result<Regex, RuntimeError> {
    Regex::new(pattern).map_err(|err| {
        runtime_error!(
            InvalidOperation,
            span,
            "Некорректное регулярное выражение '{}': {}",
            pattern,
            err
        )
    })
}

fn make_regex_resource(regex: Regex) -> Value {
    Value::NativeResource(SharedMut::new(Box::new(regex) as Box<dyn Any + Send + Sync>))
}

fn build_regex_object(
    interp: &Interpreter,
    pattern: String,
    span: Span,
) -> Result<Value, RuntimeError> {
    let class_symbol = interp.intern_string("РегулярноеВыражение");
    let Some(class_ref) = interp.std_classes.get(&class_symbol).cloned() else {
        return bail_runtime!(
            InvalidOperation,
            span,
            "Класс РегулярноеВыражение не найден"
        );
    };

    let compiled = compile_regex(&pattern, span)?;
    let instance = ClassInstance::new(class_symbol, class_ref);
    let instance_ref = SharedMut::new(instance);
    let pattern_sym = interp.intern_string("__pattern");
    let regex_sym = interp.intern_string("__regex");

    instance_ref.write(|instance| {
        instance
            .field_values
            .insert(pattern_sym, Value::Text(pattern));
        instance
            .field_values
            .insert(regex_sym, make_regex_resource(compiled));
    });

    Ok(Value::Object(instance_ref))
}

fn get_regex_parts(
    interp: &Interpreter,
    args: &[crate::interpreter::prelude::CallArgValue],
    span: Span,
) -> Result<(String, Regex), RuntimeError> {
    let Some(Value::Object(instance_ref)) = CallArgListExt::first_value(args) else {
        return bail_runtime!(TypeError, span, "Ожидался объект РегулярноеВыражение");
    };

    let pattern_sym = interp.intern_string("__pattern");
    let regex_sym = interp.intern_string("__regex");

    instance_ref.read(|instance| {
        let pattern = match instance.field_values.get(&pattern_sym) {
            Some(Value::Text(pattern)) => pattern.clone(),
            _ => {
                return bail_runtime!(
                    InvalidOperation,
                    span,
                    "РегулярноеВыражение не инициализирован"
                )
            }
        };

        let regex = match instance.field_values.get(&regex_sym) {
            Some(Value::NativeResource(resource)) => resource.read(|boxed| {
                boxed
                    .as_ref()
                    .downcast_ref::<Regex>()
                    .cloned()
                    .ok_or_else(|| {
                        runtime_error!(
                            TypeError,
                            span,
                            "Внутренний ресурс РегулярноеВыражение поврежден"
                        )
                    })
            })?,
            _ => {
                return bail_runtime!(
                    InvalidOperation,
                    span,
                    "РегулярноеВыражение не инициализирован"
                )
            }
        };

        Ok((pattern, regex))
    })
}

fn capture_values(captures: regex::Captures<'_>) -> Value {
    let values = captures
        .iter()
        .map(|capture| {
            capture
                .map(|item| Value::Text(item.as_str().to_string()))
                .unwrap_or(Value::Empty)
        })
        .collect();
    Value::List(SharedMut::new(values))
}

pub fn setup_regex_class(interner: &SharedInterner) -> (Symbol, SharedMut<ClassDefinition>) {
    let name =
        interner.write(|i| i.get_or_intern(crate::builtins::catalog::class::REGEX.names.canonical));
    let mut class_def = ClassDefinition::new(name, Span::default());

    define_constructor!(class_def, (interp, args, span) {
        let (Some(Value::Object(instance)), Some(Value::Text(pattern))) = (
            CallArgListExt::first_value(&args),
            CallArgListExt::get_value(&args, 1),
        ) else {
            return bail_runtime!(TypeError, span, "Использование: новый РегулярноеВыражение(шаблон)");
        };

        let compiled = compile_regex(pattern, span)?;
        let pattern_sym = interp.intern_string("__pattern");
        let regex_sym = interp.intern_string("__regex");

        instance.write(|i| {
            i.field_values
                .insert(pattern_sym, Value::Text(pattern.clone()));
            i.field_values
                .insert(regex_sym, make_regex_resource(compiled));
        });

        Ok(Value::Empty)
    });

    define_method!(class_def, interner, crate::builtins::catalog::method::PATTERN.canonical => (interp, args, span) {
        let (pattern, _) = get_regex_parts(interp, &args, span)?;
        Ok(Value::Text(pattern))
    });

    define_method!(class_def, interner, crate::builtins::catalog::method::MATCHES.canonical => (interp, args, span) {
        let (_, regex) = get_regex_parts(interp, &args, span)?;
        if let Some(Value::Text(text)) = CallArgListExt::get_value(&args, 1) {
            Ok(Value::Boolean(regex.is_match(text)))
        } else {
            bail_runtime!(TypeError, span, "Использование: regex.совпадает(text)")
        }
    });

    define_method!(class_def, interner, crate::builtins::catalog::method::FIND.canonical => (interp, args, span) {
        let (_, regex) = get_regex_parts(interp, &args, span)?;
        if let Some(Value::Text(text)) = CallArgListExt::get_value(&args, 1) {
            Ok(regex
                .find(text)
                .map(|item| Value::Text(item.as_str().to_string()))
                .unwrap_or(Value::Empty))
        } else {
            bail_runtime!(TypeError, span, "Использование: regex.найти(text)")
        }
    });

    define_method!(class_def, interner, crate::builtins::catalog::method::FIND_ALL.canonical => (interp, args, span) {
        let (_, regex) = get_regex_parts(interp, &args, span)?;
        if let Some(Value::Text(text)) = CallArgListExt::get_value(&args, 1) {
            let matches = regex
                .find_iter(text)
                .map(|item| Value::Text(item.as_str().to_string()))
                .collect();
            Ok(Value::List(SharedMut::new(matches)))
        } else {
            bail_runtime!(TypeError, span, "Использование: regex.найти_все(text)")
        }
    });

    define_method!(class_def, interner, crate::builtins::catalog::method::GROUPS.canonical => (interp, args, span) {
        let (_, regex) = get_regex_parts(interp, &args, span)?;
        if let Some(Value::Text(text)) = CallArgListExt::get_value(&args, 1) {
            Ok(regex
                .captures(text)
                .map(capture_values)
                .unwrap_or(Value::Empty))
        } else {
            bail_runtime!(TypeError, span, "Использование: regex.группы(text)")
        }
    });

    define_method!(class_def, interner, crate::builtins::catalog::method::GROUPS_ALL.canonical => (interp, args, span) {
        let (_, regex) = get_regex_parts(interp, &args, span)?;
        if let Some(Value::Text(text)) = CallArgListExt::get_value(&args, 1) {
            let groups = regex
                .captures_iter(text)
                .map(capture_values)
                .collect();
            Ok(Value::List(SharedMut::new(groups)))
        } else {
            bail_runtime!(TypeError, span, "Использование: regex.группы_все(text)")
        }
    });

    define_method!(class_def, interner, crate::builtins::catalog::method::REPLACE.canonical => (interp, args, span) {
        let (_, regex) = get_regex_parts(interp, &args, span)?;
        if let (Some(Value::Text(text)), Some(Value::Text(replacement))) = (
            CallArgListExt::get_value(&args, 1),
            CallArgListExt::get_value(&args, 2),
        ) {
            Ok(Value::Text(regex.replace(text, replacement.as_str()).to_string()))
        } else {
            bail_runtime!(TypeError, span, "Использование: regex.заменить(text, replacement)")
        }
    });

    define_method!(class_def, interner, crate::builtins::catalog::method::REPLACE_ALL.canonical => (interp, args, span) {
        let (_, regex) = get_regex_parts(interp, &args, span)?;
        if let (Some(Value::Text(text)), Some(Value::Text(replacement))) = (
            CallArgListExt::get_value(&args, 1),
            CallArgListExt::get_value(&args, 2),
        ) {
            Ok(Value::Text(regex.replace_all(text, replacement.as_str()).to_string()))
        } else {
            bail_runtime!(TypeError, span, "Использование: regex.заменить_все(text, replacement)")
        }
    });

    define_method!(class_def, interner, crate::builtins::catalog::method::SPLIT.canonical => (interp, args, span) {
        let (_, regex) = get_regex_parts(interp, &args, span)?;
        if let Some(Value::Text(text)) = CallArgListExt::get_value(&args, 1) {
            let parts = regex
                .split(text)
                .map(|part| Value::Text(part.to_string()))
                .collect();
            Ok(Value::List(SharedMut::new(parts)))
        } else {
            bail_runtime!(TypeError, span, "Использование: regex.разделить(text)")
        }
    });

    (name, SharedMut::new(class_def))
}

pub fn setup_regex_func(interpreter: &mut Interpreter, interner: &SharedInterner) {
    define_builtin!(interpreter, interner, crate::builtins::catalog::function::REGEX.canonical => (interp, arguments, span) {
        expect_args!(arguments, 1, span, "выражение");
        if let Value::Text(pattern) = &arguments[0].value {
            build_regex_object(interp, pattern.clone(), span)
        } else {
            bail_runtime!(TypeError, span, "Функция регулярное_выражение ожидает строку")
        }
    });
}
