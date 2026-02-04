#[macro_export]
macro_rules! define_builtin {
    ($map:expr, $interner:expr, $self_inst:ident, $name:literal, ($args:ident, $span_arg:ident) -> $out_type:ty $body:block) => {
        let sym = $interner.write(|i| i.get_or_intern($name));

        $map.insert(sym, BuiltinFn(std::sync::Arc::new(move |$self_inst, $args, $span_arg| {
            let res: $out_type = (|| $body)();
            res
        })));
    };

    ($map:expr, $interner:expr, $self_inst:ident, $name:literal, ($args:ident, $span_arg:ident) $body:block) => {
        let sym = $interner.write(|i| i.get_or_intern($name));

        $map.insert(sym, BuiltinFn(std::sync::Arc::new(move |$self_inst, $args, $span_arg| {
            let _ = (|| $body)();
            Ok(Value::Empty)
        })));
    };
}
#[macro_export]
macro_rules! setup_builtins {
    ($self_name:ident, { $($name:literal ($args:ident, $span_ident:ident) $(-> $res:ty)? $body:block)* }) => {
        impl Interpreter {
            pub fn define_builtins(&mut self) {
                let interner = self.interner.clone();
                $(
                    define_builtin!(self.builtins, interner, $self_name, $name, ($args, $span_ident) $(-> $res)? $body);
                )*
            }
        }
    };
}