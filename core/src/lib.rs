pub mod ast;
pub mod builtins;
pub mod interpreter;
pub mod r#macro;
pub mod parser;
pub mod shared;
pub mod traits;

use crate::interpreter::prelude::{Interpreter, SharedInterner};
use crate::traits::prelude::CoreOperations;
use lazy_static::lazy_static;
use std::sync::RwLock;
use string_interner::StringInterner;

lazy_static! {
    pub static ref INTERNER: SharedInterner = SharedInterner::new(StringInterner::new());
    pub static ref INTERPRETER: RwLock<Interpreter> =
        RwLock::new(Interpreter::new(INTERNER.clone()));
}
