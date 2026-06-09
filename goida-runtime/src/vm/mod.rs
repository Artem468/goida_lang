use crate::ast::prelude::{
    BinaryOperator, ErrorData, Span, Visibility,
};
use crate::bytecode::{Chunk, Instruction, Register, RegisterArg};
use crate::hir::Binding;
use crate::interpreter::prelude::{
    CallArgValue, Interpreter, RuntimeError, RuntimeFieldData
    , Value,
};
use crate::traits::prelude::{
    CoreOperations, InterpreterClasses, InterpreterFunctions, ValueOperations,
};
use crate::{bail_runtime, runtime_error};
use std::collections::HashSet;
use string_interner::DefaultSymbol as Symbol;

pub struct Vm<'a> {
    interpreter: &'a mut Interpreter,
    module: Symbol,
    locals: Vec<Option<Value>>,
    local_constants: HashSet<u32>,
}

impl<'a> Vm<'a> {
    pub fn new(interpreter: &'a mut Interpreter, module: Symbol) -> Self {
        Self {
            interpreter,
            module,
            locals: Vec::new(),
            local_constants: HashSet::new(),
        }
    }

    pub fn run(mut self, chunk: &Chunk) -> Result<(), RuntimeError> {
        self.run_chunk(chunk)
    }

    pub fn evaluate_compiled(
        interpreter: &'a mut Interpreter,
        module: Symbol,
        id: crate::ast::prelude::ExprId,
    ) -> Result<Value, RuntimeError> {
        let chunk = interpreter
            .modules
            .get(&module)
            .and_then(|module| module.bytecode.expressions.get(&id))
            .cloned()
            .ok_or_else(|| {
                runtime_error!(
                    InvalidOperation,
                    Span::default(),
                    "Compiled expression is missing"
                )
            })?;
        Self::new(interpreter, module).run_value(&chunk)
    }

    fn run_chunk(&mut self, chunk: &Chunk) -> Result<(), RuntimeError> {
        self.execute_chunk(chunk).map(|_| ())
    }

    fn run_value(&mut self, chunk: &Chunk) -> Result<Value, RuntimeError> {
        let registers = self.execute_chunk(chunk)?;
        Ok(chunk
            .result
            .and_then(|result| registers.get(result as usize).cloned())
            .unwrap_or(Value::Empty))
    }
}

include!("implementation/execute.rs");

impl<'a> Vm<'a> {
    fn get(registers: &[Value], register: Register) -> Value {
        registers[register as usize].clone()
    }

    fn set(registers: &mut [Value], register: Register, value: Value) {
        registers[register as usize] = value;
    }

    fn args(registers: &[Value], args: &[RegisterArg]) -> Vec<CallArgValue> {
        args.iter()
            .map(|arg| CallArgValue {
                name: arg.name,
                value: Self::get(registers, arg.register),
            })
            .collect()
    }

    fn set_local(&mut self, slot: usize, value: Value) {
        if self.locals.len() <= slot {
            self.locals.resize(slot + 1, None);
        }
        self.locals[slot] = Some(value);
    }

    fn load_identifier(&mut self, name: Symbol, span: Span) -> Result<Value, RuntimeError> {
        if let Some(value) = self.interpreter.environment.read(|env| env.get(&name)) {
            return self.interpreter.resolve_runtime_value(value, span);
        }
        let module =
            self.interpreter.modules.get(&self.module).ok_or_else(|| {
                runtime_error!(InvalidOperation, span, "Current module is missing")
            })?;
        if let Some(value) = module.globals.get(&name) {
            return self.interpreter.resolve_runtime_value(value.clone(), span);
        }
        if let Some(builtin) = self.interpreter.builtins.get(&name) {
            return Ok(Value::Builtin(builtin.clone()));
        }
        if let Some(module) = self.interpreter.resolve_import_alias_symbol(module, name) {
            return Ok(Value::Module(module));
        }
        let name = self.interpreter.resolve_symbol(name).unwrap_or_default();
        if let Some((module_name, member_name)) = name.split_once('.') {
            let module_symbol = self.interpreter.intern_string(module_name);
            let member_symbol = self.interpreter.intern_string(member_name);
            if let Some(module) = self
                .interpreter
                .resolve_import_alias_symbol(module, module_symbol)
            {
                if let Some((_, value)) = self
                    .interpreter
                    .resolve_module_member_value(module, member_symbol)
                {
                    return self.interpreter.resolve_runtime_value(value, span);
                }
            }
        }
        bail_runtime!(UndefinedVariable, span, "{}", name)
    }

    fn binary(
        &self,
        op: BinaryOperator,
        left: Value,
        right: Value,
        span: Span,
    ) -> Result<Value, RuntimeError> {
        match op {
            BinaryOperator::Add => self.interpreter.add_values(left, right, span),
            BinaryOperator::Sub => self.interpreter.subtract_values(left, right, span),
            BinaryOperator::Mul => self.interpreter.multiply_values(left, right, span),
            BinaryOperator::Div => self.interpreter.divide_values(left, right, span),
            BinaryOperator::Mod => self.interpreter.modulo_values(left, right, span),
            BinaryOperator::Eq => Ok(Value::Boolean(left == right)),
            BinaryOperator::Ne => Ok(Value::Boolean(left != right)),
            BinaryOperator::Gt => self.interpreter.compare_greater(left, right, span),
            BinaryOperator::Lt => self.interpreter.compare_less(left, right, span),
            BinaryOperator::Ge => self.interpreter.compare_greater_equal(left, right, span),
            BinaryOperator::Le => self.interpreter.compare_less_equal(left, right, span),
            BinaryOperator::And | BinaryOperator::Or => unreachable!(),
        }
    }

    fn read_index(&self, object: Value, index: Value, span: Span) -> Result<Value, RuntimeError> {
        match object {
            Value::List(values) => values.read(|values| {
                let index = index.resolve_index(values.len(), span)?;
                values
                    .get(index)
                    .cloned()
                    .ok_or_else(|| runtime_error!(InvalidOperation, span, "Index out of bounds"))
            }),
            Value::Array(values) => {
                let index = index.resolve_index(values.len(), span)?;
                values
                    .get(index)
                    .cloned()
                    .ok_or_else(|| runtime_error!(InvalidOperation, span, "Index out of bounds"))
            }
            Value::Dict(values) => values.read(|values| {
                values
                    .get(&self.interpreter.format_value(&index))
                    .cloned()
                    .ok_or_else(|| {
                        runtime_error!(InvalidOperation, span, "Dictionary key is missing")
                    })
            }),
            _ => bail_runtime!(TypeError, span, "Value cannot be indexed"),
        }
    }

    fn assign_index(
        &self,
        object: Value,
        index: Value,
        value: Value,
        span: Span,
    ) -> Result<(), RuntimeError> {
        match object {
            Value::List(values) => values.write(|values| {
                let index = index.resolve_index(values.len(), span)?;
                values[index] = value;
                Ok(())
            }),
            Value::Dict(values) => values.write(|values| {
                values.insert(self.interpreter.format_value(&index), value);
                Ok(())
            }),
            _ => bail_runtime!(TypeError, span, "Value cannot be assigned by index"),
        }
    }

    fn read_property(
        &mut self,
        object: Result<Value, RuntimeError>,
        property: Symbol,
        receiver_is_this: bool,
        receiver_name: Option<Symbol>,
        span: Span,
    ) -> Result<Value, RuntimeError> {
        match object {
            Ok(Value::Module(module)) => self
                .interpreter
                .resolve_module_member_value(module, property)
                .map(|(_, value)| value)
                .ok_or_else(|| runtime_error!(UndefinedVariable, span, "Module member is missing")),
            Ok(Value::Object(instance)) => {
                let external = !receiver_is_this && self.interpreter.method_depth == 0;
                let field = instance.read(|instance| {
                    if !instance.is_field_accessible(&property, external) {
                        return None;
                    }
                    if let Some(value) = instance.field_values.get(&property) {
                        Some(Ok(value.clone()))
                    } else {
                        instance.get_field(&property).cloned().map(Err)
                    }
                });
                match field {
                    Some(Ok(value)) => Ok(value),
                    Some(Err(Some(expr))) => {
                        Vm::evaluate_compiled(self.interpreter, self.module, expr)
                    }
                    Some(Err(None)) => Ok(Value::Empty),
                    None => bail_runtime!(InvalidOperation, span, "Property is not accessible"),
                }
            }
            Ok(Value::Class(class)) => {
                let field = class.read(|class| class.fields.get(&property).cloned());
                match field {
                    Some((Visibility::Private, _, _)) if !receiver_is_this => {
                        bail_runtime!(InvalidOperation, span, "Property is private")
                    }
                    Some((_, false, _)) => {
                        bail_runtime!(InvalidOperation, span, "Property is not static")
                    }
                    Some((_, true, RuntimeFieldData::Value(value))) => Ok(value.read(Clone::clone)),
                    Some((_, true, RuntimeFieldData::Expression(_))) => {
                        bail_runtime!(InvalidOperation, span, "Static property is not initialized")
                    }
                    None => bail_runtime!(UndefinedVariable, span, "Property is missing"),
                }
            }
            Err(error) if receiver_name.is_none() => Err(error),
            _ => bail_runtime!(InvalidOperation, span, "Property receiver is invalid"),
        }
    }

    fn assign_property(
        &self,
        object: Value,
        property: Symbol,
        value: Value,
        receiver_is_this: bool,
        span: Span,
    ) -> Result<(), RuntimeError> {
        let Value::Object(instance) = object else {
            return bail_runtime!(TypeMismatch, span, "Expected object");
        };
        let external = !receiver_is_this && self.interpreter.method_depth == 0;
        instance.write(|instance| {
            if !instance.is_field_accessible(&property, external) {
                return bail_runtime!(InvalidOperation, span, "Property is not accessible");
            }
            instance.set_field_value(property, value);
            Ok(())
        })
    }

    fn call_method(
        &mut self,
        target: Value,
        method: Symbol,
        args: Vec<CallArgValue>,
        receiver_is_this: bool,
        span: Span,
    ) -> Result<Value, RuntimeError> {
        if let Some(class) = self.interpreter.get_class_for_value(&target) {
            if let Some((visibility, is_static, method_type)) = class.read(|class| {
                class
                    .methods
                    .get(&method)
                    .map(|(v, s, m)| (v.clone(), *s, m.clone()))
            }) {
                if matches!(target, Value::Class(_)) && !is_static {
                    return bail_runtime!(
                        InvalidOperation,
                        span,
                        "Instance method needs an object"
                    );
                }
                if !receiver_is_this && matches!(visibility, Visibility::Private) {
                    return bail_runtime!(InvalidOperation, span, "Method is private");
                }
                let receiver = if is_static { Value::Empty } else { target };
                let module = method_type.get_module().unwrap_or(self.module);
                return self
                    .interpreter
                    .call_method(method_type, args, receiver, module, span);
            }
        }
        if let Value::Module(module) = target {
            if let Some((definition_module, value)) =
                self.interpreter.resolve_module_member_value(module, method)
            {
                return match value {
                    Value::Function(function) => {
                        self.interpreter
                            .call_function(function, args, definition_module, span)
                    }
                    Value::Builtin(function) => function(self.interpreter, args, span),
                    Value::Class(class) => {
                        self.interpreter
                            .instantiate_class(class, definition_module, args, span)
                    }
                    _ => bail_runtime!(UndefinedFunction, span, "Module member is not callable"),
                };
            }
        }
        bail_runtime!(UndefinedMethod, span, "Method is missing")
    }
}

#[cfg(test)]
mod tests;
