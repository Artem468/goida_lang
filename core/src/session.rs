use crate::interpreter::prelude::{Interpreter, SharedInterner};
use crate::shared::SharedMut;
use crate::traits::prelude::CoreOperations;
use string_interner::StringInterner;

/// Isolated language session owning its interner and runtime state.
#[derive(Debug)]
pub struct Session {
    pub runtime: Interpreter,
}

impl Session {
    pub fn new() -> Self {
        let interner = SharedMut::new(StringInterner::new());
        let mut runtime = Interpreter::new(interner);
        runtime.define_builtins();
        Self { runtime }
    }

    pub fn interner(&self) -> SharedInterner {
        self.runtime.interner.clone()
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
