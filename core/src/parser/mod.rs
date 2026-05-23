mod builder;
mod builtin_errors;
mod imports;
mod lexer;
#[allow(clippy::module_inception)]
pub mod parser;
pub mod prelude;
pub mod structs;
mod syntax;
mod validation;

lalrpop_util::lalrpop_mod!(pub(crate) grammar, "/parser/grammar.rs");
