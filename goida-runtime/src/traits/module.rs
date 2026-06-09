use crate::ast::prelude::{AstArena, FunctionDefinition, StmtId};
use crate::bytecode::{BytecodeModule, BytecodeSource};
use crate::hir::{HirModule, HirSource};
use crate::interpreter::prelude::{Module, SharedInterner};
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
            arena: AstArena::new(),
            hir: HirModule::default(),
            bytecode: BytecodeModule::default(),
            functions: HashMap::new(),
            classes: HashMap::new(),
            body: Vec::new(),
            imports: Vec::new(),
            modules: HashMap::new(),
            globals: HashMap::new(),
        }
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

    fn class_names(&self) -> Vec<Symbol> {
        self.classes.keys().copied().collect()
    }

    fn module_names(&self) -> Vec<Symbol> {
        self.modules.keys().copied().collect()
    }
}

impl BytecodeSource for Module {
    fn name(&self) -> Symbol {
        self.name
    }

    fn arena(&self) -> &AstArena {
        &self.arena
    }

    fn body(&self) -> &[StmtId] {
        &self.body
    }

    fn functions_to_compile(&self) -> Vec<Arc<FunctionDefinition>> {
        let mut functions: Vec<_> = self.functions.values().cloned().collect();
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
}
