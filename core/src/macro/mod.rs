#[macro_export]
macro_rules! define_method {
    ($class:expr, $interner:expr, $($(@$flag:ident)+)? $name:literal => ($interp:pat, $args:pat, $span:pat) $body:block) => {
        {
            let vis = {
                #[allow(unused_mut)]
                let mut v = $crate::ast::prelude::Visibility::Public;
                $( $(
                    if stringify!($flag) == "private" { v = $crate::ast::prelude::Visibility::Private; }
                )+ )?
                v
            };

            let is_static = {
                #[allow(unused_mut)]
                let mut s = false;
                $( $(
                    if stringify!($flag) == "static" { s = true; }
                )+ )?
                s
            };

            $class.add_method(
                $interner.write(|i| i.get_or_intern($name)),
                vis,
                is_static,
                $crate::interpreter::prelude::BuiltinFn(std::sync::Arc::new(move |$interp, $args, $span| {
                    $body
                })),
            );
        }
    };
}

#[macro_export]
macro_rules! define_constructor {
    ($class:expr, ($interp:pat, $args:pat, $span:pat) $body:block) => {
        $class.set_constructor($crate::interpreter::prelude::BuiltinFn(
            std::sync::Arc::new(move |$interp, $args, $span| $body),
        ));
    };
}

#[macro_export]
macro_rules! define_builtin {
    ($interpreter:expr, $interner:expr, $name:literal => ($interp:pat, $args:pat, $span:pat) $body:block) => {
        $interpreter.builtins.insert(
            $interner.write(|i| i.get_or_intern($name)),
            $crate::interpreter::prelude::BuiltinFn(std::sync::Arc::new(
                move |$interp, $args, $span| $body,
            )),
        );
    };
}
