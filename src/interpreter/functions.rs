use crate::ast::prelude::{FunctionDefinition, Program};
use crate::interpreter::structs::{Environment, Interpreter, RuntimeError, Value};
use crate::interpreter::traits::{InterpreterFunctions, StatementExecutor};

impl InterpreterFunctions for Interpreter {
    fn call_function(
        &mut self,
        function: FunctionDefinition,
        arguments: Vec<Value>,
        program: &Program,
    ) -> Result<Value, RuntimeError> {
        let prev_module = self.current_module.clone();
        let module_name = if let Some(module_symbol) = function.module {
            Some(
                program
                    .arena
                    .resolve_symbol(module_symbol)
                    .unwrap()
                    .to_string(),
            )
        } else {
            None
        };
        self.current_module = module_name;

        let parent_env = self.environment.clone();
        self.environment = Environment::with_parent(parent_env);

        if arguments.len() != function.params.len() {
            return Err(RuntimeError::InvalidOperation(format!(
                "Функция {} ожидает {} аргументов, получено {}",
                program.arena.resolve_symbol(function.name).unwrap(),
                function.params.len(),
                arguments.len()
            )));
        }

        for (param, arg_value) in function.params.iter().zip(arguments.iter()) {
            let param_name = program
                .arena
                .resolve_symbol(param.name)
                .unwrap()
                .to_string();
            self.environment.define(param_name, arg_value.clone());
        }

        let mut result = Value::Empty;
        match self.execute_statement(function.body, program) {
            Ok(()) => {}
            Err(RuntimeError::Return(val)) => {
                result = val;
            }
            Err(e) => {
                self.environment = self.environment.clone().pop();
                self.current_module = prev_module;
                return Err(e);
            }
        }

        self.environment = self.environment.clone().pop();
        self.current_module = prev_module;
        Ok(result)
    }

    fn call_function_by_name(
        &mut self,
        name: &str,
        arguments: Vec<Value>,
        program: &Program,
    ) -> Result<Value, RuntimeError> {
        // 1. Проверяем доступ через модуль (module.func)
        if let Some(dot_index) = name.find('.') {
            let module_name = &name[..dot_index];
            let func_name = &name[dot_index + 1..];
            if let Some(module) = self.modules.get(module_name).cloned() {
                if let Some(function) = module.functions.get(func_name) {
                    return self.call_function(function.clone(), arguments, &module.program);
                }
            }
            return Err(RuntimeError::UndefinedFunction(name.to_string()));
        }

        // 2. Сначала ищем в environment
        if let Some(val) = self.environment.get(name) {
            if let Value::Function(func) = val {
                return self.call_function((*func).clone(), arguments, program);
            } else {
                return Err(RuntimeError::InvalidOperation(format!(
                    "'{}' не является функцией",
                    name
                )));
            }
        }

        // 3. Ищем в текущем модуле
        if let Some(function) = self.functions.get(name).cloned() {
            return self.call_function(function, arguments, program);
        }

        if let Some(module_name) = &self.current_module {
            if let Some(module) = self.modules.get(module_name).cloned() {
                if let Some(function) = module.functions.get(name) {
                    return self.call_function(function.clone(), arguments, &module.program);
                }
            }
        }

        // 4. Если нигде не найдено
        Err(RuntimeError::UndefinedFunction(name.to_string()))
    }

}
