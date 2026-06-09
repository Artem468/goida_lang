use crate::ast::prelude::ErrorData;
use crate::builtins::registry::*;
use crate::interpreter::prelude::{Interpreter, RuntimeError, SharedInterner, Value};
use crate::{bail_runtime, define_builtin, expect_args, runtime_error};

pub fn setup_float_func(interpreter: &mut Interpreter, interner: &SharedInterner) {
    define_builtin!(interpreter, interner, function::FLOAT.canonical => (_, arguments, span) {
        expect_args!(arguments, 1, span, "дробь");
        let n: f64 = match arguments[0].value.clone().try_into() {
            Ok(i) => i,
            Err(err) => return bail_runtime!(InvalidOperation, span, "{}", err),
        };
        Ok(Value::Float(n))
    });
}
