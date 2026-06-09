//! Source-language model, formatting and source paths.

pub use goida_model::{SharedInterner, SharedMut};
pub mod ast;
pub mod formatter;
pub mod import_paths;

pub mod prelude {
    pub use crate::ast::prelude::*;
    pub use crate::formatter::format_source;
}
