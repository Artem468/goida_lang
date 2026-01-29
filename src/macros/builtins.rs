#[macro_export]
macro_rules! define_builtin {
    ($map:expr, $interner:expr, $self_inst:ident, $name:literal, ($args:ident) -> $out_type:ty $body:block) => {
        let sym = $interner.write().unwrap().get_or_intern($name);
        $map.insert(sym, BuiltinFn(std::sync::Arc::new(move |$self_inst, $args| {
            let res: $out_type = (|| $body)();
            res
        })));
    };
    
    ($map:expr, $interner:expr, $self_inst:ident, $name:literal, ($args:ident) $body:block) => {
        let sym = $interner.write().unwrap().get_or_intern($name);
        $map.insert(sym, BuiltinFn(std::sync::Arc::new(move |$self_inst, $args| {
            let _ = (|| $body)();
            Ok(Value::Empty)
        })));
    };
}

#[macro_export]
macro_rules! setup_builtins {
    ($self_name:ident, { $($name:literal ($args:ident) $(-> $res:ty)? $body:block)* }) => {
        impl Interpreter {
            pub fn define_builtins(&mut self) {
                let interner = Arc::clone(&self.interner);
                $(
                    define_builtin!(self.builtins, interner, $self_name, $name, ($args) $(-> $res)? $body);
                )*
            }
        }
    };
}
