use crate::builtins::registry::BUILTINS;
use crate::diagnostics::DiagnosticLanguage;
use crate::interpreter::heap::CollectionStats;
use crate::interpreter::prelude::{Interpreter, Module, RuntimeError, SharedInterner};
use crate::traits::prelude::CoreOperations;

/// Isolated language session owning its interner and runtime state.
#[derive(Debug)]
pub struct Session {
    runtime: Interpreter,
}

impl Session {
    pub fn new() -> Self {
        let interner = goida_model::new_interner();
        let mut runtime = Interpreter::new(interner);
        match BUILTINS.install(&mut runtime) {
            Ok(()) => {}
            Err(error) => match error {},
        }
        Self { runtime }
    }

    pub fn interner(&self) -> SharedInterner {
        self.runtime.interner.clone()
    }

    /// Sets arguments exposed to source code through `Система::аргументы()`.
    pub fn set_script_args(&mut self, args: Vec<String>) {
        self.runtime.script_args = args;
    }

    pub fn set_diagnostic_language(&mut self, language: DiagnosticLanguage) {
        self.runtime.diagnostic_language = language;
    }

    pub fn diagnostic_language(&self) -> DiagnosticLanguage {
        self.runtime.diagnostic_language
    }

    /// Executes an already parsed and lowered module tree.
    pub fn execute(&mut self, module: Module) -> Result<(), RuntimeError> {
        let module_id = module.name;
        self.runtime.load_start_module(module);
        self.runtime.interpret(module_id)
    }

    /// Keeps a partial module available for source-aware diagnostics.
    pub fn register_diagnostic_module(&mut self, module: Module) {
        self.runtime.modules.insert(module.name, module);
    }

    /// Read-only access for diagnostics and embedding integrations.
    pub fn runtime(&self) -> &Interpreter {
        &self.runtime
    }

    /// Runs cycle collection and returns heap statistics.
    pub fn collect_cycles(&self) -> CollectionStats {
        self.runtime.collect_cycles()
    }
}

impl Default for Session {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::Session;

    #[test]
    fn sessions_own_independent_interners_and_runtimes() {
        let first = Session::new();
        let second = Session::new();
        let first_interner = first.interner();
        let second_interner = second.interner();

        assert!(!first_interner.ptr_eq(&second_interner));

        first_interner.write(|interner| {
            interner.get_or_intern("__only_in_first_session__");
        });
        assert!(second_interner
            .read(|interner| interner.get("__only_in_first_session__"))
            .is_none());
    }
}
