use crate::ast::prelude::FunctionDefinition;
use crate::interpreter::structs::{Environment, Interpreter, RuntimeError, Value};
use crate::traits::prelude::{CoreOperations, InterpreterFunctions, StatementExecutor};
use string_interner::DefaultSymbol as Symbol;

impl InterpreterFunctions for Interpreter {
    fn call_function(
        &mut self,
        function: FunctionDefinition,
        arguments: Vec<Value>,
        current_module_id: Symbol,
    ) -> Result<Value, RuntimeError> {
        let arena = &self.modules.get(&current_module_id).unwrap().arena;

        let parent_env = self.environment.clone();
        self.environment = Environment::with_parent(parent_env);

        if arguments.len() != function.params.len() {
            return Err(RuntimeError::InvalidOperation(format!(
                "Функция {} ожидает {} аргументов, получено {}",
                arena.resolve_symbol(&self.interner, function.name).unwrap(),
                function.params.len(),
                arguments.len()
            )));
        }

        for (param, arg_value) in function.params.iter().zip(arguments.iter()) {
            self.environment.define(param.name, arg_value.clone());
        }

        let mut result = Value::Empty;
        match self.execute_statement(function.body, current_module_id) {
            Ok(()) => {}
            Err(RuntimeError::Return(val)) => {
                result = val;
            }
            Err(e) => {
                self.environment = self.environment.clone().pop();
                return Err(e);
            }
        }

        self.environment = self.environment.clone().pop();
        Ok(result)
    }

    fn call_function_by_name(
        &mut self,
        name: Symbol,
        arguments: Vec<Value>,
        current_module_id: Symbol,
    ) -> Result<Value, RuntimeError> {
        let name_str = self.resolve_symbol(name).unwrap();

        if let Some(dot_index) = name_str.find('.') {
            let mod_part = &name_str[..dot_index];
            let func_part = &name_str[dot_index + 1..];

            let mod_sym = self.interner.write().unwrap().get_or_intern(mod_part);
            let func_sym = self.interner.write().unwrap().get_or_intern(func_part);

            if let Some(target_module) = self.modules.get(&mod_sym) {
                if let Some(function) = target_module.functions.get(&func_sym) {
                    return self.call_function(function.clone(), arguments, mod_sym);
                }
            }
            return Err(RuntimeError::UndefinedFunction(name_str));
        }
        
        let current_module = self
            .modules
            .get(&current_module_id)
            .ok_or_else(|| RuntimeError::InvalidOperation("Текущий модуль не найден".into()))?;

        if let Some(function) = current_module.functions.get(&name) {
            let func_clone = function.clone();
            return self.call_function(func_clone, arguments, current_module_id);
        }
        
        if let Some(Value::Function(func)) = current_module.globals.get(&name) {
            let func_clone = (**func).clone();
            return self.call_function(func_clone, arguments, current_module_id);
        }
        
        for import in &current_module.imports {
            for &module_symbol in &import.files {
                if let Some(m) = self.modules.get(&module_symbol) {
                    if let Some(f) = m.functions.get(&name) {
                        let f_clone = f.clone();
                        return self.call_function(f_clone, arguments, module_symbol);
                    }
                }
            }
        }
        
        if let Some(builtin_fn) = self.builtins.get(&name) {
            return builtin_fn(self, arguments);
        }

        Err(RuntimeError::UndefinedFunction(name_str))
    }
}
