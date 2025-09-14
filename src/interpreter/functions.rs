use crate::ast::prelude::{Function, Program};
use crate::interpreter::structs::{Environment, Interpreter, RuntimeError, Value};
use crate::interpreter::traits::{InterpreterFunctions, StatementExecutor};

impl InterpreterFunctions for Interpreter {
    fn call_function(
        &mut self,
        function: Function,
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
        if let Some(dot_index) = name.find('.') {
            let module_name = &name[..dot_index];
            let func_name = &name[dot_index + 1..];
            if let Some(module) = self.modules.get(module_name).cloned() {
                if let Some(function) = module.functions.get(func_name) {
                    return self.call_function(function.clone(), arguments, &module.program);
                }
            }
            Err(RuntimeError::UndefinedFunction(name.to_string()))
        } else {
            if let Some(function) = self.functions.get(name).cloned() {
                self.call_function(function, arguments, program)
            } else if let Some(module_name) = &self.current_module {
                if let Some(module) = self.modules.get(module_name).cloned() {
                    if let Some(function) = module.functions.get(name) {
                        return self.call_function(function.clone(), arguments, &module.program);
                    }
                }
                Err(RuntimeError::UndefinedFunction(name.to_string()))
            } else {
                Err(RuntimeError::UndefinedFunction(name.to_string()))
            }
        }
    }
}
