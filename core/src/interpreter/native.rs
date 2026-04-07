use crate::ast::prelude::{
    DataType, ErrorData, NativeLibraryDefinition, PrimitiveType, RuntimeType, Span,
};
use crate::ffi::GoidaFfiValue;
use crate::interpreter::prelude::{
    BuiltinFn, CallArgValue, Interpreter, LoadedNativeLibrary, NativeFunctionBinding,
    NativeGlobalBinding, RuntimeError, Value,
};
use crate::shared::SharedMut;
use crate::traits::prelude::CoreOperations;
use libffi::middle::{Arg, Cif, CodePtr, Type};
use libloading::Library;
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
                symbol_name: self.resolve_symbol(function.name).unwrap_or_default(),
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
                symbol_name: self.resolve_symbol(global.name).unwrap_or_default(),
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
        let bound_arguments = self.bind_native_arguments(binding, arguments, span)?;
        let mut ffi_values: Vec<GoidaFfiValue> = bound_arguments
            .iter()
            .cloned()
            .map(GoidaFfiValue::from_value)
            .collect();
        let raw_ptrs: Vec<*const GoidaFfiValue> =
            ffi_values.iter().map(|value| value as *const _).collect();
        let ffi_args: Vec<Arg> = raw_ptrs.iter().map(Arg::new).collect();
        let cif = Cif::new(vec![Type::pointer(); ffi_args.len()], Type::pointer());

        let library = self.get_loaded_native_library(&binding.library_path, span)?;
        let symbol_name = binding.symbol_name.as_bytes();
        let function_ptr = library.read(|library| unsafe {
            library
                .handle
                .get::<*const ()>(symbol_name)
                .map(|symbol| *symbol)
                .map_err(|err| {
                    RuntimeError::InvalidOperation(ErrorData::new(
                        span,
                        format!("Не удалось найти символ '{}': {}", binding.symbol_name, err),
                    ))
                })
        })?;

        let result_ptr = unsafe {
            cif.call::<*mut GoidaFfiValue>(CodePtr::from_ptr(function_ptr as *mut _), &ffi_args)
        };

        for value in &mut ffi_values {
            unsafe { value.release_boxed() };
        }

        if result_ptr.is_null() {
            return Ok(Value::Empty);
        }

        let result = unsafe {
            let ffi_value = *Box::from_raw(result_ptr);
            ffi_value.into_value()
        };
        self.ensure_value_matches_type(
            &result,
            binding.return_type,
            binding.module_id,
            span,
            "возвращаемое значение",
        )?;
        Ok(result)
    }

    fn bind_native_arguments(
        &self,
        binding: &NativeFunctionBinding,
        arguments: Vec<CallArgValue>,
        span: Span,
    ) -> Result<Vec<Value>, RuntimeError> {
        let mut final_args: Vec<Option<Value>> = vec![None; binding.params.len()];
        let mut positional_index = 0usize;
        let mut saw_named = false;

        for argument in arguments {
            match argument.name {
                Some(name) => {
                    saw_named = true;
                    let index = binding
                        .params
                        .iter()
                        .position(|param| param.name == name)
                        .ok_or_else(|| {
                            RuntimeError::InvalidOperation(ErrorData::new(
                                span,
                                format!(
                                    "Неизвестный именованный аргумент '{}'",
                                    self.resolve_symbol(name).unwrap_or_default()
                                ),
                            ))
                        })?;

                    if final_args[index].is_some() {
                        return Err(RuntimeError::InvalidOperation(ErrorData::new(
                            span,
                            format!(
                                "Аргумент '{}' передан несколько раз",
                                self.resolve_symbol(name).unwrap_or_default()
                            ),
                        )));
                    }
                    final_args[index] = Some(argument.value);
                }
                None => {
                    if saw_named {
                        return Err(RuntimeError::InvalidOperation(ErrorData::new(
                            span,
                            "Именованные аргументы должны идти после позиционных".into(),
                        )));
                    }
                    if positional_index >= binding.params.len() {
                        return Err(RuntimeError::InvalidOperation(ErrorData::new(
                            span,
                            format!(
                                "Ожидалось {} аргументов, получено {}",
                                binding.params.len(),
                                positional_index + 1
                            ),
                        )));
                    }
                    final_args[positional_index] = Some(argument.value);
                    positional_index += 1;
                }
            }
        }

        let mut values = Vec::with_capacity(binding.params.len());
        for (index, param) in binding.params.iter().enumerate() {
            let value = final_args[index].clone().ok_or_else(|| {
                RuntimeError::InvalidOperation(ErrorData::new(
                    span,
                    format!(
                        "Аргумент '{}' не передан",
                        self.resolve_symbol(param.name).unwrap_or_default()
                    ),
                ))
            })?;
            self.ensure_value_matches_type(
                &value,
                Some(param.param_type),
                binding.module_id,
                span,
                "аргумент",
            )?;
            values.push(value);
        }

        Ok(values)
    }

    fn resolve_native_library_path(
        &self,
        current_module_id: Symbol,
        path_symbol: Symbol,
        span: Span,
    ) -> Result<PathBuf, RuntimeError> {
        let module = self.modules.get(&current_module_id).ok_or_else(|| {
            RuntimeError::InvalidOperation(ErrorData::new(span, "Текущий модуль не найден".into()))
        })?;
        let path = module
            .arena
            .resolve_symbol(&self.interner, path_symbol)
            .ok_or_else(|| {
                RuntimeError::InvalidOperation(ErrorData::new(
                    span,
                    "Путь библиотеки не найден".into(),
                ))
            })?;

        let relative_path = Path::new(&path);
        let module_dir = module.path.parent().unwrap_or_else(|| Path::new("."));
        let full_path = module_dir.join(relative_path);

        if full_path.exists() {
            return Ok(full_path);
        }

        if full_path.extension().is_none() {
            let candidate = full_path.with_extension(std::env::consts::DLL_EXTENSION);
            if candidate.exists() {
                return Ok(candidate);
            }
        }

        Err(RuntimeError::IOError(ErrorData::new(
            span,
            format!("Библиотека не найдена: {}", full_path.display()),
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

        let library = unsafe { Library::new(path) }.map_err(|err| {
            RuntimeError::IOError(ErrorData::new(
                span,
                format!(
                    "Не удалось загрузить библиотеку '{}': {}",
                    path.display(),
                    err
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
                format!("Библиотека '{}' не загружена", path.display()),
            ))
        })
    }

    fn read_native_global(
        &self,
        binding: &NativeGlobalBinding,
        span: Span,
    ) -> Result<Value, RuntimeError> {
        let library = self.get_loaded_native_library(&binding.library_path, span)?;
        let symbol_name = binding.symbol_name.as_bytes();
        let value = library.read(|library| unsafe {
            let symbol = library
                .handle
                .get::<*mut GoidaFfiValue>(symbol_name)
                .map_err(|err| {
                    RuntimeError::InvalidOperation(ErrorData::new(
                        span,
                        format!("Не удалось найти символ '{}': {}", binding.symbol_name, err),
                    ))
                })?;
            let ptr = *symbol;
            if ptr.is_null() {
                return Err(RuntimeError::InvalidOperation(ErrorData::new(
                    span,
                    format!(
                        "Глобальная переменная '{}' не инициализирована",
                        binding.symbol_name
                    ),
                )));
            }
            Ok((*ptr).clone_value())
        })?;

        self.ensure_value_matches_type(
            &value,
            Some(binding.value_type),
            binding.module_id,
            span,
            "глобальная переменная",
        )?;
        Ok(value)
    }

    fn write_native_global(
        &self,
        binding: &NativeGlobalBinding,
        value: Value,
        span: Span,
    ) -> Result<(), RuntimeError> {
        self.ensure_value_matches_type(
            &value,
            Some(binding.value_type),
            binding.module_id,
            span,
            "глобальная переменная",
        )?;
        let library = self.get_loaded_native_library(&binding.library_path, span)?;
        let symbol_name = binding.symbol_name.as_bytes();
        library.read(|library| unsafe {
            let symbol = library
                .handle
                .get::<*mut GoidaFfiValue>(symbol_name)
                .map_err(|err| {
                    RuntimeError::InvalidOperation(ErrorData::new(
                        span,
                        format!("Не удалось найти символ '{}': {}", binding.symbol_name, err),
                    ))
                })?;
            let ptr = *symbol;
            if ptr.is_null() {
                return Err(RuntimeError::InvalidOperation(ErrorData::new(
                    span,
                    format!(
                        "Глобальная переменная '{}' не инициализирована",
                        binding.symbol_name
                    ),
                )));
            }
            (*ptr).write_value(value);
            Ok(())
        })
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
                "Модуль не найден".into(),
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
