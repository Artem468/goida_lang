use crate::ast::prelude::{
    DataType, ErrorData, NativeLibraryDefinition, PrimitiveType, RuntimeType, Span,
};
use crate::interpreter::prelude::{
    BuiltinFn, CallArgValue, Interpreter, LoadedNativeLibrary, NativeFunctionBinding,
    NativeGlobalBinding, RuntimeError, Value,
};
use crate::shared::SharedMut;
use crate::traits::prelude::CoreOperations;
use libffi::middle::{Arg, Cif, CodePtr, Type};
#[cfg(windows)]
use libloading::os::windows::{Library as WindowsLibrary, LOAD_WITH_ALTERED_SEARCH_PATH};
use libloading::Library;
use std::error::Error as StdError;
use std::ffi::c_void;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use string_interner::DefaultSymbol as Symbol;

impl Interpreter {
    pub(crate) fn load_native_library_definition(
        &mut self,
        definition: NativeLibraryDefinition,
        current_module_id: Symbol,
    ) -> Result<(), RuntimeError> {
        let path =
            self.resolve_native_library_path(current_module_id, definition.path, definition.span)?;
        self.ensure_native_library_loaded(&path, definition.span)?;

        let path = Arc::new(path);

        for function in definition.functions {
            let binding = NativeFunctionBinding {
                module_id: current_module_id,
                library_path: path.clone(),
                symbol_name: function.name,
                params: function.params.clone(),
                return_type: function.return_type,
            };
            let binding_for_closure = binding.clone();
            let value = Value::Builtin(BuiltinFn(Arc::new(move |interpreter, arguments, span| {
                interpreter.call_native_function(&binding_for_closure, arguments, span)
            })));

            self.environment
                .write(|env| env.define(function.name, value.clone()));
            if let Some(module) = self.modules.get_mut(&current_module_id) {
                module.globals.insert(function.name, value);
            }
        }

        for global in definition.globals {
            let value = Value::NativeGlobal(Arc::new(NativeGlobalBinding {
                module_id: current_module_id,
                library_path: path.clone(),
                symbol_name: global.name,
                value_type: global.value_type,
            }));

            self.environment
                .write(|env| env.define(global.name, value.clone()));
            if let Some(module) = self.modules.get_mut(&current_module_id) {
                module.globals.insert(global.name, value);
            }
        }

        Ok(())
    }

    pub(crate) fn resolve_runtime_value(
        &self,
        value: Value,
        span: Span,
    ) -> Result<Value, RuntimeError> {
        match value {
            Value::NativeGlobal(binding) => self.read_native_global(&binding, span),
            other => Ok(other),
        }
    }

    pub(crate) fn try_assign_native_identifier(
        &self,
        name: Symbol,
        value: Value,
        current_module_id: Symbol,
        span: Span,
    ) -> Result<bool, RuntimeError> {
        if let Some(Value::NativeGlobal(binding)) = self.environment.read(|env| env.get(&name)) {
            self.write_native_global(&binding, value, span)?;
            return Ok(true);
        }

        if let Some(module) = self.modules.get(&current_module_id) {
            if let Some(Value::NativeGlobal(binding)) = module.globals.get(&name) {
                self.write_native_global(binding, value, span)?;
                return Ok(true);
            }
        }

        Ok(false)
    }

    pub(crate) fn call_native_function(
        &self,
        binding: &NativeFunctionBinding,
        arguments: Vec<CallArgValue>,
        span: Span,
    ) -> Result<Value, RuntimeError> {
        let function_name = self.resolve_symbol(binding.symbol_name).unwrap_or_default();
        let interner = self.interner.clone();
        let resolve_symbol = move |symbol| {
            interner
                .read(|i| i.resolve(symbol).map(|s| s.to_string()))
                .unwrap_or_default()
        };
        let mut missing = |param: &crate::ast::prelude::Parameter| {
            let param_name = self.resolve_symbol(param.name).unwrap_or_default();
            Err(RuntimeError::InvalidOperation(ErrorData::new(
                span,
                format!(
                    "Аргумент '{}' для native {} не передан",
                    param_name, function_name
                ),
            )))
        };
        let bound_arguments = Self::bind_arguments(
            &binding.params,
            arguments,
            span,
            "native",
            &function_name,
            resolve_symbol,
            &mut missing,
        )?;

        let mut ffi_values = Vec::with_capacity(bound_arguments.len());
        let mut ffi_param_types = Vec::with_capacity(bound_arguments.len());
        for (param, value) in binding.params.iter().zip(bound_arguments.iter()) {
            let kind = self.native_kind_from_type_id(param.param_type, binding.module_id, span)?;
            if kind == NativeFfiKind::Void {
                return Err(RuntimeError::TypeError(ErrorData::new(
                    span,
                    format!(
                        "Native parameter '{}' cannot be void",
                        self.resolve_symbol(param.name).unwrap_or_default()
                    ),
                )));
            }
            ffi_param_types.push(kind.libffi_type());
            ffi_values.push(Self::value_to_ffi_arg(value.clone(), kind, span)?);
        }

        let ffi_args: Vec<Arg> = ffi_values.iter().map(NativeFfiArgValue::as_arg).collect();
        let return_kind =
            self.native_kind_from_optional_type_id(binding.return_type, binding.module_id, span)?;
        let cif = Cif::new(ffi_param_types, return_kind.libffi_type());

        let library = self.get_loaded_native_library(&binding.library_path, span)?;
        let symbol_name = self.resolve_symbol(binding.symbol_name).unwrap_or_default();
        let function_ptr = library.read(|library| unsafe {
            library
                .handle
                .get::<*const ()>(symbol_name.as_bytes())
                .map(|symbol| *symbol)
                .map_err(|err| {
                    RuntimeError::InvalidOperation(ErrorData::new(
                        span,
                        format!("Failed to find symbol '{}': {}", symbol_name, err),
                    ))
                })
        })?;

        let result = unsafe {
            match return_kind {
                NativeFfiKind::Void => {
                    cif.call::<()>(CodePtr::from_ptr(function_ptr as *mut _), &ffi_args);
                    Value::Empty
                }
                NativeFfiKind::I64 => Value::Number(
                    cif.call::<i64>(CodePtr::from_ptr(function_ptr as *mut _), &ffi_args),
                ),
                NativeFfiKind::F64 => Value::Float(
                    cif.call::<f64>(CodePtr::from_ptr(function_ptr as *mut _), &ffi_args),
                ),
                NativeFfiKind::Pointer => {
                    let ptr = cif
                        .call::<*mut c_void>(CodePtr::from_ptr(function_ptr as *mut _), &ffi_args);
                    if ptr.is_null() {
                        Value::Empty
                    } else if let Some(value) = ffi_values
                        .iter()
                        .find_map(|arg| arg.roundtrip_value_for_pointer(ptr))
                    {
                        value
                    } else {
                        Value::Number(ptr as isize as i64)
                    }
                }
            }
        };

        self.ensure_value_matches_type(
            &result,
            binding.return_type,
            binding.module_id,
            span,
            "return value",
        )?;
        Ok(result)
    }

    fn resolve_native_library_path(
        &self,
        current_module_id: Symbol,
        path_symbol: Symbol,
        span: Span,
    ) -> Result<PathBuf, RuntimeError> {
        let module = self.modules.get(&current_module_id).ok_or_else(|| {
            RuntimeError::InvalidOperation(ErrorData::new(span, "Current module is missing".into()))
        })?;
        let path = module
            .arena
            .resolve_symbol(&self.interner, path_symbol)
            .ok_or_else(|| {
                RuntimeError::InvalidOperation(ErrorData::new(
                    span,
                    "Native library path is missing".into(),
                ))
            })?;

        let relative_path = Path::new(&path);
        let module_path = if module.path.is_absolute() {
            module.path.clone()
        } else {
            std::env::current_dir()
                .map_err(|err| {
                    RuntimeError::IOError(ErrorData::new(
                        span,
                        format!("Failed to resolve current directory: {err}"),
                    ))
                })?
                .join(&module.path)
        };
        let module_dir = module_path.parent().unwrap_or_else(|| Path::new("."));
        let full_path = module_dir.join(relative_path);

        if full_path.exists() {
            return full_path.canonicalize().map_err(|err| {
                RuntimeError::IOError(ErrorData::new(
                    span,
                    format!(
                        "Failed to normalize native library path '{}': {}",
                        full_path.display(),
                        err
                    ),
                ))
            });
        }

        if full_path.extension().is_none() {
            let candidate = full_path.with_extension(std::env::consts::DLL_EXTENSION);
            if candidate.exists() {
                return candidate.canonicalize().map_err(|err| {
                    RuntimeError::IOError(ErrorData::new(
                        span,
                        format!(
                            "Failed to normalize native library path '{}': {}",
                            candidate.display(),
                            err
                        ),
                    ))
                });
            }
        }

        Err(RuntimeError::IOError(ErrorData::new(
            span,
            format!("Native library not found: {}", full_path.display()),
        )))
    }

    fn ensure_native_library_loaded(
        &mut self,
        path: &Path,
        span: Span,
    ) -> Result<(), RuntimeError> {
        if self.native_libraries.contains_key(path) {
            return Ok(());
        }

        let library = load_native_library(path).map_err(|err| {
            let detail = err
                .source()
                .map(|source| format!("{err}: {source}"))
                .unwrap_or_else(|| err.to_string());
            RuntimeError::IOError(ErrorData::new(
                span,
                format!(
                    "Failed to load native library '{}': {}",
                    path.display(),
                    detail
                ),
            ))
        })?;

        self.native_libraries.insert(
            path.to_path_buf(),
            SharedMut::new(LoadedNativeLibrary { handle: library }),
        );
        Ok(())
    }

    fn get_loaded_native_library(
        &self,
        path: &Path,
        span: Span,
    ) -> Result<SharedMut<LoadedNativeLibrary>, RuntimeError> {
        self.native_libraries.get(path).cloned().ok_or_else(|| {
            RuntimeError::InvalidOperation(ErrorData::new(
                span,
                format!("Native library '{}' is not loaded", path.display()),
            ))
        })
    }

    fn read_native_global(
        &self,
        binding: &NativeGlobalBinding,
        span: Span,
    ) -> Result<Value, RuntimeError> {
        let kind = self.native_kind_from_type_id(binding.value_type, binding.module_id, span)?;
        let global_name = self.resolve_symbol(binding.symbol_name).unwrap_or_default();
        if kind == NativeFfiKind::Void {
            return Err(RuntimeError::TypeError(ErrorData::new(
                span,
                format!("Global '{}' cannot have void type", global_name),
            )));
        }

        let library = self.get_loaded_native_library(&binding.library_path, span)?;
        let symbol_name = global_name.as_bytes();
        let value = library.read(|library| unsafe {
            match kind {
                NativeFfiKind::I64 => {
                    let symbol = library.handle.get::<*mut i64>(symbol_name).map_err(|err| {
                        RuntimeError::InvalidOperation(ErrorData::new(
                            span,
                            format!("Failed to find symbol '{}': {}", global_name, err),
                        ))
                    })?;
                    let ptr = *symbol;
                    if ptr.is_null() {
                        return Err(RuntimeError::InvalidOperation(ErrorData::new(
                            span,
                            format!("Global '{}' is not initialized", global_name),
                        )));
                    }
                    Ok(Value::Number(*ptr))
                }
                NativeFfiKind::F64 => {
                    let symbol = library.handle.get::<*mut f64>(symbol_name).map_err(|err| {
                        RuntimeError::InvalidOperation(ErrorData::new(
                            span,
                            format!("Failed to find symbol '{}': {}", global_name, err),
                        ))
                    })?;
                    let ptr = *symbol;
                    if ptr.is_null() {
                        return Err(RuntimeError::InvalidOperation(ErrorData::new(
                            span,
                            format!("Global '{}' is not initialized", global_name),
                        )));
                    }
                    Ok(Value::Float(*ptr))
                }
                NativeFfiKind::Pointer => {
                    let symbol = library
                        .handle
                        .get::<*mut *mut c_void>(symbol_name)
                        .map_err(|err| {
                            RuntimeError::InvalidOperation(ErrorData::new(
                                span,
                                format!("Failed to find symbol '{}': {}", global_name, err),
                            ))
                        })?;
                    let ptr = *symbol;
                    if ptr.is_null() {
                        return Err(RuntimeError::InvalidOperation(ErrorData::new(
                            span,
                            format!("Global '{}' is not initialized", global_name),
                        )));
                    }
                    Ok(Value::Number((*ptr) as isize as i64))
                }
                NativeFfiKind::Void => unreachable!(),
            }
        })?;

        self.ensure_value_matches_type(
            &value,
            Some(binding.value_type),
            binding.module_id,
            span,
            "global",
        )?;
        Ok(value)
    }

    fn write_native_global(
        &self,
        binding: &NativeGlobalBinding,
        value: Value,
        span: Span,
    ) -> Result<(), RuntimeError> {
        let kind = self.native_kind_from_type_id(binding.value_type, binding.module_id, span)?;
        let global_name = self.resolve_symbol(binding.symbol_name).unwrap_or_default();
        if kind == NativeFfiKind::Void {
            return Err(RuntimeError::TypeError(ErrorData::new(
                span,
                format!("Global '{}' cannot have void type", global_name),
            )));
        }

        self.ensure_value_matches_type(
            &value,
            Some(binding.value_type),
            binding.module_id,
            span,
            "global",
        )?;
        let library = self.get_loaded_native_library(&binding.library_path, span)?;
        let symbol_name = global_name.as_bytes();
        library.read(|library| unsafe {
            match kind {
                NativeFfiKind::I64 => {
                    let symbol = library.handle.get::<*mut i64>(symbol_name).map_err(|err| {
                        RuntimeError::InvalidOperation(ErrorData::new(
                            span,
                            format!("Failed to find symbol '{}': {}", global_name, err),
                        ))
                    })?;
                    let ptr = *symbol;
                    if ptr.is_null() {
                        return Err(RuntimeError::InvalidOperation(ErrorData::new(
                            span,
                            format!("Global '{}' is not initialized", global_name),
                        )));
                    }
                    let Value::Number(number) = value else {
                        return Err(RuntimeError::TypeError(ErrorData::new(
                            span,
                            format!("Global '{}' expects i64 value", global_name),
                        )));
                    };
                    *ptr = number;
                    Ok(())
                }
                NativeFfiKind::F64 => {
                    let symbol = library.handle.get::<*mut f64>(symbol_name).map_err(|err| {
                        RuntimeError::InvalidOperation(ErrorData::new(
                            span,
                            format!("Failed to find symbol '{}': {}", global_name, err),
                        ))
                    })?;
                    let ptr = *symbol;
                    if ptr.is_null() {
                        return Err(RuntimeError::InvalidOperation(ErrorData::new(
                            span,
                            format!("Global '{}' is not initialized", global_name),
                        )));
                    }
                    let Value::Float(float) = value else {
                        return Err(RuntimeError::TypeError(ErrorData::new(
                            span,
                            format!("Global '{}' expects f64 value", global_name),
                        )));
                    };
                    *ptr = float;
                    Ok(())
                }
                NativeFfiKind::Pointer => {
                    let symbol = library
                        .handle
                        .get::<*mut *mut c_void>(symbol_name)
                        .map_err(|err| {
                            RuntimeError::InvalidOperation(ErrorData::new(
                                span,
                                format!("Failed to find symbol '{}': {}", global_name, err),
                            ))
                        })?;
                    let ptr = *symbol;
                    if ptr.is_null() {
                        return Err(RuntimeError::InvalidOperation(ErrorData::new(
                            span,
                            format!("Global '{}' is not initialized", global_name),
                        )));
                    }
                    let number = match value {
                        Value::Number(number) => number,
                        Value::Empty => 0,
                        _ => {
                            return Err(RuntimeError::TypeError(ErrorData::new(
                                span,
                                format!("Global '{}' expects pointer value", global_name),
                            )))
                        }
                    };
                    *ptr = number as isize as usize as *mut c_void;
                    Ok(())
                }
                NativeFfiKind::Void => unreachable!(),
            }
        })
    }

    fn native_kind_from_optional_type_id(
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

    fn native_kind_from_type_id(
        &self,
        type_id: u32,
        module_id: Symbol,
        span: Span,
    ) -> Result<NativeFfiKind, RuntimeError> {
        let module = self.modules.get(&module_id).ok_or_else(|| {
            RuntimeError::InvalidOperation(ErrorData::new(span, "Module not found".into()))
        })?;
        let data_type = module.arena.types.get(type_id as usize).ok_or_else(|| {
            RuntimeError::TypeError(ErrorData::new(
                span,
                format!("Unknown native type id {}", type_id),
            ))
        })?;

        match data_type {
            DataType::Unit => Ok(NativeFfiKind::Void),
            DataType::Primitive(PrimitiveType::Number) => Ok(NativeFfiKind::I64),
            DataType::Primitive(PrimitiveType::Float) => Ok(NativeFfiKind::F64),
            DataType::Primitive(PrimitiveType::Pointer) => Ok(NativeFfiKind::Pointer),
            DataType::Any => Ok(NativeFfiKind::Pointer),
            other => Err(RuntimeError::TypeError(ErrorData::new(
                span,
                format!(
                    "Неподдерживаемый тип для native ABI: {}. Используйте число/дробь/указатель/пустота",
                    Self::describe_type(other)
                ),
            ))),
        }
    }

    fn value_to_ffi_arg(
        value: Value,
        kind: NativeFfiKind,
        span: Span,
    ) -> Result<NativeFfiArgValue, RuntimeError> {
        match kind {
            NativeFfiKind::I64 => match value {
                Value::Number(number) => Ok(NativeFfiArgValue::I64(number)),
                _ => Err(RuntimeError::TypeError(ErrorData::new(
                    span,
                    "Аргумент native-функции должен быть типа 'число'".into(),
                ))),
            },
            NativeFfiKind::F64 => match value {
                Value::Float(float) => Ok(NativeFfiArgValue::F64(float)),
                _ => Err(RuntimeError::TypeError(ErrorData::new(
                    span,
                    "Аргумент native-функции должен быть типа 'дробь'".into(),
                ))),
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
                _ => Err(RuntimeError::TypeError(ErrorData::new(
                    span,
                    "Аргумент типа 'указатель' должен быть адресом, пустотой или значением строка/список/массив/словарь".into(),
                ))),
            },
            NativeFfiKind::Void => Err(RuntimeError::TypeError(ErrorData::new(
                span,
                "Тип 'пустота' нельзя использовать для аргумента native-функции".into(),
            ))),
        }
    }

    fn is_managed_pointer_value(value: &Value) -> bool {
        matches!(
            value,
            Value::Text(_) | Value::List(_) | Value::Array(_) | Value::Dict(_)
        )
    }

    fn ensure_value_matches_type(
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
        Err(RuntimeError::TypeError(ErrorData::new(
            span,
            format!("Неверный тип {}: ожидался {}", label, expected_name),
        )))
    }

    fn value_matches_type(
        &self,
        value: &Value,
        expected: u32,
        module_id: Symbol,
    ) -> Result<bool, RuntimeError> {
        let module = self.modules.get(&module_id).ok_or_else(|| {
            RuntimeError::InvalidOperation(ErrorData::new(
                Span::default(),
                "Module not found".into(),
            ))
        })?;
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

#[cfg(windows)]
fn load_native_library(path: &Path) -> Result<Library, libloading::Error> {
    unsafe { WindowsLibrary::load_with_flags(path, LOAD_WITH_ALTERED_SEARCH_PATH) }.map(Into::into)
}

#[cfg(not(windows))]
fn load_native_library(path: &Path) -> Result<Library, libloading::Error> {
    unsafe { Library::new(path) }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum NativeFfiKind {
    Void,
    I64,
    F64,
    Pointer,
}

impl NativeFfiKind {
    fn libffi_type(self) -> Type {
        match self {
            NativeFfiKind::Void => Type::void(),
            NativeFfiKind::I64 => Type::i64(),
            NativeFfiKind::F64 => Type::f64(),
            NativeFfiKind::Pointer => Type::pointer(),
        }
    }
}

#[derive(Debug, Clone)]
enum NativeFfiArgValue {
    I64(i64),
    F64(f64),
    Pointer(*mut c_void),
    ManagedPointer(Box<Value>, *mut c_void),
}

impl NativeFfiArgValue {
    fn as_arg(&self) -> Arg<'_> {
        match self {
            NativeFfiArgValue::I64(value) => Arg::new(value),
            NativeFfiArgValue::F64(value) => Arg::new(value),
            NativeFfiArgValue::Pointer(value) => Arg::new(value),
            NativeFfiArgValue::ManagedPointer(_, ptr) => Arg::new(ptr),
        }
    }

    fn roundtrip_value_for_pointer(&self, pointer: *mut c_void) -> Option<Value> {
        match self {
            NativeFfiArgValue::ManagedPointer(value, ptr) if *ptr == pointer => {
                Some((**value).clone())
            }
            _ => None,
        }
    }
}
