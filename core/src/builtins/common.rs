use crate::ast::prelude::ErrorData;
use crate::interpreter::prelude::{Interpreter, RuntimeError, SharedInterner, Value};
use crate::traits::core::CoreOperations;
use crate::{define_builtin, expect_args, runtime_error};

pub fn setup_type_func(interpreter: &mut Interpreter, interner: &SharedInterner) {
    define_builtin!(interpreter, interner, crate::builtins::catalog::function::TYPE.canonical => (interpreter, arguments, span) {
        expect_args!(arguments, 1, span, "тип");

        let val = &arguments[0].value;
        match val {
            Value::Number(_) => Ok(Value::Text("число".into())),
            Value::Float(_) => Ok(Value::Text("дробь".into())),
            Value::Text(_) => Ok(Value::Text("строка".into())),
            Value::Boolean(_) => Ok(Value::Text("логический".into())),
            Value::Object(obj) => {
                let name_sym = obj.read(|i| i.class_name);
                let name = interpreter.resolve_symbol(name_sym)
                    .ok_or_else(|| runtime_error!(InvalidOperation, span, "Тип не найден"))?;
                Ok(Value::Text(format!("объект \"{}\"", name)))
            }
            Value::Class(cls) => {
                let name_sym = cls.read(|i| i.name);
                let name = interpreter.resolve_symbol(name_sym)
                    .ok_or_else(|| runtime_error!(InvalidOperation, span, "Тип не найден"))?;
                Ok(Value::Text(format!("класс \"{}\"", name)))
            }
            Value::Function(obj) => {
                let name = interpreter.resolve_symbol(obj.name)
                    .ok_or_else(|| runtime_error!(InvalidOperation, span, "Тип не найден"))?;
                Ok(Value::Text(format!("функция \"{}\"", name)))
            }
            Value::Builtin(_) => Ok(Value::Text("встроенная функция".into())),
            Value::Module(sym) => {
                let name = interpreter.resolve_symbol(*sym)
                    .ok_or_else(|| runtime_error!(InvalidOperation, span, "Модуль не найден"))?;
                Ok(Value::Text(format!("модуль \"{}\"", name)))
            }
            Value::List(_) => Ok(Value::Text("список".into())),
            Value::Array(_) => Ok(Value::Text("массив".into())),
            Value::Dict(_) => Ok(Value::Text("словарь".into())),
            Value::Iterator(_) => Ok(Value::Text("итератор".into())),
            Value::Thread(_) => Ok(Value::Text("Поток".into())),
            Value::Mutex(_) => Ok(Value::Text("Мьютекс".into())),
            Value::RwLock(_) => Ok(Value::Text("БлокировкаЧтенияЗаписи".into())),
            Value::NativeResource(_) => Ok(Value::Text("ресурс".into())),
            Value::NativeGlobal(_) => Ok(Value::Text("нативная переменная".into())),
            Value::Empty => Ok(Value::Text("пустота".into())),
        }
    });
}

pub fn setup_is_instance_func(interpreter: &mut Interpreter, interner: &SharedInterner) {
    define_builtin!(interpreter, interner, crate::builtins::catalog::function::IS.canonical => (interpreter, arguments, span) {
        expect_args!(arguments, 2, span, "является");

        let target = &arguments[0].value;
        let schema = &arguments[1].value;

        match (target, schema) {
            (Value::Object(obj), Value::Class(cls)) => {
                let obj_class_sym = obj.read(|i| i.class_name);
                let target_class_sym = cls.read(|c| c.name);
                Ok(Value::Boolean(obj_class_sym == target_class_sym))
            }
            (val, Value::Class(cls)) => {
                let target_class_sym = cls.read(|c| c.name);
                if let Some(actual_class_sym) = interpreter.get_class_for_value(val) {
                    let obj_class_sym = actual_class_sym.read(|d| d.name);
                    Ok(Value::Boolean(target_class_sym == obj_class_sym))
                } else {
                    Ok(Value::Boolean(false))
                }
            }
            _ => Ok(Value::Boolean(false)),
        }
    });
}
