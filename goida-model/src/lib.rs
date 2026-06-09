//! Dependency-free shared compiler/runtime model.

mod shared;

pub use shared::{new_interner, SharedInterner, SharedMut};
pub use string_interner::DefaultSymbol as Symbol;
