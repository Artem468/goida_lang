use crate::interpreter::prelude::{Environment, SharedInterner};
use crate::interpreter::structs::{Interpreter, Module, RuntimeError, Value};
use crate::parser::prelude::ParserStructs;
use crate::traits::prelude::{CoreOperations, InterpreterClasses, StatementExecutor};
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::Arc;
use string_interner::DefaultSymbol as Symbol;

impl CoreOperations for Interpreter {
    fn new(main_module: Module, interner: SharedInterner) -> Self {
        let mut modules = HashMap::new();
        modules.insert(main_module.name, main_module);

        Interpreter {
            builtins: HashMap::new(),
            modules,
            interner,
            environment: Environment::new(),
        }
    }

    fn interpret(&mut self, mut module: Module) -> Result<(), RuntimeError> {
        for import in &module.imports {
            for path_symbol in &import.files {
                let path = module
                    .arena
                    .resolve_symbol(&self.interner, *path_symbol)
                    .unwrap();
                let relative_path = std::path::Path::new(&path);
                let full_path = module.path.join(relative_path).with_extension("goida");
                let file_stem =
                    full_path
                        .file_stem()
                        .and_then(|s| s.to_str())
                        .ok_or_else(|| {
                            RuntimeError::InvalidOperation(format!(
                                "Невозможно получить имя модуля из пути: {}",
                                full_path.display()
                            ))
                        })?;
                let code = std::fs::read_to_string(&full_path).map_err(|err| {
                    RuntimeError::IOError(format!("{} | {err}", full_path.display()))
                })?;

                let parser = ParserStructs::Parser::new(
                    Arc::clone(&self.interner),
                    file_stem,
                    module.path.clone(),
                );
                match parser.parse(code.as_str()) {
                    Ok(new_module) => {
                        let module_symbol = self.interner.write().unwrap().get_or_intern(file_stem);
                        for &stmt_id in &new_module.body {
                            self.execute_statement(stmt_id, module_symbol)?;
                        }
                        self.modules.insert(module_symbol, new_module);
                    }
                    Err(err) => {
                        println!("{:#?}", err);
                    }
                }
            }
        }

        for (class_name, class_def) in &module.classes {
            self.register_class(class_def.clone(), *class_name)?;
        }

        for (function_name, function_fn) in &module.functions {
            self.environment.define(
                function_name.clone(),
                Value::Function(Rc::new(function_fn.clone())),
            );
        }

        for (builtin_name, builtin_fn) in &self.builtins.clone() {
            self.environment
                .define(builtin_name.clone(), Value::Builtin(builtin_fn.clone()));
        }

        for &stmt_id in &module.body {
            match self.execute_statement(stmt_id, module.name) {
                Err(RuntimeError::Return(_)) => {}
                Err(e) => return Err(e),
                Ok(()) => {}
            }
        }

        Ok(())
    }

    fn resolve_symbol(&self, symbol: Symbol) -> Option<String> {
        self.interner
            .read()
            .expect("interner lock poisoned")
            .resolve(symbol)
            .map(|s| s.to_string())
    }

    fn intern_string(&self, s: &str) -> Symbol {
        self.interner
            .write()
            .expect("interner lock poisoned")
            .get_or_intern(s)
    }
}
