use crate::ast::prelude::{AstArena, FunctionDefinition, StmtId};
use crate::bytecode::{BytecodeModule, BytecodeSource};
use crate::hir::{CallableSignature, HirModule, HirSource};
use crate::interpreter::prelude::{CompiledModule, Module, SharedInterner, Value};
use crate::shared::SharedMut;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use string_interner::DefaultSymbol as Symbol;

impl Module {
    pub fn new(interner: &SharedInterner, name: &str, path: PathBuf) -> Self {
        let symbol = interner.write(|i| i.get_or_intern(name));

        Self {
            name: symbol,
            path,
            compiled: CompiledModule {
                arena: AstArena::new(),
                hir: HirModule::default(),
                bytecode: BytecodeModule::default(),
                functions: HashMap::new(),
                body: Vec::new(),
                imports: Vec::new(),
            },
            classes: HashMap::new(),
            modules: HashMap::new(),
            globals: HashMap::new(),
            global_slots: Vec::new(),
        }
    }

    pub(crate) fn initialize_global_slots(&mut self) {
        self.global_slots = self
            .hir
            .global_names
            .iter()
            .map(|name| self.globals.get(name).cloned().map(SharedMut::new))
            .collect();
    }

    pub(crate) fn global_slot(&self, slot: u32) -> Option<Value> {
        self.global_slots
            .get(slot as usize)
            .and_then(|value| value.as_ref())
            .map(|value| value.read(Clone::clone))
    }

    pub(crate) fn set_global_slot(&mut self, slot: u32, value: Value) {
        let Some(name) = self.hir.global_names.get(slot as usize).copied() else {
            return;
        };
        if self.global_slots.len() <= slot as usize {
            self.global_slots.resize(slot as usize + 1, None);
        }
        if let Some(target) = &self.global_slots[slot as usize] {
            target.write(|target| *target = value.clone());
        } else {
            self.global_slots[slot as usize] = Some(SharedMut::new(value.clone()));
        }
        self.globals.insert(name, value);
    }

    pub(crate) fn set_global(&mut self, name: Symbol, value: Value) {
        if let Some(slot) = self
            .hir
            .global_names
            .iter()
            .position(|candidate| *candidate == name)
        {
            if self.global_slots.len() <= slot {
                self.global_slots.resize(slot + 1, None);
            }
            if let Some(target) = &self.global_slots[slot] {
                target.write(|target| *target = value.clone());
            } else {
                self.global_slots[slot] = Some(SharedMut::new(value.clone()));
            }
        }
        self.globals.insert(name, value);
    }
}

impl HirSource for Module {
    fn arena(&self) -> &AstArena {
        &self.arena
    }

    fn body(&self) -> &[StmtId] {
        &self.body
    }

    fn global_names(&self) -> Vec<Symbol> {
        self.globals.keys().copied().collect()
    }

    fn functions(&self) -> Vec<Arc<FunctionDefinition>> {
        self.functions.values().cloned().collect()
    }

    fn functions_to_type_check(&self) -> Vec<Arc<FunctionDefinition>> {
        let mut functions = self.functions.values().cloned().collect::<Vec<_>>();
        for class in self.classes.values() {
            class.read(|class| {
                functions.extend(class.methods.values().filter_map(
                    |(_, _, method)| match method {
                        crate::interpreter::prelude::RuntimeMethodType::User(function) => {
                            Some(function.clone())
                        }
                        crate::interpreter::prelude::RuntimeMethodType::Native(_) => None,
                    },
                ));
                if let Some(crate::interpreter::prelude::RuntimeMethodType::User(constructor)) =
                    &class.constructor
                {
                    functions.push(constructor.clone());
                }
            });
        }
        functions
    }

    fn class_names(&self) -> Vec<Symbol> {
        self.classes.keys().copied().collect()
    }

    fn is_module_name(&self, name: Symbol) -> bool {
        self.modules.contains_key(&name)
    }

    fn callable_signatures(&self) -> Vec<CallableSignature> {
        let mut signatures = self
            .functions
            .values()
            .map(|function| CallableSignature {
                name: function.name,
                params: function.params.clone(),
                return_type: function.return_type,
                span: function.span,
            })
            .collect::<Vec<_>>();

        for statement in &self.arena.statements {
            match &statement.kind {
                crate::ast::prelude::StatementKind::FunctionDefinition(function) => {
                    signatures.push(CallableSignature {
                        name: function.name,
                        params: function.params.clone(),
                        return_type: function.return_type,
                        span: function.span,
                    });
                }
                crate::ast::prelude::StatementKind::NativeLibraryDefinition(library) => {
                    signatures.extend(library.functions.iter().map(|function| CallableSignature {
                        name: function.name,
                        params: function.params.clone(),
                        return_type: function.return_type,
                        span: function.span,
                    }));
                }
                _ => {}
            }
        }
        signatures
    }
}

impl BytecodeSource for Module {
    fn name(&self) -> Symbol {
        self.name
    }
}
