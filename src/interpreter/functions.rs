use crate::ast::prelude::{ErrorData, FunctionDefinition, Span};
use crate::interpreter::structs::{Environment, Interpreter, RuntimeError, Value};
use crate::shared::SharedMut;
use crate::traits::prelude::{CoreOperations, ExpressionEvaluator, InterpreterFunctions, StatementExecutor};
use string_interner::DefaultSymbol as Symbol;

impl InterpreterFunctions for Interpreter {
    fn call_function(
        &mut self,
        function: FunctionDefinition,
        mut arguments: Vec<Value>,
        current_module_id: Symbol,
        span: Span,
    ) -> Result<Value, RuntimeError> {
        let function_name = self.modules.get(&current_module_id)
            .and_then(|m| m.arena.resolve_symbol(&self.interner, function.name))
            .unwrap_or_else(|| "неизвестно".to_string());

        let parent_env = self.environment.clone();
        let total_params = function.params.len();

        if arguments.len() > total_params {
            return Err(RuntimeError::InvalidOperation(ErrorData::new(
                span,
                format!("Функция {} ожидает {} аргументов, получено {}",
                        function_name, total_params, arguments.len()),
            )));
        }

        if arguments.len() < total_params {
            for i in arguments.len()..total_params {
                let param = &function.params[i];
                if let Some(default_expr_id) = param.default_value {
                    let val = self.evaluate_expression(default_expr_id, current_module_id)?;
                    arguments.push(val);
                } else {
                    let param_name = self.resolve_symbol(param.name).unwrap_or_default();
                    return Err(RuntimeError::InvalidOperation(ErrorData::new(
                        span,
                        format!("Аргумент '{}' функции {} не передан", param_name, function_name),
                    )));
                }
            }
        }

        let mut local_env = Environment::with_parent(parent_env.clone());
        for (param, arg_value) in function.params.iter().zip(arguments.iter()) {
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
        arguments: Vec<Value>,
        current_module_id: Symbol,
        span: Span,
    ) -> Result<Value, RuntimeError> {
        if let Some(val) = self.environment.read(|env| env.get(&name)) {
            if let Value::Function(func) = val {
                let func_clone = (*func).clone();
                return self.call_function(func_clone, arguments, current_module_id, span);
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

        if let Some(builtin_fn) = self.builtins.get(&name) {
            return builtin_fn(self, arguments, span);
        }

        Err(RuntimeError::UndefinedFunction(ErrorData::new(
            span, name_str,
        )))
    }
}
