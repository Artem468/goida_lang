use super::SourceFormatter;
use crate::parser::parser::token_source_text;
use crate::parser::syntax as syn;

pub(super) fn format_params(params: &[syn::Param]) -> String {
    params
        .iter()
        .map(|param| {
            let type_name = param
                .type_name
                .as_ref()
                .map(|ty| format!(": {ty}"))
                .unwrap_or_default();
            let default_value = param
                .default_value
                .as_ref()
                .map(|value| format!(" = {}", expr(value)))
                .unwrap_or_default();
            format!("{}{}{}", param.name, type_name, default_value)
        })
        .collect::<Vec<_>>()
        .join(", ")
}

pub(super) fn modifiers(visibility: Option<syn::Visibility>, is_static: bool) -> Vec<String> {
    let mut parts = Vec::new();
    if let Some(visibility) = visibility {
        parts.push(
            match visibility {
                syn::Visibility::Public => "public",
                syn::Visibility::Private => "private",
            }
            .to_string(),
        );
    }
    if is_static {
        parts.push("static".to_string());
    }
    parts
}

pub(super) fn catch_pattern(pattern: &Option<syn::CatchPattern>) -> String {
    match pattern {
        None => String::new(),
        Some(syn::CatchPattern::Text(name, _)) => format!(" (as {name})"),
        Some(syn::CatchPattern::Type(name, _)) => format!(" ({name})"),
        Some(syn::CatchPattern::TypeAndText {
            type_name,
            text_name,
            ..
        }) => format!(" ({type_name} as {text_name})"),
    }
}

pub(super) fn for_update(update: &syn::ForUpdate) -> String {
    match update {
        syn::ForUpdate::Assign { name, value, .. } => format!("{name} = {}", expr(value)),
        syn::ForUpdate::AssignTarget { target, value, .. } => {
            format!("{} = {}", expr(target), expr(value))
        }
        syn::ForUpdate::Compound {
            target, op, value, ..
        } => {
            format!("{} {} {}", expr(target), compound_op(*op), expr(value))
        }
        syn::ForUpdate::Expr(value) => expr(value),
    }
}

pub(super) fn compound_op(op: syn::CompoundOp) -> &'static str {
    match op {
        syn::CompoundOp::Add => "+=",
        syn::CompoundOp::Sub => "-=",
        syn::CompoundOp::Mul => "*=",
        syn::CompoundOp::Div => "/=",
        syn::CompoundOp::Mod => "%=",
    }
}

pub(super) fn expr(value: &syn::Expr) -> String {
    expr_with_parent_prec(value, 0, false)
}

pub(super) fn expr_with_parent_prec(value: &syn::Expr, parent_prec: u8, is_right: bool) -> String {
    let own_prec = expr_prec(value);
    let mut rendered = match &value.node {
        syn::ExprKind::Number(value) => value.to_string(),
        syn::ExprKind::Float(value) => value.to_string(),
        syn::ExprKind::Text(value) => string_literal(value),
        syn::ExprKind::Boolean(true) => "true".to_string(),
        syn::ExprKind::Boolean(false) => "false".to_string(),
        syn::ExprKind::Empty => "void".to_string(),
        syn::ExprKind::Identifier(name) => name.clone(),
        syn::ExprKind::Binary { op, left, right } => {
            let prec = binary_prec(*op);
            format!(
                "{} {} {}",
                expr_with_parent_prec(left, prec, false),
                binary_op(*op),
                expr_with_parent_prec(right, prec, true)
            )
        }
        syn::ExprKind::Unary { op, operand } => {
            format!(
                "{}{}",
                unary_op(*op),
                expr_with_parent_prec(operand, own_prec, false)
            )
        }
        syn::ExprKind::FunctionCall { function, args } => {
            format!(
                "{}({})",
                expr_with_parent_prec(function, own_prec, false),
                format_args(args)
            )
        }
        syn::ExprKind::MethodCall {
            object,
            method,
            args,
        } => {
            format!(
                "{}.{}({})",
                expr_with_parent_prec(object, own_prec, false),
                method,
                format_args(args)
            )
        }
        syn::ExprKind::PropertyAccess { object, property } => {
            format!(
                "{}.{}",
                expr_with_parent_prec(object, own_prec, false),
                property
            )
        }
        syn::ExprKind::Index { object, index } => {
            format!(
                "{}[{}]",
                expr_with_parent_prec(object, own_prec, false),
                expr(index)
            )
        }
        syn::ExprKind::ObjectCreation { class_name, args } => {
            format!("new {}({})", class_name, format_args(args))
        }
        syn::ExprKind::Lambda { params, body } => {
            let body = match body {
                syn::LambdaBody::Expr(value) => expr(value),
                syn::LambdaBody::Block(items, _) => {
                    let mut formatter = SourceFormatter::new(Vec::new());
                    formatter.output.push_str("{\n");
                    formatter.indent += 1;
                    formatter.items(items);
                    formatter.indent -= 1;
                    formatter.output.push('}');
                    formatter.finish()
                }
            };
            format!("lambda({}) => {}", format_params(params), body)
        }
        syn::ExprKind::MacroCall(call) => {
            format!("{}!{}", call.name, macro_call_args(call))
        }
    };

    if own_prec < parent_prec || (is_right && own_prec == parent_prec && own_prec < 8) {
        rendered = format!("({rendered})");
    }
    rendered
}

pub(super) fn expr_prec(expr: &syn::Expr) -> u8 {
    match &expr.node {
        syn::ExprKind::Binary { op, .. } => binary_prec(*op),
        syn::ExprKind::Unary { .. } => 6,
        syn::ExprKind::FunctionCall { .. }
        | syn::ExprKind::MethodCall { .. }
        | syn::ExprKind::PropertyAccess { .. }
        | syn::ExprKind::Index { .. } => 7,
        _ => 8,
    }
}

pub(super) fn binary_prec(op: syn::BinaryOp) -> u8 {
    match op {
        syn::BinaryOp::Or => 1,
        syn::BinaryOp::And => 2,
        syn::BinaryOp::Eq
        | syn::BinaryOp::Ne
        | syn::BinaryOp::Lt
        | syn::BinaryOp::Le
        | syn::BinaryOp::Gt
        | syn::BinaryOp::Ge => 3,
        syn::BinaryOp::Add | syn::BinaryOp::Sub => 4,
        syn::BinaryOp::Mul | syn::BinaryOp::Div | syn::BinaryOp::Mod => 5,
    }
}

pub(super) fn binary_op(op: syn::BinaryOp) -> &'static str {
    match op {
        syn::BinaryOp::Add => "+",
        syn::BinaryOp::Sub => "-",
        syn::BinaryOp::Mul => "*",
        syn::BinaryOp::Div => "/",
        syn::BinaryOp::Mod => "%",
        syn::BinaryOp::Eq => "==",
        syn::BinaryOp::Ne => "!=",
        syn::BinaryOp::Lt => "<",
        syn::BinaryOp::Le => "<=",
        syn::BinaryOp::Gt => ">",
        syn::BinaryOp::Ge => ">=",
        syn::BinaryOp::And => "and",
        syn::BinaryOp::Or => "or",
    }
}

pub(super) fn unary_op(op: syn::UnaryOp) -> &'static str {
    match op {
        syn::UnaryOp::Negative => "-",
        syn::UnaryOp::Not => "!",
    }
}

pub(super) fn format_args(args: &[syn::CallArg]) -> String {
    args.iter()
        .map(|arg| {
            if let Some(name) = &arg.name {
                format!("{name} = {}", expr(&arg.value))
            } else {
                expr(&arg.value)
            }
        })
        .collect::<Vec<_>>()
        .join(", ")
}

pub(super) fn macro_call_args(call: &syn::MacroCall) -> String {
    let (open, close) = match call.delimiter {
        syn::MacroDelimiter::Paren => ('(', ')'),
        syn::MacroDelimiter::Bracket => ('[', ']'),
        syn::MacroDelimiter::Brace => ('{', '}'),
    };
    let args = call
        .args
        .iter()
        .map(|token| token_source_text(&token.token))
        .collect::<Vec<_>>()
        .join(" ");
    format!("{open}{args}{close}")
}

pub(super) fn format_macro_matchers(matchers: &[syn::MacroMatcher]) -> String {
    matchers
        .iter()
        .map(|matcher| match matcher {
            syn::MacroMatcher::Token(token) => token_source_text(&token.token),
            syn::MacroMatcher::Fragment { name, kind } => {
                format!("${name}:{}", macro_fragment_name(*kind))
            }
            syn::MacroMatcher::Repeat {
                matcher,
                separator,
                op,
            } => format!(
                "$({}){}{}",
                format_macro_matchers(matcher),
                format_macro_tokens(separator),
                macro_repeat_op(*op)
            ),
        })
        .collect::<Vec<_>>()
        .join(" ")
}

pub(super) fn format_macro_template(template: &[syn::MacroTemplate]) -> String {
    template
        .iter()
        .map(|item| match item {
            syn::MacroTemplate::Token(token) => token_source_text(&token.token),
            syn::MacroTemplate::Variable(name) => format!("${name}"),
            syn::MacroTemplate::Delimited {
                delimiter,
                template,
                ..
            } => {
                let (open, close) = macro_delimiters(*delimiter);
                format!("{open}{}{close}", format_macro_template(template))
            }
            syn::MacroTemplate::Repeat {
                template,
                separator,
                op,
            } => format!(
                "$({}){}{}",
                format_macro_template(template),
                format_macro_tokens(separator),
                macro_repeat_op(*op)
            ),
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn format_macro_tokens(tokens: &[syn::MacroToken]) -> String {
    tokens
        .iter()
        .map(|token| token_source_text(&token.token))
        .collect::<Vec<_>>()
        .join(" ")
}

fn macro_fragment_name(kind: syn::MacroFragmentKind) -> &'static str {
    match kind {
        syn::MacroFragmentKind::Expr => "expr",
        syn::MacroFragmentKind::Ident => "ident",
        syn::MacroFragmentKind::Block => "block",
        syn::MacroFragmentKind::Stmt => "stmt",
    }
}

fn macro_repeat_op(op: syn::MacroRepeatOp) -> &'static str {
    match op {
        syn::MacroRepeatOp::ZeroOrMore => "*",
        syn::MacroRepeatOp::OneOrMore => "+",
    }
}

fn macro_delimiters(delimiter: syn::MacroDelimiter) -> (char, char) {
    match delimiter {
        syn::MacroDelimiter::Paren => ('(', ')'),
        syn::MacroDelimiter::Bracket => ('[', ']'),
        syn::MacroDelimiter::Brace => ('{', '}'),
    }
}

pub(super) fn string_literal(value: &str) -> String {
    format!("{value:?}")
}
