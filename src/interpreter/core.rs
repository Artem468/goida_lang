use crate::ast::prelude::{ClassDefinition, ErrorData};
use crate::ast::program::FieldData;
use crate::ast::source::SourceManager;
use crate::interpreter::prelude::{Environment, SharedInterner};
use crate::interpreter::structs::{Interpreter, Module, RuntimeError, Value};
use crate::parser::prelude::Parser;
use crate::shared::SharedMut;
use crate::traits::prelude::{
    CoreOperations, ExpressionEvaluator, InterpreterClasses, StatementExecutor,
};
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
use string_interner::{DefaultSymbol as Symbol, StringInterner};

impl CoreOperations for Interpreter {
    fn new(interner: SharedInterner) -> Self {
        Interpreter {
            std_classes: HashMap::new(),
            builtins: HashMap::new(),
            modules: HashMap::new(),
            interner,
            environment: SharedMut::new(Environment::new()),
            source_manager: SourceManager::new(),
        }
    }

    fn load_start_module(&mut self, main_module: Module) -> &mut Self {
        self.modules.insert(main_module.name, main_module);
        self
    }

    fn interpret(&mut self, module_id: Symbol) -> Result<(), RuntimeError> {
        let module = self.modules.get_mut(&module_id).unwrap().clone();
        self.load_imports(&module)?;

        for (class_name, class_def) in &module.classes {
            let class_value = Value::Class(class_def.clone());
            self.environment
                .write(|env| env.define(class_name.clone(), class_value.clone()));
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
                .write(|env| env.define(function_name.clone(), func_value.clone()));
            if let Some(mod_entry) = self.modules.get_mut(&module.name) {
                mod_entry.globals.insert(*function_name, func_value);
            }
        }

        for (builtin_name, builtin_fn) in &self.builtins.clone() {
            self.environment
                .write(|env| env.define(builtin_name.clone(), Value::Builtin(builtin_fn.clone())));
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

    fn resolve_symbol(&self, symbol: Symbol) -> Option<String> {
        self.interner
            .read(|i| i.resolve(symbol).map(|s| s.to_string()))
    }

    fn intern_string(&self, s: &str) -> Symbol {
        self.interner.write(|i| i.get_or_intern(s))
    }

    fn load_imports(&mut self, module: &Module) -> Result<(), RuntimeError> {
        for import in &module.imports {
            let item = &import.item;
            let path = module
                .arena
                .resolve_symbol(&self.interner, item.path)
                .unwrap();
            let relative_path = Path::new(&path);
            let module_dir = module.path.parent().unwrap_or_else(|| Path::new("."));
            let full_path = module_dir.join(relative_path).with_extension("goida");
            let file_stem = full_path
                .file_stem()
                .and_then(|s| s.to_str())
                .ok_or_else(|| {
                    RuntimeError::InvalidOperation(ErrorData::new(
                        import.span,
                        format!(
                            "Невозможно получить имя модуля из пути: {}",
                            full_path.display()
                        ),
                    ))
                })?;
            let code = std::fs::read_to_string(&full_path).map_err(|err| {
                RuntimeError::IOError(ErrorData::new(
                    import.span,
                    format!("{} | {err}", full_path.display()),
                ))
            })?;

            let parser = Parser::new(self.interner.clone(), file_stem, full_path.clone());

            // Пустой модуль для вывода данных о нем на случай ошибки
            let _module = parser.module.clone();

            match parser.parse(code.as_str()) {
                Ok(new_module) => {
                    let module_symbol = self.interner.write(|i| i.get_or_intern(file_stem));

                    if let Some(importer) = self.modules.get_mut(&module.name) {
                        if importer.globals.contains_key(&item.alias)
                            || importer.functions.contains_key(&item.alias)
                            || importer.classes.contains_key(&item.alias)
                        {
                            let alias_name = self.resolve_symbol(item.alias).unwrap_or_default();
                            return Err(RuntimeError::InvalidOperation(ErrorData::new(
                                import.span,
                                format!("Import alias '{}' is already used", alias_name),
                            )));
                        }
                        importer
                            .globals
                            .insert(item.alias, Value::Module(module_symbol));
                    }

                    if self.modules.contains_key(&module_symbol) {
                        continue;
                    }

                    self.modules.insert(module_symbol, new_module.clone());

                    self.load_imports(&new_module)?;

                    let previous_env = self.environment.clone();
                    self.environment = SharedMut::new(Environment::new());

                    for &stmt_id in &new_module.body {
                        self.execute_statement(stmt_id, module_symbol)?;
                    }

                    self.environment = previous_env;

                    for (_class_name, class_def) in &new_module.classes {
                        let class_def_with_module =
                            self.set_class_module(class_def.clone(), module_symbol);
                        if let Some(module) = self.modules.get_mut(&module_symbol) {
                            module.classes.insert(
                                class_def_with_module.read(|i| i.name),
                                class_def_with_module,
                            );
                        }
                    }

                    for (function_name, function_fn) in &new_module.functions {
                        let func_value = Value::Function(Arc::new(function_fn.clone()));
                        self.environment
                            .write(|env| env.define(function_name.clone(), func_value.clone()));
                        if let Some(module) = self.modules.get_mut(&module_symbol) {
                            module.globals.insert(*function_name, func_value);
                        }
                    }
                }
                Err(err) => {
                    self.modules.insert(_module.name, _module);
                    return Err(RuntimeError::ImportError(err));
                }
            }
        }
        Ok(())
    }

    fn get_class_for_value(&self, value: &Value) -> Option<SharedMut<ClassDefinition>> {
        let class_name = match value {
            Value::Text(_) => "Строка",
            Value::List(_) => "Список",
            Value::Array(_) => "Массив",
            Value::Dict(_) => "Словарь",
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
        let _name = self
            .modules
            .get(module_id)
            .unwrap()
            .path
            .canonicalize()
            .unwrap()
            .to_string_lossy()
            .to_string();
        _name.strip_prefix(r"\\?\").unwrap_or(&_name).to_string()
    }
}

impl Interpreter {
    pub(crate) fn resolve_import_alias_symbol(
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
                let file_stem = Path::new(&path).file_stem()?.to_str()?;
                return Some(self.interner.write(|i| i.get_or_intern(file_stem)));
            }
        }

        None
    }
}
