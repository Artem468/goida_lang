use crate::ast::prelude::{ErrorData, NativeLibraryDefinition, Span};
use crate::interpreter::native_support::{
    load_native_library, native_library_path_candidates, NativeFfiArgValue, NativeFfiKind,
};
use crate::interpreter::prelude::{
    BuiltinFn, CallArgValue, Interpreter, LoadedNativeLibrary, NativeFunctionBinding,
    NativeGlobalBinding, RuntimeError, Value,
};
use crate::shared::SharedMut;
use crate::traits::prelude::CoreOperations;
use crate::{bail_runtime, runtime_error};
use libffi::middle::{Arg, Cif, CodePtr};
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
                module.set_global(function.name, value);
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
                module.set_global(global.name, value);
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
            bail_runtime!(
                InvalidOperation,
                span,
                "Аргумент '{}' для native {} не передан",
                param_name,
                function_name
            )
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
                return bail_runtime!(
                    TypeError,
                    span,
                    "Native parameter '{}' cannot be void",
                    self.resolve_symbol(param.name).unwrap_or_default()
                );
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
                    runtime_error!(
                        InvalidOperation,
                        span,
                        "Failed to find symbol '{}': {}",
                        symbol_name,
                        err
                    )
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
        let module = self
            .modules
            .get(&current_module_id)
            .ok_or_else(|| runtime_error!(InvalidOperation, span, "Current module is missing"))?;
        let path = module
            .arena
            .resolve_symbol(&self.interner, path_symbol)
            .ok_or_else(|| {
                runtime_error!(InvalidOperation, span, "Native library path is missing")
            })?;

        let relative_path = Path::new(&path);
        let module_path = if module.path.is_absolute() {
            module.path.clone()
        } else {
            std::env::current_dir()
                .map_err(|err| {
                    runtime_error!(IOError, span, "Failed to resolve current directory: {err}")
                })?
                .join(&module.path)
        };
        let module_dir = module_path.parent().unwrap_or_else(|| Path::new("."));
        let full_path = module_dir.join(relative_path);

        for candidate in native_library_path_candidates(&full_path) {
            if candidate.exists() {
                return candidate.canonicalize().map_err(|err| {
                    runtime_error!(
                        IOError,
                        span,
                        "Failed to normalize native library path '{}': {}",
                        candidate.display(),
                        err
                    )
                });
            }
        }

        bail_runtime!(
            IOError,
            span,
            "Native library not found: {}",
            full_path.display()
        )
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
            runtime_error!(
                IOError,
                span,
                "Failed to load native library '{}': {}",
                path.display(),
                detail
            )
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
            runtime_error!(
                InvalidOperation,
                span,
                "Native library '{}' is not loaded",
                path.display()
            )
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
            return bail_runtime!(
                TypeError,
                span,
                "Global '{}' cannot have void type",
                global_name
            );
        }

        let library = self.get_loaded_native_library(&binding.library_path, span)?;
        let symbol_name = global_name.as_bytes();
        let value = library.read(|library| unsafe {
            match kind {
                NativeFfiKind::I64 => {
                    let symbol = library.handle.get::<*mut i64>(symbol_name).map_err(|err| {
                        runtime_error!(
                            InvalidOperation,
                            span,
                            "Failed to find symbol '{}': {}",
                            global_name,
                            err
                        )
                    })?;
                    let ptr = *symbol;
                    if ptr.is_null() {
                        return bail_runtime!(
                            InvalidOperation,
                            span,
                            "Global '{}' is not initialized",
                            global_name
                        );
                    }
                    Ok(Value::Number(*ptr))
                }
                NativeFfiKind::F64 => {
                    let symbol = library.handle.get::<*mut f64>(symbol_name).map_err(|err| {
                        runtime_error!(
                            InvalidOperation,
                            span,
                            "Failed to find symbol '{}': {}",
                            global_name,
                            err
                        )
                    })?;
                    let ptr = *symbol;
                    if ptr.is_null() {
                        return bail_runtime!(
                            InvalidOperation,
                            span,
                            "Global '{}' is not initialized",
                            global_name
                        );
                    }
                    Ok(Value::Float(*ptr))
                }
                NativeFfiKind::Pointer => {
                    let symbol = library
                        .handle
                        .get::<*mut *mut c_void>(symbol_name)
                        .map_err(|err| {
                            runtime_error!(
                                InvalidOperation,
                                span,
                                "Failed to find symbol '{}': {}",
                                global_name,
                                err
                            )
                        })?;
                    let ptr = *symbol;
                    if ptr.is_null() {
                        return bail_runtime!(
                            InvalidOperation,
                            span,
                            "Global '{}' is not initialized",
                            global_name
                        );
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
            return bail_runtime!(
                TypeError,
                span,
                "Global '{}' cannot have void type",
                global_name
            );
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
                        runtime_error!(
                            InvalidOperation,
                            span,
                            "Failed to find symbol '{}': {}",
                            global_name,
                            err
                        )
                    })?;
                    let ptr = *symbol;
                    if ptr.is_null() {
                        return bail_runtime!(
                            InvalidOperation,
                            span,
                            "Global '{}' is not initialized",
                            global_name
                        );
                    }
                    let Value::Number(number) = value else {
                        return bail_runtime!(
                            TypeError,
                            span,
                            "Global '{}' expects i64 value",
                            global_name
                        );
                    };
                    *ptr = number;
                    Ok(())
                }
                NativeFfiKind::F64 => {
                    let symbol = library.handle.get::<*mut f64>(symbol_name).map_err(|err| {
                        runtime_error!(
                            InvalidOperation,
                            span,
                            "Failed to find symbol '{}': {}",
                            global_name,
                            err
                        )
                    })?;
                    let ptr = *symbol;
                    if ptr.is_null() {
                        return bail_runtime!(
                            InvalidOperation,
                            span,
                            "Global '{}' is not initialized",
                            global_name
                        );
                    }
                    let Value::Float(float) = value else {
                        return bail_runtime!(
                            TypeError,
                            span,
                            "Global '{}' expects f64 value",
                            global_name
                        );
                    };
                    *ptr = float;
                    Ok(())
                }
                NativeFfiKind::Pointer => {
                    let symbol = library
                        .handle
                        .get::<*mut *mut c_void>(symbol_name)
                        .map_err(|err| {
                            runtime_error!(
                                InvalidOperation,
                                span,
                                "Failed to find symbol '{}': {}",
                                global_name,
                                err
                            )
                        })?;
                    let ptr = *symbol;
                    if ptr.is_null() {
                        return bail_runtime!(
                            InvalidOperation,
                            span,
                            "Global '{}' is not initialized",
                            global_name
                        );
                    }
                    let number = match value {
                        Value::Number(number) => number,
                        Value::Empty => 0,
                        _ => {
                            return bail_runtime!(
                                TypeError,
                                span,
                                "Global '{}' expects pointer value",
                                global_name
                            )
                        }
                    };
                    *ptr = number as isize as usize as *mut c_void;
                    Ok(())
                }
                NativeFfiKind::Void => unreachable!(),
            }
        })
    }
}
