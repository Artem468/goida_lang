use std::fmt;
use std::fmt::Debug;
use crate::interpreter::prelude::{BuiltinFn, Interpreter, RuntimeError, Value};

impl Debug for BuiltinFn {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "<Builtin function at {:p}>", self.0)
    }
}

impl std::ops::Deref for BuiltinFn {
    type Target = dyn Fn(&Interpreter, Vec<Value>) -> Result<Value, RuntimeError> + Send + Sync;
    fn deref(&self) -> &Self::Target {
        &*self.0
    }
}