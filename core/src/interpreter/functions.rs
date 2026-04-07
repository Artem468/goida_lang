use crate::ast::prelude::{ErrorData, FunctionDefinition, Span};
use crate::interpreter::structs::{CallArgValue, Environment, Interpreter, RuntimeError, Value};
use crate::shared::SharedMut;
use crate::traits::prelude::{
    CoreOperations, ExpressionEvaluator, InterpreterFunctions, StatementExecutor,
};
use string_interner::DefaultSymbol as Symbol;

impl InterpreterFunctions for Interpreter {
    fn call_function(
        &mut self,
        function: FunctionDefinition,
        arguments: Vec<CallArgValue>,
        current_module_id: Symbol,
        span: Span,
    ) -> Result<Value, RuntimeError> {
        let parent_env = self.environment.clone();
        let final_arguments =
            self.bind_call_arguments(&function, arguments, current_module_id, span, "Функция")?;

        let mut local_env = Environment::with_parent(parent_env.clone());
        for (param, arg_value) in function.params.iter().zip(final_arguments.iter()) {
            local_env.define(param.name, arg_value.clone());
        }

        self.environment = SharedMut::new(local_env);
        let execution_result = self.execute_statement(function.body, current_module_id);
        self.environment = parent_env;

        match execution_result {
            Ok(()) => Ok(Value::Empty),
            Err(RuntimeError::Return(_, val)) => Ok(val),
            Err(e) => Err(e),
        }
    }

    fn call_function_by_name(
        &mut self,
        name: Symbol,
        arguments: Vec<CallArgValue>,
        current_module_id: Symbol,
        span: Span,
    ) -> Result<Value, RuntimeError> {
        if let Some(val) = self.environment.read(|env| env.get(&name)) {
            match val {
                Value::Function(func) => {
                    let func_clone = (*func).clone();
                    return self.call_function(func_clone, arguments, current_module_id, span);
                }
                Value::Builtin(builtin) => {
                    return builtin(self, arguments, span);
                }
                _ => {}
            }
        }

        let name_str = self.resolve_symbol(name).unwrap();

        let current_module = self.modules.get(&current_module_id).ok_or_else(|| {
            RuntimeError::InvalidOperation(ErrorData::new(span, "Текущий модуль не найден".into()))
        })?;

        if let Some(dot_index) = name_str.find('.') {
            let mod_part = &name_str[..dot_index];
            let func_part = &name_str[dot_index + 1..];

            let mod_sym = self.interner.write(|i| i.get_or_intern(mod_part));
            let func_sym = self.interner.write(|i| i.get_or_intern(func_part));

            let target_module_symbol = self.resolve_import_alias_symbol(current_module, mod_sym);

            if let Some(target_module) = target_module_symbol.and_then(|sym| self.modules.get(&sym))
            {
                if let Some(function) = target_module.functions.get(&func_sym) {
                    return self.call_function(
                        function.clone(),
                        arguments,
                        target_module.name,
                        span,
                    );
                }
                if let Some(Value::Builtin(builtin)) = target_module.globals.get(&func_sym) {
                    return builtin(self, arguments, span);
                }
            }
            return Err(RuntimeError::UndefinedFunction(ErrorData::new(
                span, name_str,
            )));
        }

        if let Some(function) = current_module.functions.get(&name) {
            let func_clone = function.clone();
            return self.call_function(func_clone, arguments, current_module_id, span);
        }

        if let Some(Value::Function(func)) = current_module.globals.get(&name) {
            let func_clone = (**func).clone();
            return self.call_function(func_clone, arguments, current_module_id, span);
        }
        if let Some(Value::Builtin(builtin)) = current_module.globals.get(&name) {
            return builtin(self, arguments, span);
        }

        if let Some(builtin_fn) = self.builtins.get(&name) {
            return builtin_fn(self, arguments, span);
        }

        Err(RuntimeError::UndefinedFunction(ErrorData::new(
            span, name_str,
        )))
    }
}

impl Interpreter {
    pub(crate) fn bind_call_arguments(
        &mut self,
        function: &FunctionDefinition,
        arguments: Vec<CallArgValue>,
        current_module_id: Symbol,
        span: Span,
        kind_label: &str,
    ) -> Result<Vec<Value>, RuntimeError> {
        let function_name = self
            .modules
            .get(&current_module_id)
            .and_then(|m| m.arena.resolve_symbol(&self.interner, function.name))
            .unwrap_or_else(|| "неизвестно".to_string());

        let total_params = function.params.len();
        let mut final_args: Vec<Option<Value>> = vec![None; total_params];
        let mut positional_index = 0usize;
        let mut saw_named = false;

        for arg in arguments {
            match arg.name {
                Some(name) => {
                    saw_named = true;
                    let mut target_index = None;
                    for (idx, param) in function.params.iter().enumerate() {
                        if param.name == name {
                            target_index = Some(idx);
                            break;
                        }
                    }

                    let idx = match target_index {
                        Some(i) => i,
                        None => {
                            let name_str = self.resolve_symbol(name).unwrap_or_default();
                            return Err(RuntimeError::InvalidOperation(ErrorData::new(
                                span,
                                format!(
                                    "Неизвестный именованный аргумент '{}' для {} {}",
                                    name_str, kind_label, function_name
                                ),
                            )));
                        }
                    };

                    if final_args[idx].is_some() {
                        let name_str = self.resolve_symbol(name).unwrap_or_default();
                        return Err(RuntimeError::InvalidOperation(ErrorData::new(
                            span,
                            format!(
                                "Аргумент '{}' для {} {} передан несколько раз",
                                name_str, kind_label, function_name
                            ),
                        )));
                    }

                    final_args[idx] = Some(arg.value);
                }
                None => {
                    if saw_named {
                        return Err(RuntimeError::InvalidOperation(ErrorData::new(
                            span,
                            "Именованные аргументы должны идти после позиционных".into(),
                        )));
                    }
                    if positional_index >= total_params {
                        return Err(RuntimeError::InvalidOperation(ErrorData::new(
                            span,
                            format!(
                                "{} {} ожидает {} аргументов, получено {}",
                                kind_label,
                                function_name,
                                total_params,
                                positional_index + 1
                            ),
                        )));
                    }
                    final_args[positional_index] = Some(arg.value);
                    positional_index += 1;
                }
            }
        }

        for (idx, param) in function.params.iter().enumerate() {
            if final_args[idx].is_none() {
                if let Some(default_expr_id) = param.default_value {
                    let val = self.evaluate_expression(default_expr_id, current_module_id)?;
                    final_args[idx] = Some(val);
                } else {
                    let param_name = self.resolve_symbol(param.name).unwrap_or_default();
                    return Err(RuntimeError::InvalidOperation(ErrorData::new(
                        span,
                        format!(
                            "Аргумент '{}' для {} {} не передан",
                            param_name, kind_label, function_name
                        ),
                    )));
                }
            }
        }

        Ok(final_args
            .into_iter()
            .map(|val| val.expect("argument binding should be complete"))
            .collect())
    }
}
