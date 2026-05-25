use crate::ast::prelude::ErrorData;
use crate::interpreter::prelude::{Interpreter, RuntimeError, SharedInterner, Value};
use crate::{define_builtin, expect_args, runtime_error};

pub fn setup_bool_func(interpreter: &mut Interpreter, interner: &SharedInterner) {
    define_builtin!(interpreter, interner, crate::builtins::catalog::function::BOOLEAN.canonical => (_, arguments, span) {
        expect_args!(arguments, 1, span, "логический");

        let n: bool = arguments[0].value.clone().try_into()?;
        Ok(Value::Boolean(n))
    });
}
