use super::SourceFormatter;
use crate::parser::lexer::Token;
use crate::parser::parser::token_source_text;
use crate::parser::structs::FormatLanguage;
use crate::parser::syntax as syn;

pub(super) fn format_params(params: &[syn::Param], language: FormatLanguage) -> String {
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
                .map(|value| format!(" = {}", expr(value, language)))
                .unwrap_or_default();
            format!("{}{}{}", param.name, type_name, default_value)
        })
        .collect::<Vec<_>>()
        .join(", ")
}

pub(super) fn modifiers(
    visibility: Option<syn::Visibility>,
    is_static: bool,
    language: FormatLanguage,
) -> Vec<String> {
    let mut parts = Vec::new();
    if let Some(visibility) = visibility {
        parts.push(
            match visibility {
                syn::Visibility::Public => language.select("public", "публичный"),
                syn::Visibility::Private => language.select("private", "приватный"),
            }
            .to_string(),
        );
    }
    if is_static {
        parts.push(language.select("static", "статичный").to_string());
    }
    parts
}

pub(super) fn catch_pattern(
    pattern: &Option<syn::CatchPattern>,
    language: FormatLanguage,
) -> String {
    match pattern {
        None => String::new(),
        Some(syn::CatchPattern::Text(name, _)) => {
            format!(" ({} {name})", language.select("as", "как"))
        }
        Some(syn::CatchPattern::Type(name, _)) => format!(" ({name})"),
        Some(syn::CatchPattern::TypeAndText {
            type_name,
            text_name,
            ..
        }) => format!(
            " ({type_name} {} {text_name})",
            language.select("as", "как")
        ),
    }
}

pub(super) fn for_update(update: &syn::ForUpdate, language: FormatLanguage) -> String {
    match update {
        syn::ForUpdate::Assign { name, value, .. } => {
            format!("{name} = {}", expr(value, language))
        }
        syn::ForUpdate::AssignTarget { target, value, .. } => {
            format!("{} = {}", expr(target, language), expr(value, language))
        }
        syn::ForUpdate::Compound {
            target, op, value, ..
        } => {
            format!(
                "{} {} {}",
                expr(target, language),
                compound_op(*op),
                expr(value, language)
            )
        }
        syn::ForUpdate::Expr(value) => expr(value, language),
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

pub(super) fn expr(value: &syn::Expr, language: FormatLanguage) -> String {
    expr_with_parent_prec(value, 0, false, language)
}

pub(super) fn expr_with_parent_prec(
    value: &syn::Expr,
    parent_prec: u8,
    is_right: bool,
    language: FormatLanguage,
) -> String {
    let own_prec = expr_prec(value);
    let mut rendered = match &value.node {
        syn::ExprKind::Number(value) => value.to_string(),
        syn::ExprKind::Float(value) => value.to_string(),
        syn::ExprKind::Text(value) => string_literal(value),
        syn::ExprKind::Boolean(true) => language.select("true", "истина").to_string(),
        syn::ExprKind::Boolean(false) => language.select("false", "ложь").to_string(),
        syn::ExprKind::Empty => language.select("void", "пустота").to_string(),
        syn::ExprKind::Identifier(name) => name.clone(),
        syn::ExprKind::Binary { op, left, right } => {
            let prec = binary_prec(*op);
            format!(
                "{} {} {}",
                expr_with_parent_prec(left, prec, false, language),
                binary_op(*op, language),
                expr_with_parent_prec(right, prec, true, language)
            )
        }
        syn::ExprKind::Unary { op, operand } => {
            format!(
                "{}{}",
                unary_op(*op),
                expr_with_parent_prec(operand, own_prec, false, language)
            )
        }
        syn::ExprKind::FunctionCall { function, args } => {
            format!(
                "{}({})",
                expr_with_parent_prec(function, own_prec, false, language),
                format_args(args, language)
            )
        }
        syn::ExprKind::MethodCall {
            object,
            method,
            args,
        } => {
            format!(
                "{}.{}({})",
                expr_with_parent_prec(object, own_prec, false, language),
                method,
                format_args(args, language)
            )
        }
        syn::ExprKind::PropertyAccess { object, property } => {
            format!(
                "{}.{}",
                expr_with_parent_prec(object, own_prec, false, language),
                property
            )
        }
        syn::ExprKind::Index { object, index } => {
            format!(
                "{}[{}]",
                expr_with_parent_prec(object, own_prec, false, language),
                expr(index, language)
            )
        }
        syn::ExprKind::ObjectCreation { class_name, args } => {
            format!(
                "{} {}({})",
                language.select("new", "новый"),
                class_name,
                format_args(args, language)
            )
        }
        syn::ExprKind::Lambda { params, body } => {
            let body = match body {
                syn::LambdaBody::Expr(value) => expr(value, language),
                syn::LambdaBody::Block(items, _) => {
                    let mut formatter = SourceFormatter::new(Vec::new(), language);
                    formatter.output.push_str("{\n");
                    formatter.indent += 1;
                    formatter.items(items);
                    formatter.indent -= 1;
                    formatter.output.push('}');
                    formatter.finish()
                }
            };
            format!("lambda({}) => {}", format_params(params, language), body)
        }
        syn::ExprKind::MacroCall(call) => {
            format!("{}!{}", call.name, macro_call_args(call, language))
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

pub(super) fn binary_op(op: syn::BinaryOp, language: FormatLanguage) -> &'static str {
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
        syn::BinaryOp::And => language.select("and", "и"),
        syn::BinaryOp::Or => language.select("or", "или"),
    }
}

pub(super) fn unary_op(op: syn::UnaryOp) -> &'static str {
    match op {
        syn::UnaryOp::Negative => "-",
        syn::UnaryOp::Not => "!",
    }
}

pub(super) fn format_args(args: &[syn::CallArg], language: FormatLanguage) -> String {
    args.iter()
        .map(|arg| {
            if let Some(name) = &arg.name {
                format!("{name} = {}", expr(&arg.value, language))
            } else {
                expr(&arg.value, language)
            }
        })
        .collect::<Vec<_>>()
        .join(", ")
}

pub(super) fn macro_call_args(call: &syn::MacroCall, language: FormatLanguage) -> String {
    let (open, close) = match call.delimiter {
        syn::MacroDelimiter::Paren => ('(', ')'),
        syn::MacroDelimiter::Bracket => ('[', ']'),
        syn::MacroDelimiter::Brace => ('{', '}'),
    };
    let args = call
        .args
        .iter()
        .map(|token| localized_token_text(&token.token, language))
        .collect::<Vec<_>>()
        .join(" ");
    format!("{open}{args}{close}")
}

pub(super) fn format_macro_matchers(
    matchers: &[syn::MacroMatcher],
    language: FormatLanguage,
) -> String {
    matchers
        .iter()
        .map(|matcher| match matcher {
            syn::MacroMatcher::Token(token) => localized_token_text(&token.token, language),
            syn::MacroMatcher::Fragment { name, kind } => {
                format!("${name}:{}", macro_fragment_name(*kind, language))
            }
            syn::MacroMatcher::Repeat {
                matcher,
                separator,
                op,
            } => format!(
                "$({}){}{}",
                format_macro_matchers(matcher, language),
                format_macro_tokens(separator, language),
                macro_repeat_op(*op)
            ),
        })
        .collect::<Vec<_>>()
        .join(" ")
}

pub(super) fn format_macro_template(
    template: &[syn::MacroTemplate],
    language: FormatLanguage,
) -> String {
    template
        .iter()
        .map(|item| match item {
            syn::MacroTemplate::Token(token) => localized_token_text(&token.token, language),
            syn::MacroTemplate::Variable(name) => format!("${name}"),
            syn::MacroTemplate::Delimited {
                delimiter,
                template,
                ..
            } => {
                let (open, close) = macro_delimiters(*delimiter);
                format!("{open}{}{close}", format_macro_template(template, language))
            }
            syn::MacroTemplate::Repeat {
                template,
                separator,
                op,
            } => format!(
                "$({}){}{}",
                format_macro_template(template, language),
                format_macro_tokens(separator, language),
                macro_repeat_op(*op)
            ),
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn format_macro_tokens(tokens: &[syn::MacroToken], language: FormatLanguage) -> String {
    tokens
        .iter()
        .map(|token| localized_token_text(&token.token, language))
        .collect::<Vec<_>>()
        .join(" ")
}

fn macro_fragment_name(kind: syn::MacroFragmentKind, language: FormatLanguage) -> &'static str {
    match kind {
        syn::MacroFragmentKind::Expr => language.select("expr", "выр"),
        syn::MacroFragmentKind::Ident => language.select("ident", "имя"),
        syn::MacroFragmentKind::Block => language.select("block", "блок"),
        syn::MacroFragmentKind::Stmt => language.select("stmt", "инстр"),
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

fn localized_token_text(token: &Token, language: FormatLanguage) -> String {
    let keyword = match token {
        Token::KwImport => Some(("import", "подключить")),
        Token::KwFrom => Some(("from", "из")),
        Token::KwFunction => Some(("function", "функция")),
        Token::KwLibrary => Some(("library", "библиотека")),
        Token::KwVariable => Some(("variable", "переменная")),
        Token::KwClass => Some(("class", "класс")),
        Token::KwConstructor => Some(("constructor", "конструктор")),
        Token::KwPublic => Some(("public", "публичный")),
        Token::KwPrivate => Some(("private", "приватный")),
        Token::KwStatic => Some(("static", "статичный")),
        Token::KwConst => Some(("const", "константа")),
        Token::KwIf => Some(("if", "если")),
        Token::KwElse => Some(("else", "иначе")),
        Token::KwWhile => Some(("while", "пока")),
        Token::KwFor => Some(("for", "для")),
        Token::KwThread => Some(("thread", "поток")),
        Token::KwTry => Some(("try", "попробовать")),
        Token::KwCatch => Some(("catch", "перехватить")),
        Token::KwRaise => Some(("raise", "выбросить")),
        Token::KwAs => Some(("as", "как")),
        Token::KwNew => Some(("new", "новый")),
        Token::KwReturn => Some(("return", "вернуть")),
        Token::KwAnd => Some(("and", "и")),
        Token::KwOr => Some(("or", "или")),
        Token::True => Some(("true", "истина")),
        Token::False => Some(("false", "ложь")),
        Token::Empty => Some(("void", "пустота")),
        Token::KwMacro => Some(("macro", "макрос")),
        _ => None,
    };

    keyword
        .map(|(english, russian)| language.select(english, russian).to_string())
        .unwrap_or_else(|| token_source_text(token))
}

pub(super) fn string_literal(value: &str) -> String {
    format!("{value:?}")
}
