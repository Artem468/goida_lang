use crate::ast::prelude::Span;
use crate::ast::program::MethodType;
use crate::interpreter::prelude::{BuiltinFn, CallArgValue, Interpreter, RuntimeError, Value};
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

impl PartialEq for MethodType {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (MethodType::User(a), MethodType::User(b)) => a == b,
            (MethodType::Native(a), MethodType::Native(b)) => Arc::ptr_eq(&a.0, &b.0),
            _ => false,
        }
    }
}

impl From<BuiltinFn> for MethodType {
    fn from(builtin: BuiltinFn) -> Self {
        MethodType::Native(Arc::new(builtin))
    }
}
