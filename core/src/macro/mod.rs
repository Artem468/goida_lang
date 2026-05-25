#[macro_export]
macro_rules! define_method {
    ($class:expr, $interner:expr, $($(@$flag:ident)+)? $name:expr => ($interp:pat, $args:pat, $span:pat) $body:block) => {
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

            let method = $crate::interpreter::prelude::BuiltinFn(std::sync::Arc::new(move |$interp, $args, $span| {
                    $body
                }));
            let aliases = $crate::builtins::catalog::method_names($name);
            if aliases.is_empty() {
                $class.add_method($interner.write(|i| i.get_or_intern($name)), vis, is_static, method);
            } else {
                for alias in aliases {
                    $class.add_method(
                        $interner.write(|i| i.get_or_intern(alias)),
                        vis.clone(),
                        is_static,
                        method.clone(),
                    );
                }
            }
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
    ($interpreter:expr, $interner:expr, $name:expr => ($interp:pat, $args:pat, $span:pat) $body:block) => {{
        let builtin = $crate::interpreter::prelude::BuiltinFn(std::sync::Arc::new(
            move |$interp, $args, $span| $body,
        ));
        let aliases = $crate::builtins::catalog::function_names($name);
        if aliases.is_empty() {
            $interpreter
                .builtins
                .insert($interner.write(|i| i.get_or_intern($name)), builtin);
        } else {
            for alias in aliases {
                $interpreter
                    .builtins
                    .insert($interner.write(|i| i.get_or_intern(alias)), builtin.clone());
            }
        }
    }};
}

#[macro_export]
macro_rules! define_builtin_macro {
    ($expander:expr, $name:literal => {
        $(($matcher:literal) => { $template:literal };)+
    }) => {{
        let rules = [
            $(($matcher, $template),)+
        ];
        $expander.register_builtin($name, &rules)?;
    }};

    ($expander:expr, $name:literal, $(($matcher:literal, $template:literal)),+ $(,)?) => {{
        let rules = [
            $(($matcher, $template),)+
        ];
        $expander.register_builtin($name, &rules)?;
    }};
}

#[macro_export]
macro_rules! runtime_error {
    ($variant:ident, $span:expr, $fmt:expr $(, $arg:expr)*) => {
        RuntimeError::$variant(ErrorData::new(
            $span,
            format!($fmt $(, $arg)*),
        ))
    };

    ($variant:ident, $span:expr, $fmt:expr $(, $arg:expr)* => $extra:expr) => {
        RuntimeError::$variant(
            ErrorData::new($span, format!($fmt $(, $arg)*)),
            $extra
        )
    };
}

#[macro_export]
macro_rules! bail_runtime {
    ($variant:ident, $span:expr, $fmt:expr $(, $arg:expr)*) => {
        Err(runtime_error!($variant, $span, $fmt $(, $arg)*))
    };
    ($variant:ident, $span:expr, $fmt:expr $(, $arg:expr)* => $extra:expr) => {
        Err(runtime_error!($variant, $span, $fmt $(, $arg)* => $extra))
    };
}

#[macro_export]
macro_rules! expect_args {
    ($args:expr, $n:expr, $span:expr, $name:expr) => {
        if $args.len() != $n {
            return $crate::bail_runtime!(
                InvalidOperation,
                $span,
                "{} ожидает {} аргументов, получено {}",
                $name,
                $n,
                $args.len()
            );
        }
    };
}
