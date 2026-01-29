use crate::ast::prelude::AstArena;
use crate::interpreter::prelude::{Module, SharedInterner};
use std::collections::HashMap;
use std::path::PathBuf;

impl Module {
    pub fn new(interner: &SharedInterner, name: &str, path: PathBuf) -> Self {
        let symbol = interner.write().expect("Can't lock interner").get_or_intern(name);

        Self {
            name: symbol,
            path,
            arena: AstArena::new(),
            functions: HashMap::new(),
            classes: HashMap::new(),
            body: Vec::new(),
            imports: Vec::new(),
            globals: HashMap::new(),
        }
    }
}