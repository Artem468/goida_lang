#![allow(clippy::duplicate_mod)]

#[cfg(test)]
#[path = "../algorithms_test.rs"]
mod algorithms_test;
#[cfg(test)]
#[path = "../bilingual_builtins_test.rs"]
mod bilingual_builtins_test;
#[cfg(test)]
#[path = "../coverage_syntax_test.rs"]
mod coverage_syntax_test;
#[cfg(test)]
#[path = "../examples_test.rs"]
mod examples_test;
#[cfg(test)]
#[path = "../imports_and_builtin_test.rs"]
mod imports_and_builtin_test;
#[cfg(test)]
#[path = "../macro_expander_test.rs"]
mod macro_expander_test;
#[cfg(test)]
#[path = "../package_manager_test.rs"]
mod package_manager_test;
#[cfg(test)]
#[path = "../syntax_test.rs"]
mod syntax_test;
