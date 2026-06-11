use crate::ast::prelude::{ErrorData, Span};
use crate::interpreter::structs::{
    CallArgValue, Interpreter, Module, RuntimeClassDefinition, RuntimeError, Value,
};
use crate::runtime_error;
use crate::shared::SharedMut;
use crate::traits::prelude::{CoreOperations, InterpreterClasses};
use string_interner::DefaultSymbol as Symbol;

impl Interpreter {
    fn resolve_module_path(&self, current_module: &Module, parts: &[&str]) -> Option<Symbol> {
        let (first, rest) = parts.split_first()?;
        let first_symbol = self.intern_string(first);
        let mut module_id = self.resolve_import_alias_symbol(current_module, first_symbol)?;

        for part in rest {
            let part_symbol = self.intern_string(part);
            let (_, value) = self.resolve_module_member_value(module_id, part_symbol)?;
            match value {
                Value::Module(next_module_id) => module_id = next_module_id,
                _ => return None,
            }
        }
        Some(module_id)
    }

    pub(crate) fn resolve_class_for_creation(
        &self,
        class_name: Symbol,
        current_module_id: Symbol,
        span: Span,
    ) -> Result<(SharedMut<RuntimeClassDefinition>, Symbol), RuntimeError> {
        if let Some(Value::Class(class)) = self.environment.read(|env| env.get(&class_name)) {
            return Ok((class, current_module_id));
        }

        let name = self.resolve_symbol(class_name).unwrap_or_default();
        let parts = name.split('.').collect::<Vec<_>>();
        if parts.len() > 1 {
            let class_name = parts.last().copied().unwrap_or_default();
            let module_parts = &parts[..parts.len() - 1];
            let current = self.modules.get(&current_module_id).ok_or_else(|| {
                runtime_error!(InvalidOperation, span, "Current module is missing")
            })?;
            let module_id = self
                .resolve_module_path(current, module_parts)
                .ok_or_else(|| runtime_error!(InvalidOperation, span, "Module is missing"))?;
            let module = self
                .modules
                .get(&module_id)
                .ok_or_else(|| runtime_error!(InvalidOperation, span, "Module is missing"))?;
            let class_symbol = self.intern_string(class_name);
            let class = module
                .classes
                .get(&class_symbol)
                .cloned()
                .ok_or_else(|| runtime_error!(UndefinedVariable, span, "Class is missing"))?;
            return Ok((class, module.name));
        }

        self.modules
            .get(&current_module_id)
            .and_then(|module| module.classes.get(&class_name))
            .or_else(|| self.std_classes.get(&class_name))
            .cloned()
            .map(|class| (class, current_module_id))
            .ok_or_else(|| runtime_error!(UndefinedVariable, span, "Class '{}' is missing", name))
    }

    pub(crate) fn instantiate_class(
        &mut self,
        class: SharedMut<RuntimeClassDefinition>,
        definition_module: Symbol,
        arguments: Vec<CallArgValue>,
        span: Span,
    ) -> Result<Value, RuntimeError> {
        let class = self.set_class_module(class, definition_module);
        let instance = SharedMut::new(RuntimeClassDefinition::create_instance(class.clone()));
        self.adopt_value(&Value::Object(instance.clone()));
        if let Some(constructor) = class.read(|class| class.constructor.clone()) {
            let module = constructor.get_module().unwrap_or(definition_module);
            self.call_method(
                constructor,
                arguments,
                Value::Object(instance.clone()),
                module,
                span,
            )?;
        }

        let data_key = self.intern_string("__data");
        Ok(instance
            .write(|instance| instance.field_values.remove(&data_key))
            .unwrap_or(Value::Object(instance)))
    }
}
