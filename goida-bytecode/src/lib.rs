//! Register-bytecode model and compiler API.

pub(crate) use goida_hir as hir;
pub(crate) use goida_syntax::ast;

mod implementation;

pub use implementation::*;
