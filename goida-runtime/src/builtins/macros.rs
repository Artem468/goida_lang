use crate::ast::prelude::{ErrorData, Span};
use crate::builtins::registry::*;
use crate::parser::lexer::Token;
use crate::parser::macro_expander::MacroExpander;
use crate::parser::prelude::ParseError;
use crate::parser::syntax as syn;
use std::ops::Range;
use string_interner::Symbol;

pub(crate) fn setup_macro_builtins(expander: &mut MacroExpander) -> Result<(), ParseError> {
    for alias in BUILTINS.macro_names(macros::FORMAT.canonical) {
        expander.register_native(alias, expand_format_macro);
    }
    Ok(())
}

#[derive(Debug, PartialEq)]
enum FormatPart {
    Text(String),
    Placeholder,
}

fn expand_format_macro(
    call: &syn::MacroCall,
    args: &[syn::MacroToken],
) -> Result<Vec<syn::MacroToken>, ParseError> {
    let parts = split_top_level_args(args).ok_or_else(|| {
        macro_error(
            call.span.clone(),
            "Некорректные аргументы встроенного макроса 'format'",
        )
    })?;
    let Some((first, value_parts)) = parts.split_first() else {
        return Err(macro_error(
            call.span.clone(),
            "Макрос 'format' ожидает строку формата",
        ));
    };
    let [syn::MacroToken {
        token: Token::String(pattern),
        ..
    }] = first.as_slice()
    else {
        return Err(macro_error(
            call.span.clone(),
            "Первый аргумент 'format' должен быть строковым литералом",
        ));
    };

    let format_parts = parse_format_pattern(pattern, call.span.clone())?;
    let placeholders = format_parts
        .iter()
        .filter(|part| matches!(part, FormatPart::Placeholder))
        .count();
    if placeholders != value_parts.len() {
        return Err(macro_error(
            call.span.clone(),
            format!(
                "Макрос 'format' ожидал {} аргументов для подстановки, получено {}",
                placeholders,
                value_parts.len()
            ),
        ));
    }

    if placeholders == 0 {
        return Ok(vec![macro_token(
            Token::String(pattern.clone()),
            call.span.clone(),
        )]);
    }

    let mut output = vec![macro_token(Token::String(String::new()), call.span.clone())];
    let mut value_index = 0usize;

    for part in format_parts {
        let tokens = match part {
            FormatPart::Text(text) if text.is_empty() => continue,
            FormatPart::Text(text) => vec![macro_token(Token::String(text), call.span.clone())],
            FormatPart::Placeholder => {
                let tokens = parenthesize_value_part(&value_parts[value_index], call.span.clone());
                value_index += 1;
                tokens
            }
        };
        output.push(macro_token(Token::Plus, call.span.clone()));
        output.extend(tokens);
    }

    Ok(output)
}

fn parenthesize_value_part(
    tokens: &[syn::MacroToken],
    fallback_span: Range<usize>,
) -> Vec<syn::MacroToken> {
    let open_span = tokens
        .first()
        .map(|token| token.span.start..token.span.start)
        .unwrap_or_else(|| fallback_span.clone());
    let close_span = tokens
        .last()
        .map(|token| token.span.end..token.span.end)
        .unwrap_or(fallback_span);

    let mut output = vec![macro_token(Token::LParen, open_span)];
    output.extend(tokens.iter().cloned());
    output.push(macro_token(Token::RParen, close_span));
    output
}

fn split_top_level_args(tokens: &[syn::MacroToken]) -> Option<Vec<Vec<syn::MacroToken>>> {
    if tokens.is_empty() {
        return Some(Vec::new());
    }

    let mut args = Vec::new();
    let mut current = Vec::new();
    let mut stack = Vec::new();

    for token in tokens {
        match token.token {
            Token::LParen => {
                stack.push(Token::RParen);
                current.push(token.clone());
            }
            Token::LBracket => {
                stack.push(Token::RBracket);
                current.push(token.clone());
            }
            Token::LBrace => {
                stack.push(Token::RBrace);
                current.push(token.clone());
            }
            Token::RParen | Token::RBracket | Token::RBrace => {
                if stack.pop().as_ref() != Some(&token.token) {
                    return None;
                }
                current.push(token.clone());
            }
            Token::Comma if stack.is_empty() => {
                args.push(current);
                current = Vec::new();
            }
            _ => current.push(token.clone()),
        }
    }

    if !stack.is_empty() {
        return None;
    }
    args.push(current);
    Some(args)
}

fn parse_format_pattern(pattern: &str, span: Range<usize>) -> Result<Vec<FormatPart>, ParseError> {
    let mut parts = Vec::new();
    let mut text = String::new();
    let mut chars = pattern.chars().peekable();

    while let Some(ch) = chars.next() {
        match ch {
            '{' if chars.peek() == Some(&'}') => {
                chars.next();
                if !text.is_empty() {
                    parts.push(FormatPart::Text(std::mem::take(&mut text)));
                }
                parts.push(FormatPart::Placeholder);
            }
            '{' if chars.peek() == Some(&'{') => {
                chars.next();
                text.push('{');
            }
            '}' if chars.peek() == Some(&'}') => {
                chars.next();
                text.push('}');
            }
            '{' | '}' => {
                return Err(macro_error(
                    span,
                    "В строке 'format' одиночные фигурные скобки запрещены; используйте '{}' или экранирование '{{' и '}}'",
                ));
            }
            other => text.push(other),
        }
    }

    if !text.is_empty() {
        parts.push(FormatPart::Text(text));
    }
    Ok(parts)
}

fn macro_token(token: Token, span: Range<usize>) -> syn::MacroToken {
    syn::MacroToken { token, span }
}

fn macro_error(span: Range<usize>, message: impl Into<String>) -> ParseError {
    ParseError::InvalidSyntax(ErrorData::new(
        Span::new(
            span.start,
            span.end,
            string_interner::DefaultSymbol::try_from_usize(0).unwrap(),
        ),
        message.into(),
    ))
}
