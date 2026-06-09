use crate::ast::prelude::Span;
use crate::interpreter::prelude::{
    BuiltinFn, CallArgValue, Interpreter, RuntimeError, RuntimeMethodType, Value,
};
use std::fmt;
use std::fmt::Debug;
use std::sync::Arc;

impl Debug for BuiltinFn {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "<Builtin function at {:p}>", self.0)
    }
}

impl std::ops::Deref for BuiltinFn {
    type Target =
        dyn Fn(&Interpreter, Vec<CallArgValue>, Span) -> Result<Value, RuntimeError> + Send + Sync;
    fn deref(&self) -> &Self::Target {
        &*self.0
    }
}

impl PartialEq for RuntimeMethodType {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (RuntimeMethodType::User(a), RuntimeMethodType::User(b)) => a == b,
            (RuntimeMethodType::Native(a), RuntimeMethodType::Native(b)) => Arc::ptr_eq(&a.0, &b.0),
            _ => false,
        }
    }
}

impl From<BuiltinFn> for RuntimeMethodType {
    fn from(builtin: BuiltinFn) -> Self {
        RuntimeMethodType::Native(Arc::new(builtin))
    }
}
