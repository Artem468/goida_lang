//! Resolved high-level representation and visitor API.

pub(crate) use goida_syntax::ast;

mod implementation;
mod model;
mod type_check;

pub use implementation::*;
pub use model::*;
pub use type_check::*;
