use crate::ast::prelude::{DataType, ErrorData, PrimitiveType, RuntimeType, Span};
use crate::interpreter::native_support::{NativeFfiArgValue, NativeFfiKind};
use crate::interpreter::prelude::{Interpreter, RuntimeError, Value};
use crate::{bail_runtime, runtime_error};
use std::ffi::c_void;
use string_interner::DefaultSymbol as Symbol;

impl Interpreter {
    pub(super) fn native_kind_from_optional_type_id(
        &self,
        type_id: Option<u32>,
        module_id: Symbol,
        span: Span,
    ) -> Result<NativeFfiKind, RuntimeError> {
        let Some(type_id) = type_id else {
            return Ok(NativeFfiKind::Void);
        };
        self.native_kind_from_type_id(type_id, module_id, span)
    }

    pub(super) fn native_kind_from_type_id(
        &self,
        type_id: u32,
        module_id: Symbol,
        span: Span,
    ) -> Result<NativeFfiKind, RuntimeError> {
        let module = self
            .modules
            .get(&module_id)
            .ok_or_else(|| runtime_error!(InvalidOperation, span, "Module not found"))?;
        let data_type =
            module.arena.types.get(type_id as usize).ok_or_else(|| {
                runtime_error!(TypeError, span, "Unknown native type id {}", type_id)
            })?;

        match data_type {
            DataType::Unit => Ok(NativeFfiKind::Void),
            DataType::Primitive(PrimitiveType::Number) => Ok(NativeFfiKind::I64),
            DataType::Primitive(PrimitiveType::Float) => Ok(NativeFfiKind::F64),
            DataType::Primitive(PrimitiveType::Pointer) => Ok(NativeFfiKind::Pointer),
            DataType::Any => Ok(NativeFfiKind::Pointer),
            other => bail_runtime!(
                TypeError,
                span,
                "Неподдерживаемый тип для native ABI: {}. Используйте число/дробь/указатель/пустота",
                    Self::describe_type(other)
            ),
        }
    }

    pub(super) fn value_to_ffi_arg(
        value: Value,
        kind: NativeFfiKind,
        span: Span,
    ) -> Result<NativeFfiArgValue, RuntimeError> {
        match kind {
            NativeFfiKind::I64 => match value {
                Value::Number(number) => Ok(NativeFfiArgValue::I64(number)),
                _ => bail_runtime!(TypeError, span, "Аргумент native-функции должен быть типа 'число'"),
            },
            NativeFfiKind::F64 => match value {
                Value::Float(float) => Ok(NativeFfiArgValue::F64(float)),
                _ => bail_runtime!(TypeError, span, "Аргумент native-функции должен быть типа 'дробь'"),
            },
            NativeFfiKind::Pointer => match value {
                Value::Number(number) => {
                    Ok(NativeFfiArgValue::Pointer(number as isize as usize as *mut c_void))
                }
                Value::Empty => Ok(NativeFfiArgValue::Pointer(std::ptr::null_mut())),
                Value::Text(s) => {
                    let mut s_with_zero = s.clone();
                    s_with_zero.push('\0');

                    let managed_value = Value::Text(s_with_zero);
                    let boxed = Box::new(managed_value);

                    let ptr = if let Value::Text(ref inner_s) = *boxed {
                        inner_s.as_ptr() as *mut c_void
                    } else {
                        std::ptr::null_mut()
                    };

                    Ok(NativeFfiArgValue::ManagedPointer(boxed, ptr))
                }
                value if Self::is_managed_pointer_value(&value) => {
                    let mut boxed = Box::new(value);
                    let ptr = boxed.as_mut() as *mut Value as *mut c_void;
                    Ok(NativeFfiArgValue::ManagedPointer(boxed, ptr))
                }
                _ => bail_runtime!(TypeError, span, "Аргумент типа 'указатель' должен быть адресом, пустотой или значением строка/список/массив/словарь"),
            },
            NativeFfiKind::Void => bail_runtime!(TypeError, span, "Тип 'пустота' нельзя использовать для аргумента native-функции"),
        }
    }

    fn is_managed_pointer_value(value: &Value) -> bool {
        matches!(
            value,
            Value::Text(_) | Value::List(_) | Value::Array(_) | Value::Dict(_)
        )
    }

    pub(super) fn ensure_value_matches_type(
        &self,
        value: &Value,
        expected: Option<u32>,
        module_id: Symbol,
        span: Span,
        label: &str,
    ) -> Result<(), RuntimeError> {
        let Some(expected) = expected else {
            return Ok(());
        };
        if self.value_matches_type(value, expected, module_id)? {
            return Ok(());
        }

        let expected_name = self
            .modules
            .get(&module_id)
            .and_then(|module| module.arena.types.get(expected as usize))
            .map(Self::describe_type)
            .unwrap_or_else(|| "неизвестно".to_string());
        bail_runtime!(
            TypeError,
            span,
            "Неверный тип {}: ожидался {}",
            label,
            expected_name
        )
    }

    fn value_matches_type(
        &self,
        value: &Value,
        expected: u32,
        module_id: Symbol,
    ) -> Result<bool, RuntimeError> {
        let module = self
            .modules
            .get(&module_id)
            .ok_or_else(|| runtime_error!(InvalidOperation, Span::default(), "Module not found"))?;
        let Some(data_type) = module.arena.types.get(expected as usize) else {
            return Ok(true);
        };

        Ok(match data_type {
            DataType::Any => true,
            DataType::Unit => matches!(value, Value::Empty),
            DataType::Primitive(primitive) => match primitive {
                PrimitiveType::Number => matches!(value, Value::Number(_)),
                PrimitiveType::Float => matches!(value, Value::Float(_)),
                PrimitiveType::Text => matches!(value, Value::Text(_)),
                PrimitiveType::Boolean => matches!(value, Value::Boolean(_)),
                PrimitiveType::Pointer => {
                    matches!(value, Value::Number(_) | Value::Empty)
                        || Self::is_managed_pointer_value(value)
                }
            },
            DataType::List(_) => matches!(value, Value::List(_)),
            DataType::Array(_) => matches!(value, Value::Array(_)),
            DataType::Dict { .. } => matches!(value, Value::Dict(_)),
            DataType::Function { .. } => matches!(value, Value::Function(_) | Value::Builtin(_)),
            DataType::Object(symbol) => match value {
                Value::Object(object) => object.read(|object| object.class_name == *symbol),
                Value::Class(class) => class.read(|class| class.name == *symbol),
                _ => false,
            },
            DataType::Runtime(runtime_type) => match runtime_type {
                RuntimeType::Class => matches!(value, Value::Class(_)),
                RuntimeType::Module => matches!(value, Value::Module(_)),
                RuntimeType::Resource => matches!(value, Value::NativeResource(_)),
            },
        })
    }

    fn describe_type(data_type: &DataType) -> String {
        match data_type {
            DataType::Primitive(primitive) => primitive.to_string(),
            DataType::List(_) => "список".into(),
            DataType::Array(_) => "массив".into(),
            DataType::Dict { .. } => "словарь".into(),
            DataType::Function { .. } => "функция".into(),
            DataType::Object(_) => "объект".into(),
            DataType::Runtime(RuntimeType::Class) => "класс".into(),
            DataType::Runtime(RuntimeType::Module) => "модуль".into(),
            DataType::Runtime(RuntimeType::Resource) => "ресурс".into(),
            DataType::Any => "неизвестно".into(),
            DataType::Unit => "пустота".into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn converts_scalar_native_arguments() {
        assert!(matches!(
            Interpreter::value_to_ffi_arg(Value::Number(42), NativeFfiKind::I64, Span::default()),
            Ok(NativeFfiArgValue::I64(42))
        ));
        assert!(matches!(
            Interpreter::value_to_ffi_arg(
                Value::Float(1.5),
                NativeFfiKind::F64,
                Span::default()
            ),
            Ok(NativeFfiArgValue::F64(value)) if value == 1.5
        ));
    }

    #[test]
    fn converts_empty_value_to_null_pointer() {
        assert!(matches!(
            Interpreter::value_to_ffi_arg(
                Value::Empty,
                NativeFfiKind::Pointer,
                Span::default()
            ),
            Ok(NativeFfiArgValue::Pointer(pointer)) if pointer.is_null()
        ));
    }

    #[test]
    fn rejects_mismatched_native_argument_type() {
        assert!(matches!(
            Interpreter::value_to_ffi_arg(
                Value::Text("wrong".into()),
                NativeFfiKind::I64,
                Span::default()
            ),
            Err(RuntimeError::TypeError(_))
        ));
    }
}
