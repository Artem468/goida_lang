//! Source-language model and source paths.

pub use goida_model::{SharedInterner, SharedMut};
pub mod ast;
pub mod import_paths;

pub mod prelude {
    pub use crate::ast::prelude::*;
}
