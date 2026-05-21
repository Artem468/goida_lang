use crate::ast::prelude::{ClassDefinition, ErrorData, Span};
use crate::interpreter::prelude::{
    CallArgListExt, CallArgValue, Interpreter, IteratorStep, RuntimeError, RuntimeIterator,
    SharedInterner, Value,
};
use crate::shared::SharedMut;
use crate::traits::prelude::InterpreterFunctions;
use crate::{bail_runtime, define_builtin, define_method, runtime_error};
use string_interner::DefaultSymbol as Symbol;

pub(crate) fn values_from_iterable(value: &Value, span: Span) -> Result<Vec<Value>, RuntimeError> {
    match value {
        Value::List(list) => Ok(list.read(|items| items.clone())),
        Value::Array(items) => Ok(items.as_ref().clone()),
        Value::Text(text) => Ok(text.chars().map(|ch| Value::Text(ch.to_string())).collect()),
        Value::Dict(dict) => Ok(dict.read(|items| {
            let mut keys: Vec<_> = items.keys().cloned().collect();
            keys.sort();
            keys.into_iter().map(Value::Text).collect()
        })),
        Value::Iterator(iterator) => Ok(iterator.source.as_ref().clone()),
        _ => bail_runtime!(TypeError, span, "Значение нельзя преобразовать в итератор"),
    }
}

fn call_callable(
    interp: &Interpreter,
    callable: Value,
    arguments: Vec<Value>,
    span: Span,
) -> Result<Value, RuntimeError> {
    let args = arguments
        .into_iter()
        .map(|value| CallArgValue { name: None, value })
        .collect();

    match callable {
        Value::Function(function) => {
            let module_id = function.module.unwrap_or(function.span.file_id);
            let mut local = interp.fork_for_thread();
            local.call_function(function, args, module_id, span)
        }
        Value::Builtin(builtin) => builtin(interp, args, span),
        _ => bail_runtime!(TypeError, span, "Ожидалась функция"),
    }
}

pub(crate) fn collect_iterator(
    interp: &Interpreter,
    iterator: &RuntimeIterator,
    span: Span,
) -> Result<Vec<Value>, RuntimeError> {
    let mut output = Vec::new();

    'items: for source_item in iterator.source.iter() {
        let mut current = source_item.clone();
        for step in iterator.steps.iter() {
            match step {
                IteratorStep::Map(callable) => {
                    current = call_callable(interp, callable.clone(), vec![current], span)?;
                }
                IteratorStep::Filter(callable) => {
                    let keep =
                        call_callable(interp, callable.clone(), vec![current.clone()], span)?;
                    if !keep.is_truthy() {
                        continue 'items;
                    }
                }
            }
        }
        output.push(current);
    }

    Ok(output)
}

pub fn setup_iterator_class(interner: &SharedInterner) -> (Symbol, SharedMut<ClassDefinition>) {
    let name = interner.write(|i| i.get_or_intern("Итератор"));

    let mut class_def = ClassDefinition::new(name, Span::default());

    define_method!(class_def, interner, "преобразовать" => (_, args, span) {
        if let (Some(Value::Iterator(iterator)), Some(callable)) = (
            CallArgListExt::first_value(&args),
            CallArgListExt::get_value(&args, 1),
        ) {
            Ok(Value::Iterator(iterator.with_step(IteratorStep::Map(callable.clone()))))
        } else {
            bail_runtime!(TypeError, span, "Использование: iterator.map(function)")
        }
    });

    define_method!(class_def, interner, "отфильтровать" => (_, args, span) {
        if let (Some(Value::Iterator(iterator)), Some(callable)) = (
            CallArgListExt::first_value(&args),
            CallArgListExt::get_value(&args, 1),
        ) {
            Ok(Value::Iterator(iterator.with_step(IteratorStep::Filter(callable.clone()))))
        } else {
            bail_runtime!(TypeError, span, "Использование: iterator.filter(function)")
        }
    });

    define_method!(class_def, interner, "свернуть" => (interp, args, span) {
        if let (Some(Value::Iterator(iterator)), Some(callable), Some(initial)) = (
            CallArgListExt::first_value(&args),
            CallArgListExt::get_value(&args, 1),
            CallArgListExt::get_value(&args, 2),
        ) {
            let mut acc = initial.clone();
            for item in collect_iterator(interp, iterator, span)? {
                acc = call_callable(interp, callable.clone(), vec![acc, item], span)?;
            }
            Ok(acc)
        } else {
            bail_runtime!(TypeError, span, "Использование: iterator.reduce(function, initial)")
        }
    });

    define_method!(class_def, interner, "список" => (interp, args, span) {
        if let Some(Value::Iterator(iterator)) = CallArgListExt::first_value(&args) {
            Ok(Value::List(SharedMut::new(collect_iterator(interp, iterator, span)?)))
        } else {
            bail_runtime!(TypeError, span, "Ожидался итератор")
        }
    });

    (name, SharedMut::new(class_def))
}

pub fn setup_iterator_func(interpreter: &mut Interpreter, interner: &SharedInterner) {
    define_builtin!(interpreter, interner, "итератор" => (_, arguments, span) {
        let Some(value) = arguments.first().map(|arg| &arg.value) else {
            return bail_runtime!(InvalidOperation, span, "итератор ожидает коллекцию");
        };
        Ok(Value::Iterator(RuntimeIterator::new(values_from_iterable(value, span)?)))
    });
}
