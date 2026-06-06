use crate::ast::prelude::{ClassDefinition, Span};
use crate::interpreter::prelude::{Interpreter, Module, SharedInterner};
use crate::parser::macro_expander::MacroExpander;
use crate::parser::prelude::ParseError;
use crate::shared::SharedMut;
use std::collections::{BTreeMap, HashMap};
use std::fmt::Write;
use string_interner::DefaultSymbol as Symbol;
use string_interner::StringInterner;

#[derive(Clone, Copy)]
pub struct BuiltinNames {
    pub canonical: &'static str,
    pub names: &'static [&'static str],
}

#[derive(Clone, Copy, Debug)]
pub enum BuiltinTypeSpec {
    Number,
    Text,
    Boolean,
    Float,
    Pointer,
    List,
    Array,
    Dict,
    Unit,
    Any,
    Function,
    Object,
    Module,
    Resource,
    Class,
}

#[derive(Clone, Copy)]
pub struct BuiltinTypeNames {
    pub names: &'static [&'static str],
    pub spec: BuiltinTypeSpec,
}

#[derive(Clone, Copy)]
pub struct BuiltinClass {
    pub names: BuiltinNames,
    pub type_spec: BuiltinTypeSpec,
}

#[derive(Clone, Copy)]
pub struct DateTimeUnit {
    pub millis: i64,
    pub add: BuiltinNames,
    pub subtract: BuiltinNames,
}

#[derive(Clone, Copy)]
pub struct BuiltinErrorClass {
    pub name: &'static str,
    pub base: Option<&'static str>,
}

type FunctionInstaller = fn(&mut Interpreter, &SharedInterner);
type ClassInstaller = fn(&SharedInterner) -> (Symbol, SharedMut<ClassDefinition>);
type MacroInstaller = fn(&mut MacroExpander) -> Result<(), ParseError>;

pub(crate) trait BuiltinInstallTarget {
    type Error;

    fn install_from(&mut self, registry: &BuiltinRegistry) -> Result<(), Self::Error>;
}

pub(crate) struct BuiltinParserTarget<'a> {
    pub module: &'a mut Module,
    pub interner: &'a SharedInterner,
}

macro_rules! declare_builtin_registry {
    (
        functions {
            $( $function:ident => (
                $function_canonical:literal,
                [$($function_name:literal),+ $(,)?],
                $function_install:path
            ); )*
        }
        classes {
            $( $class:ident => (
                $class_canonical:literal,
                [$($class_name:literal),+ $(,)?],
                $class_type:ident,
                $class_install:path
            ); )*
        }
        methods {
            $( $method:ident => (
                $method_canonical:literal,
                [$($method_name:literal),+ $(,)?]
            ); )*
        }
        macros {
            $( $macro_name:ident => (
                $macro_canonical:literal,
                [$($macro_alias:literal),+ $(,)?],
                $macro_install:path
            ); )*
        }
        types {
            $( ([$($type_name:literal),+ $(,)?], $type_spec:ident); )*
        }
        errors {
            $( $error:ident => ($error_name:literal, $error_base:expr); )*
        }
    ) => {
        pub(crate) mod function {
            use super::BuiltinNames;
            $(
                pub(crate) const $function: BuiltinNames = BuiltinNames {
                    canonical: $function_canonical,
                    names: &[$($function_name),+],
                };
            )*
        }

        pub(crate) mod class {
            use super::{BuiltinClass, BuiltinNames, BuiltinTypeSpec};
            $(
                pub(crate) const $class: BuiltinClass = BuiltinClass {
                    names: BuiltinNames {
                        canonical: $class_canonical,
                        names: &[$($class_name),+],
                    },
                    type_spec: BuiltinTypeSpec::$class_type,
                };
            )*
        }

        pub(crate) mod method {
            use super::BuiltinNames;
            $(
                pub(crate) const $method: BuiltinNames = BuiltinNames {
                    canonical: $method_canonical,
                    names: &[$($method_name),+],
                };
            )*
        }

        pub(crate) mod macros {
            use super::BuiltinNames;
            $(
                pub(crate) const $macro_name: BuiltinNames = BuiltinNames {
                    canonical: $macro_canonical,
                    names: &[$($macro_alias),+],
                };
            )*
        }

        const FUNCTIONS: &[BuiltinNames] = &[$(function::$function),*];
        const CLASSES: &[BuiltinClass] = &[$(class::$class),*];
        const METHODS: &[BuiltinNames] = &[$(method::$method),*];
        const MACROS: &[BuiltinNames] = &[$(macros::$macro_name),*];
        const TYPES: &[BuiltinTypeNames] = &[
            $(BuiltinTypeNames {
                names: &[$($type_name),+],
                spec: BuiltinTypeSpec::$type_spec,
            }),*
        ];
        const ERROR_CLASSES: &[BuiltinErrorClass] = &[
            $(BuiltinErrorClass {
                name: $error_name,
                base: $error_base,
            }),*
        ];
        const FUNCTION_INSTALLERS: &[FunctionInstaller] = &[$($function_install),*];
        const CLASS_INSTALLERS: &[ClassInstaller] = &[$($class_install),*];
        const MACRO_INSTALLERS: &[MacroInstaller] = &[$($macro_install),*];

        pub static BUILTINS: BuiltinRegistry = BuiltinRegistry {
            functions: FUNCTIONS,
            classes: CLASSES,
            methods: METHODS,
            macros: MACROS,
            types: TYPES,
            error_classes: ERROR_CLASSES,
            function_installers: FUNCTION_INSTALLERS,
            class_installers: CLASS_INSTALLERS,
            macro_installers: MACRO_INSTALLERS,
        };
    };
}

/// Single source of truth for built-in metadata and runtime installation.
pub struct BuiltinRegistry {
    functions: &'static [BuiltinNames],
    classes: &'static [BuiltinClass],
    methods: &'static [BuiltinNames],
    macros: &'static [BuiltinNames],
    types: &'static [BuiltinTypeNames],
    error_classes: &'static [BuiltinErrorClass],
    function_installers: &'static [FunctionInstaller],
    class_installers: &'static [ClassInstaller],
    macro_installers: &'static [MacroInstaller],
}

impl BuiltinRegistry {
    pub(crate) fn install<T: BuiltinInstallTarget>(&self, target: &mut T) -> Result<(), T::Error> {
        target.install_from(self)
    }

    pub const fn functions(&self) -> &'static [BuiltinNames] {
        self.functions
    }

    pub const fn classes(&self) -> &'static [BuiltinClass] {
        self.classes
    }

    pub const fn methods(&self) -> &'static [BuiltinNames] {
        self.methods
    }

    pub const fn macros(&self) -> &'static [BuiltinNames] {
        self.macros
    }

    pub const fn types(&self) -> &'static [BuiltinTypeNames] {
        self.types
    }

    pub(crate) const fn error_classes(&self) -> &'static [BuiltinErrorClass] {
        self.error_classes
    }

    pub fn function_names(&self, canonical: &str) -> &'static [&'static str] {
        names_for(self.functions, canonical)
    }

    pub fn method_names(&self, canonical: &str) -> &'static [&'static str] {
        names_for(self.methods, canonical)
    }

    pub fn macro_names(&self, canonical: &str) -> &'static [&'static str] {
        names_for(self.macros, canonical)
    }

    pub fn class_names(&self, canonical: &str) -> &'static [&'static str] {
        self.classes
            .iter()
            .find(|entry| entry.names.canonical == canonical)
            .map(|entry| entry.names.names)
            .unwrap_or(&[])
    }

    pub fn known_global_names(&self) -> impl Iterator<Item = &'static str> {
        self.functions
            .iter()
            .flat_map(|entry| entry.names.iter().copied())
            .chain(
                self.classes
                    .iter()
                    .flat_map(|entry| entry.names.names.iter().copied()),
            )
    }

    pub fn generate_markdown_docs(&self) -> String {
        let mut output = String::from("# Built-in entities\n\n");
        write_names_table(&mut output, "Functions", self.functions.iter().copied());
        write_names_table(&mut output, "Macros", self.macros.iter().copied());
        self.write_class_docs(&mut output);
        self.write_error_class_docs(&mut output);
        write_type_table(&mut output, self.types);
        output
    }

    fn write_class_docs(&self, output: &mut String) {
        output.push_str("## Classes\n\n");
        let interner = SharedMut::new(StringInterner::new());

        for install in self.class_installers {
            let (class_symbol, class) = install(&interner);
            let class_name = interner
                .read(|i| i.resolve(class_symbol).map(str::to_owned))
                .unwrap_or_default();
            let aliases = self.class_names(&class_name);
            let _ = writeln!(output, "### `{class_name}`\n");
            let _ = writeln!(output, "Aliases: {}\n", aliases.join(", "));
            output.push_str("| Method | Aliases | Static |\n|---|---|---|\n");

            let methods = class.read(|definition| {
                let mut methods = BTreeMap::new();
                for (symbol, (_, is_static, _)) in &definition.methods {
                    let Some(name) = interner.read(|i| i.resolve(*symbol).map(str::to_owned))
                    else {
                        continue;
                    };
                    let entry = self
                        .methods
                        .iter()
                        .find(|entry| entry.names.contains(&name.as_str()));
                    let canonical =
                        entry.map_or_else(|| name.clone(), |entry| entry.canonical.to_string());
                    let aliases = entry.map_or_else(
                        || vec![name],
                        |entry| {
                            entry
                                .names
                                .iter()
                                .map(|alias| (*alias).to_string())
                                .collect()
                        },
                    );
                    methods.insert(canonical, (aliases, *is_static));
                }
                methods
            });

            for (canonical, (aliases, is_static)) in methods {
                let _ = writeln!(
                    output,
                    "| `{canonical}` | {} | {} |",
                    aliases.join(", "),
                    if is_static { "yes" } else { "no" }
                );
            }
            output.push('\n');
        }
    }

    fn write_error_class_docs(&self, output: &mut String) {
        output.push_str("## Error Classes\n\n| Class | Base |\n|---|---|\n");
        for error in self.error_classes {
            let _ = writeln!(
                output,
                "| `{}` | {} |",
                error.name,
                error.base.map_or("-", |base| base)
            );
        }
        output.push('\n');
    }

    fn install_runtime(&self, interpreter: &mut Interpreter) {
        let interner = interpreter.interner.clone();
        let mut installed = std::collections::HashSet::new();
        for install in self.function_installers {
            if !installed.insert(*install as usize) {
                continue;
            }
            install(interpreter, &interner);
        }
        for install in self.class_installers {
            let (canonical, class) = install(&interner);
            self.register_class_aliases(interpreter, &interner, canonical, class);
        }
    }

    fn install_macros(&self, expander: &mut MacroExpander) -> Result<(), ParseError> {
        for install in self.macro_installers {
            install(expander)?;
        }
        Ok(())
    }

    fn register_class_aliases(
        &self,
        interpreter: &mut Interpreter,
        interner: &SharedInterner,
        canonical: Symbol,
        class: SharedMut<ClassDefinition>,
    ) {
        let canonical_name = interner
            .read(|i| i.resolve(canonical).map(str::to_owned))
            .unwrap_or_default();
        let aliases = self.class_names(&canonical_name);

        if aliases.is_empty() {
            interpreter.std_classes.insert(canonical, class);
            return;
        }

        for alias in aliases {
            interpreter
                .std_classes
                .insert(interner.write(|i| i.get_or_intern(alias)), class.clone());
        }
    }
}

impl BuiltinInstallTarget for Interpreter {
    type Error = std::convert::Infallible;

    fn install_from(&mut self, registry: &BuiltinRegistry) -> Result<(), Self::Error> {
        registry.install_runtime(self);
        Ok(())
    }
}

impl BuiltinInstallTarget for MacroExpander {
    type Error = ParseError;

    fn install_from(&mut self, registry: &BuiltinRegistry) -> Result<(), Self::Error> {
        registry.install_macros(self)
    }
}

impl BuiltinInstallTarget for BuiltinParserTarget<'_> {
    type Error = std::convert::Infallible;

    fn install_from(&mut self, registry: &BuiltinRegistry) -> Result<(), Self::Error> {
        for builtin in registry.types() {
            self.module.arena.register_builtin_type(
                self.interner,
                builtin.names,
                builtin.spec,
                None,
            );
        }
        for class in registry.classes() {
            self.module.arena.register_builtin_type(
                self.interner,
                class.names.names,
                class.type_spec,
                Some(class.names.canonical),
            );
        }

        let mut symbols = HashMap::<&str, Symbol>::new();
        for error in registry.error_classes() {
            let symbol = self.module.arena.intern_string(self.interner, error.name);
            self.module
                .arena
                .register_custom_type(self.interner, error.name);
            symbols.insert(error.name, symbol);
            let base = error.base.and_then(|base| symbols.get(base).copied());
            self.module.classes.entry(symbol).or_insert_with(|| {
                SharedMut::new(ClassDefinition::new_with_base(
                    symbol,
                    base,
                    Span::default(),
                ))
            });
        }
        Ok(())
    }
}

fn names_for(entries: &[BuiltinNames], canonical: &str) -> &'static [&'static str] {
    entries
        .iter()
        .find(|entry| entry.canonical == canonical)
        .map(|entry| entry.names)
        .unwrap_or(&[])
}

fn write_names_table(
    output: &mut String,
    title: &str,
    entries: impl IntoIterator<Item = BuiltinNames>,
) {
    let _ = writeln!(output, "## {title}\n\n| Canonical | Aliases |\n|---|---|");
    for entry in entries {
        let _ = writeln!(
            output,
            "| `{}` | {} |",
            entry.canonical,
            entry.names.join(", ")
        );
    }
    output.push('\n');
}

fn write_type_table(output: &mut String, entries: &[BuiltinTypeNames]) {
    let _ = writeln!(output, "## Types\n\n| Aliases | Type |\n|---|---|");
    for entry in entries {
        let _ = writeln!(
            output,
            "| {} | `{:?}` |",
            entry.names.join(", "),
            entry.spec
        );
    }
    output.push('\n');
}

include!("registry_declarations.rs");

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shared::SharedMut;
    use crate::traits::core::CoreOperations;
    use std::collections::HashSet;
    use string_interner::StringInterner;

    #[test]
    fn registry_names_are_unique_and_include_canonical_names() {
        for (kind, entries) in [
            ("function", BUILTINS.functions()),
            ("macro", BUILTINS.macros()),
        ] {
            validate_names(kind, entries, true);
        }
        validate_names("method", BUILTINS.methods(), false);
        validate_names(
            "class",
            &BUILTINS
                .classes()
                .iter()
                .map(|entry| entry.names)
                .collect::<Vec<_>>(),
            true,
        );
    }

    #[test]
    fn runtime_installers_cover_registered_functions_and_classes() {
        let mut interpreter = Interpreter::new(SharedMut::new(StringInterner::new()));
        BUILTINS.install(&mut interpreter).unwrap();

        let mut function_names = HashSet::new();
        for entry in BUILTINS.functions() {
            for name in entry.names {
                function_names.insert(*name);
                let symbol = interpreter.interner.write(|i| i.get_or_intern(name));
                assert!(
                    interpreter.builtins.contains_key(&symbol),
                    "missing function {name}"
                );
            }
        }
        assert_eq!(interpreter.builtins.len(), function_names.len());

        let mut class_names = HashSet::new();
        for entry in BUILTINS.classes() {
            for name in entry.names.names {
                class_names.insert(*name);
                let symbol = interpreter.interner.write(|i| i.get_or_intern(name));
                assert!(
                    interpreter.std_classes.contains_key(&symbol),
                    "missing class {name}"
                );
            }
        }
        assert_eq!(interpreter.std_classes.len(), class_names.len());

        for entry in BUILTINS.methods() {
            let installed = entry.names.iter().any(|name| {
                let symbol = interpreter.interner.write(|i| i.get_or_intern(name));
                interpreter
                    .std_classes
                    .values()
                    .any(|class| class.read(|definition| definition.methods.contains_key(&symbol)))
            });
            assert!(installed, "missing method {}", entry.canonical);
        }
    }

    #[test]
    fn documentation_is_generated_from_registry_aliases() {
        let documentation = BUILTINS.generate_markdown_docs();
        for entry in BUILTINS
            .functions()
            .iter()
            .chain(BUILTINS.methods())
            .chain(BUILTINS.macros())
        {
            assert!(documentation.contains(entry.canonical));
            for alias in entry.names {
                assert!(documentation.contains(alias));
            }
        }
        for error in BUILTINS.error_classes() {
            assert!(documentation.contains(error.name));
            if let Some(base) = error.base {
                assert!(documentation.contains(base));
            }
        }
    }

    fn validate_names(kind: &str, entries: &[BuiltinNames], require_unique_canonical: bool) {
        let mut canonical = HashSet::new();
        for entry in entries {
            if require_unique_canonical {
                assert!(
                    canonical.insert(entry.canonical),
                    "duplicate {kind} canonical name {}",
                    entry.canonical
                );
            }
            assert!(
                entry.names.contains(&entry.canonical),
                "{kind} {} does not include its canonical name",
                entry.canonical
            );
            assert_eq!(
                entry.names.iter().copied().collect::<HashSet<_>>().len(),
                entry.names.len(),
                "{} {} contains duplicate aliases",
                kind,
                entry.canonical
            );
        }
    }
}
