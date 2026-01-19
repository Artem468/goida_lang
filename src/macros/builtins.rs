#[macro_export]
macro_rules! define_builtin {
    ($map:expr, $name:literal, ($args:ident) -> $out_type:ty $body:block) => {
        $map.insert($name.to_string(), std::sync::Arc::new(move |_interpreter, $args| {
            let res: $out_type = (|| $body)();
            res
        }));
    };
    
    ($map:expr, $name:literal, ($args:ident) $body:block) => {
        $map.insert($name.to_string(), std::sync::Arc::new(move |_interpreter, $args| {
            let _ = (|$args: Vec<Value>| $body)($args);
            Ok(Value::Empty)
        }));
    };
}

#[macro_export]
macro_rules! setup_builtins {
    ($($name:literal ($args:ident) $(-> $res:ty)? $body:block)* ) => {
        impl Interpreter {
            pub fn define_builtins(&mut self) {
                let b = &mut self.builtins;
                $(
                    define_builtin!(b, $name, ($args) $(-> $res)? $body);
                )*
            }
        }
    };
}