use crate::ast::prelude::{ClassDefinition, Span};
use crate::ast::program::FieldData;
use crate::ast::source::SourceManager;
use crate::interpreter::prelude::{Environment, SharedInterner};
use crate::interpreter::structs::{Interpreter, Module, RuntimeError, Value};
use crate::shared::SharedMut;
use crate::traits::prelude::{CoreOperations, ExpressionEvaluator, StatementExecutor};
use std::collections::{HashMap, HashSet};
use std::path::Path;
use string_interner::DefaultSymbol as Symbol;

impl CoreOperations for Interpreter {
    fn new(interner: SharedInterner) -> Self {
        Interpreter {
            std_classes: HashMap::new(),
            builtins: HashMap::new(),
            modules: HashMap::new(),
            native_libraries: HashMap::new(),
            interner,
            environment: SharedMut::new(Environment::new()),
            background_threads: Vec::new(),
            method_depth: 0,
            source_manager: SourceManager::new(),
        }
    }

    fn load_start_module(&mut self, main_module: Module) -> &mut Self {
        self.modules.clear();
        self.register_module_tree(main_module);
        self
    }

    fn interpret(&mut self, module_id: Symbol) -> Result<(), RuntimeError> {
        let mut visited = HashSet::new();
        self.interpret_module(module_id, &mut visited)
    }

    fn resolve_import_alias_symbol(
        &self,
        current_module: &Module,
        alias: Symbol,
    ) -> Option<Symbol> {
        if let Some(Value::Module(module_symbol)) = current_module.globals.get(&alias) {
            return Some(*module_symbol);
        }

        for import in &current_module.imports {
            let item = &import.item;
            if item.alias == alias {
                let path = current_module
                    .arena
                    .resolve_symbol(&self.interner, item.path)?;
                let full_path = current_module
                    .path
                    .parent()
                    .unwrap_or_else(|| Path::new("."))
                    .join(Path::new(&path))
                    .with_extension("goida");
                let normalized_full_path = full_path
                    .canonicalize()
                    .unwrap_or(full_path)
                    .to_string_lossy()
                    .to_string();
                return Some(
                    self.interner
                        .write(|i| i.get_or_intern(normalized_full_path.as_str())),
                );
            }
        }

        None
    }

    fn resolve_symbol(&self, symbol: Symbol) -> Option<String> {
        self.interner
            .read(|i| i.resolve(symbol).map(|s| s.to_string()))
    }

    fn intern_string(&self, s: &str) -> Symbol {
        self.interner.write(|i| i.get_or_intern(s))
    }

    fn get_class_for_value(&self, value: &Value) -> Option<SharedMut<ClassDefinition>> {
        let class_name = match value {
            Value::Text(_) => "Строка",
            Value::List(_) => "Список",
            Value::Array(_) => "Массив",
            Value::Dict(_) => "Словарь",
            Value::Iterator(_) => "Итератор",
            Value::Thread(_) => "Поток",
            Value::Mutex(_) => "Мьютекс",
            Value::RwLock(_) => "БлокировкаЧтенияЗаписи",
            Value::Float(_) => "Дробь",
            Value::Number(_) => "Число",
            Value::Boolean(_) => "Логический",
            Value::Object(inst) => return Some(inst.read(|i| i.class_ref.clone())),
            Value::Class(class_def) => return Some(class_def.clone()),
            _ => return None,
        };

        let symbol = self.interner.read(|i| i.get(class_name))?;
        self.std_classes.get(&symbol).cloned()
    }

    fn get_file_path(&self, module_id: &Symbol) -> String {
        if let Some(module) = self.modules.get(module_id) {
            if let Ok(path) = module.path.canonicalize() {
                let name = path.to_string_lossy().to_string();
                return name.strip_prefix(r"\\?\").unwrap_or(&name).to_string();
            }

            let name = module.path.to_string_lossy().to_string();
            return name.strip_prefix(r"\\?\").unwrap_or(&name).to_string();
        }

        self.resolve_symbol(*module_id)
            .unwrap_or_else(|| "<unknown>".to_string())
    }
}

impl Interpreter {
    pub(crate) fn runtime_error_matches(
        &self,
        runtime_error_class: &str,
        handler_class: Symbol,
        current_module_id: Symbol,
    ) -> bool {
        let runtime_symbol = self.interner.read(|i| i.get(runtime_error_class));
        if runtime_symbol == Some(handler_class) {
            return true;
        }

        let generic_error = self.interner.read(|i| i.get("Ошибка"));
        if generic_error == Some(handler_class) {
            return true;
        }

        let Some(runtime_symbol) = runtime_symbol else {
            return false;
        };

        let mut current = Some(runtime_symbol);
        while let Some(class_symbol) = current {
            if class_symbol == handler_class {
                return true;
            }

            current = self
                .modules
                .get(&current_module_id)
                .and_then(|module| module.classes.get(&class_symbol))
                .and_then(|class| class.read(|class_def| class_def.base_class));
        }

        false
    }

    fn register_module_tree(&mut self, mut module: Module) {
        let module_path = module.path.to_string_lossy().to_string();
        self.source_manager.load_file(module_path.as_str());
        let nested_modules = std::mem::take(&mut module.modules);
        for nested_module in nested_modules.into_values() {
            self.register_module_tree(nested_module);
        }
        self.modules.insert(module.name, module);
    }

    fn interpret_module(
        &mut self,
        module_id: Symbol,
        visited: &mut HashSet<Symbol>,
    ) -> Result<(), RuntimeError> {
        if !visited.insert(module_id) {
            return Ok(());
        }

        let module = self.modules.get(&module_id).unwrap().clone();

        for import in &module.imports {
            if let Some(imported_module_id) =
                self.resolve_import_alias_symbol(&module, import.item.alias)
            {
                self.interpret_module(imported_module_id, visited)?;

                if let Some(imported_module) = self.modules.get(&imported_module_id).cloned() {
                    if let Some(current_module) = self.modules.get_mut(&module.name) {
                        for (name, value) in imported_module.globals {
                            current_module.globals.entry(name).or_insert(value);
                        }
                    }
                }
            }
        }

        self.scoped_environment(Environment::new(), |interpreter| {
            if let Some(mod_entry) = interpreter.modules.get(&module.name) {
                for (name, value) in mod_entry.globals.clone() {
                    interpreter.environment.write(|env| env.define(name, value));
                }
            }

            for (class_name, class_def) in &module.classes {
                let class_value = Value::Class(class_def.clone());
                interpreter
                    .environment
                    .write(|env| env.define(*class_name, class_value.clone()));
                if let Some(mod_entry) = interpreter.modules.get_mut(&module.name) {
                    mod_entry.globals.insert(*class_name, class_value);
                }

                let fields = class_def.read(|i| i.fields.clone());
                for (name, (_, is_static, data)) in fields {
                    if is_static {
                        if let FieldData::Expression(Some(expr_id)) = data {
                            let val = interpreter.evaluate_expression(expr_id, module.name)?;

                            class_def.write(|c| {
                                if let Some((_, _, target_data)) = c.fields.get_mut(&name) {
                                    *target_data = FieldData::Value(SharedMut::new(val));
                                }
                            });
                        }
                    }
                }
            }

            for (function_name, function_fn) in &module.functions {
                let func_value = Value::Function(function_fn.clone());
                interpreter
                    .environment
                    .write(|env| env.define(*function_name, func_value.clone()));
                if let Some(mod_entry) = interpreter.modules.get_mut(&module.name) {
                    mod_entry.globals.insert(*function_name, func_value);
                }
            }

            for (builtin_name, builtin_fn) in &interpreter.builtins.clone() {
                interpreter
                    .environment
                    .write(|env| env.define(*builtin_name, Value::Builtin(builtin_fn.clone())));
            }

            for (name_symbol, class_def) in &interpreter.std_classes.clone() {
                interpreter
                    .environment
                    .write(|env| env.define(*name_symbol, Value::Class(class_def.clone())));

                if let Some(mod_entry) = interpreter.modules.get_mut(&module.name) {
                    mod_entry
                        .globals
                        .insert(*name_symbol, Value::Class(class_def.clone()));
                }
            }

            for &stmt_id in &module.body {
                match interpreter.execute_statement(stmt_id, module.name) {
                    Err(RuntimeError::Return(..)) => {}
                    Err(e) => {
                        interpreter.join_background_threads(module.name, Span::default())?;
                        return Err(e);
                    }
                    Ok(()) => {}
                }
            }

            interpreter.join_background_threads(module.name, Span::default())?;

            Ok(())
        })
    }

    pub(crate) fn resolve_module_member_value(
        &self,
        module_id: Symbol,
        member: Symbol,
    ) -> Option<(Symbol, Value)> {
        let mut visited = HashSet::new();
        self.resolve_module_member_value_inner(module_id, member, &mut visited)
    }

    fn resolve_module_member_value_inner(
        &self,
        module_id: Symbol,
        member: Symbol,
        visited: &mut HashSet<Symbol>,
    ) -> Option<(Symbol, Value)> {
        if !visited.insert(module_id) {
            return None;
        }

        let module = self.modules.get(&module_id)?;

        if let Some(function) = module.functions.get(&member) {
            return Some((module_id, Value::Function(function.clone())));
        }

        if let Some(class) = module.classes.get(&member) {
            return Some((module_id, Value::Class(class.clone())));
        }

        if let Some(value) = module.globals.get(&member) {
            return Some((module_id, value.clone()));
        }

        for import in &module.imports {
            let imported_module_id = self.resolve_import_alias_symbol(module, import.item.alias)?;
            if let Some(found) =
                self.resolve_module_member_value_inner(imported_module_id, member, visited)
            {
                return Some(found);
            }
        }

        None
    }

    pub(crate) fn fork_for_thread(&self) -> Self {
        Self {
            std_classes: self.std_classes.clone(),
            builtins: self.builtins.clone(),
            modules: self.modules.clone(),
            native_libraries: self.native_libraries.clone(),
            interner: self.interner.clone(),
            environment: self.environment.clone(),
            background_threads: Vec::new(),
            method_depth: self.method_depth,
            source_manager: SourceManager::new(),
        }
    }
}
