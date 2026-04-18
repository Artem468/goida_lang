use crate::ast::prelude::ClassDefinition;
use crate::ast::program::FieldData;
use crate::ast::source::SourceManager;
use crate::interpreter::prelude::{Environment, SharedInterner};
use crate::interpreter::structs::{Interpreter, Module, RuntimeError, Value};
use crate::shared::SharedMut;
use crate::traits::prelude::{CoreOperations, ExpressionEvaluator, StatementExecutor};
use std::collections::{HashMap, HashSet};
use std::path::Path;
use std::sync::Arc;
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
            Value::Text(_) => "РЎС‚СЂРѕРєР°",
            Value::List(_) => "РЎРїРёСЃРѕРє",
            Value::Array(_) => "РњР°СЃСЃРёРІ",
            Value::Dict(_) => "РЎР»РѕРІР°СЂСЊ",
            Value::Float(_) => "Р”СЂРѕР±СЊ",
            Value::Number(_) => "Р§РёСЃР»Рѕ",
            Value::Boolean(_) => "Р›РѕРіРёС‡РµСЃРєРёР№",
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

        for (class_name, class_def) in &module.classes {
            let class_value = Value::Class(class_def.clone());
            self.environment
                .write(|env| env.define(*class_name, class_value.clone()));
            if let Some(mod_entry) = self.modules.get_mut(&module.name) {
                mod_entry.globals.insert(*class_name, class_value);
            }

            let fields = class_def.read(|i| i.fields.clone());
            for (name, (_, is_static, data)) in fields {
                if is_static {
                    if let FieldData::Expression(Some(expr_id)) = data {
                        let val = self.evaluate_expression(expr_id, module.name)?;

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
            let func_value = Value::Function(Arc::new(function_fn.clone()));
            self.environment
                .write(|env| env.define(*function_name, func_value.clone()));
            if let Some(mod_entry) = self.modules.get_mut(&module.name) {
                mod_entry.globals.insert(*function_name, func_value);
            }
        }

        for (builtin_name, builtin_fn) in &self.builtins.clone() {
            self.environment
                .write(|env| env.define(*builtin_name, Value::Builtin(builtin_fn.clone())));
        }

        for (name_symbol, class_def) in &self.std_classes.clone() {
            self.environment
                .write(|env| env.define(*name_symbol, Value::Class(class_def.clone())));

            if let Some(mod_entry) = self.modules.get_mut(&module.name) {
                mod_entry
                    .globals
                    .insert(*name_symbol, Value::Class(class_def.clone()));
            }
        }

        for &stmt_id in &module.body {
            match self.execute_statement(stmt_id, module.name) {
                Err(RuntimeError::Return(..)) => {}
                Err(e) => return Err(e),
                Ok(()) => {}
            }
        }

        Ok(())
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
            return Some((module_id, Value::Function(Arc::new(function.clone()))));
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
}
