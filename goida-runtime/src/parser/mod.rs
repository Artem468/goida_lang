mod builder;
mod formatter;
mod imports;
pub(crate) mod lexer;
pub(crate) mod macro_expander;
#[allow(clippy::module_inception)]
pub mod parser;
pub mod prelude;
pub mod structs;
pub(crate) mod syntax;
mod validation;

lalrpop_util::lalrpop_mod!(pub(crate) grammar, "/parser/grammar.rs");
