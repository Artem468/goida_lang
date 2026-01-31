use crate::interpreter::prelude::{Environment, SharedInterner};
use crate::interpreter::structs::{Interpreter, Module, RuntimeError, Value};
use crate::parser::prelude::Parser;
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

    fn interpret(&mut self, module: Module) -> Result<(), RuntimeError> {
        self.load_imports(&module)?;

        for (class_name, class_def) in &module.classes {
            self.register_class(class_def.clone(), *class_name)?;
        }

        for (function_name, function_fn) in &module.functions {
            let func_value = Value::Function(Rc::new(function_fn.clone()));
            self.environment.define(
                function_name.clone(),
                func_value.clone(),
            );
            if let Some(mod_entry) = self.modules.get_mut(&module.name) {
                mod_entry.globals.insert(*function_name, func_value);
            }
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

    fn load_imports(&mut self, module: &Module) -> Result<(), RuntimeError> {
        for import in &module.imports {
            for path_symbol in &import.files {
                let path = module
                    .arena
                    .resolve_symbol(&self.interner, *path_symbol)
                    .unwrap();
                let relative_path = std::path::Path::new(&path);
                let module_dir = module.path.parent().unwrap_or_else(|| std::path::Path::new("."));
                let full_path = module_dir.join(relative_path).with_extension("goida");
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

                let parser = Parser::Parser::new(
                    Arc::clone(&self.interner),
                    file_stem,
                    full_path.clone(),
                );
                match parser.parse(code.as_str()) {
                    Ok(new_module) => {
                        let module_symbol = self.interner.write().unwrap().get_or_intern(file_stem);
                        
                        if self.modules.contains_key(&module_symbol) {
                            continue;
                        }
                        
                        self.modules.insert(module_symbol, new_module.clone());
                        
                        self.load_imports(&new_module)?;
                        
                        for &stmt_id in &new_module.body {
                            self.execute_statement(stmt_id, module_symbol)?;
                        }
                        
                        for (_class_name, class_def) in &new_module.classes {
                            let class_def_with_module = self.set_class_module(class_def.clone(), module_symbol);
                            self.register_class(class_def_with_module, module_symbol)?;
                        }
                        
                        for (function_name, function_fn) in &new_module.functions {
                            let func_value = Value::Function(Rc::new(function_fn.clone()));
                            self.environment.define(
                                function_name.clone(),
                                func_value.clone(),
                            );
                            if let Some(module) = self.modules.get_mut(&module_symbol) {
                                module.globals.insert(*function_name, func_value);
                            }
                        }
                        
                        let imported_globals = self.collect_imported_globals(&new_module)?;
                        if let Some(module) = self.modules.get_mut(&module_symbol) {
                            for (sym, val) in imported_globals {
                                module.globals.insert(sym, val);
                            }
                        }
                    }
                    Err(err) => {
                        return Err(RuntimeError::ImportError(err));
                    }
                }
            }
        }
        Ok(())
    }

    fn collect_imported_globals(&self, module: &Module) -> Result<Vec<(Symbol, Value)>, RuntimeError> {
        let mut result = Vec::new();
        for import in &module.imports {
            for &imp_mod_sym in &import.files {
                if let Some(imp_module) = self.modules.get(&imp_mod_sym) {
                    for (sym, val) in &imp_module.globals {
                        result.push((*sym, val.clone()));
                    }
                }
            }
        }
        Ok(result)
    }
}