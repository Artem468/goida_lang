use logos::Logos;
use std::ops::Range;

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct LexicalError {
    pub span: Range<usize>,
    pub message: String,
}

#[derive(Logos, Debug, Clone, PartialEq)]
#[logos(skip r"[ \t\f]+")]
#[logos(skip r"//[^\n]*")]
pub(crate) enum Token {
    Eof,
    #[regex(r"\r?\n+")]
    Newline,
    #[token(";")]
    Semi,
    #[token("подключить")]
    KwImport,
    #[token("из")]
    KwFrom,
    #[token("функция")]
    KwFunction,
    #[token("библиотека")]
    KwLibrary,
    #[token("переменная")]
    KwVariable,
    #[token("класс")]
    KwClass,
    #[token("конструктор")]
    KwConstructor,
    #[token("публичный")]
    KwPublic,
    #[token("приватный")]
    KwPrivate,
    #[token("статичный")]
    KwStatic,
    #[token("константа")]
    KwConst,
    #[token("если")]
    KwIf,
    #[token("иначе")]
    KwElse,
    #[token("пока")]
    KwWhile,
    #[token("для")]
    KwFor,
    #[token("поток")]
    KwThread,
    #[token("попробовать")]
    KwTry,
    #[token("перехватить")]
    KwCatch,
    #[token("выбросить")]
    KwRaise,
    #[token("как")]
    KwAs,
    #[token("новый")]
    KwNew,
    #[token("вернуть")]
    KwReturn,
    #[token("и")]
    KwAnd,
    #[token("или")]
    KwOr,
    #[token("истина")]
    True,
    #[token("ложь")]
    False,
    #[token("пустота")]
    Empty,

    #[token("=>")]
    FatArrow,
    #[token("->")]
    Arrow,
    #[token("==")]
    EqEq,
    #[token("!=")]
    NotEq,
    #[token("<=")]
    Le,
    #[token(">=")]
    Ge,
    #[token("+=")]
    PlusEq,
    #[token("-=")]
    MinusEq,
    #[token("*=")]
    StarEq,
    #[token("/=")]
    SlashEq,
    #[token("%=")]
    PercentEq,

    #[token("=")]
    Eq,
    TypeEq,
    #[token("<")]
    Lt,
    #[token(">")]
    Gt,
    #[token("+")]
    Plus,
    #[token("-")]
    Minus,
    #[token("*")]
    Star,
    #[token("/")]
    Slash,
    #[token("%")]
    Percent,
    #[token("!")]
    Bang,
    #[token(".")]
    Dot,
    MethodDot,
    #[token(",")]
    Comma,
    #[token(":")]
    Colon,
    #[token("(")]
    LParen,
    LambdaLParen,
    #[token(")")]
    RParen,
    #[token("{")]
    LBrace,
    #[token("}")]
    RBrace,
    #[token("[")]
    LBracket,
    #[token("]")]
    RBracket,

    #[regex(r#""([^"\\]|\\.)*""#, parse_string)]
    String(String),
    #[regex(r"[0-9]+\.[0-9]+", parse_float)]
    Float(f64),
    #[regex(r"[0-9]+", parse_int)]
    Number(i64),
    #[regex(r"[\p{L}_][\p{L}\p{N}_]*", |lex| lex.slice().to_string())]
    Ident(String),
}

pub(crate) type SpannedToken = Result<(usize, Token, usize), LexicalError>;

pub(crate) fn lex(source: &str) -> impl Iterator<Item = SpannedToken> {
    let raw = Token::lexer(source)
        .spanned()
        .map(|(token, span)| match token {
            Ok(token) => Ok((span.start, token, span.end)),
            Err(()) => Err(LexicalError {
                span: span.clone(),
                message: format!("Неожиданный токен '{}'", &source[span]),
            }),
        })
        .collect::<Vec<_>>();

    let raw = mark_type_equals(mark_method_dots(mark_lambda_starts(raw)));
    let mut output = Vec::new();
    let mut previous_significant: Option<Token> = None;

    for (idx, item) in raw.iter().enumerate() {
        let Ok((start, token, end)) = item else {
            output.push(item.clone());
            continue;
        };

        if *token == Token::Newline {
            let next = raw[idx + 1..].iter().find_map(|candidate| match candidate {
                Ok((_, Token::Newline, _)) => None,
                Ok((_, token, _)) => Some(token),
                Err(_) => None,
            });

            if previous_significant.as_ref().is_some_and(can_end_statement)
                && next.is_none_or(|next| {
                    can_start_statement_after_newline(previous_significant.as_ref(), next)
                })
            {
                output.push(Ok((*start, Token::Semi, *end)));
                previous_significant = Some(Token::Semi);
            }
            continue;
        }

        if *token == Token::Semi {
            if previous_significant != Some(Token::Semi) {
                output.push(Ok((*start, token.clone(), *end)));
                previous_significant = Some(Token::Semi);
            }
            continue;
        }

        if *token == Token::RBrace
            && previous_significant
                .as_ref()
                .is_some_and(|token| *token != Token::Semi && can_end_statement(token))
        {
            output.push(Ok((*start, Token::Semi, *start)));
            previous_significant = Some(Token::Semi);
        }

        output.push(Ok((*start, token.clone(), *end)));
        if *token != Token::Semi {
            previous_significant = Some(token.clone());
        }
    }

    if previous_significant
        .as_ref()
        .is_some_and(|token| *token != Token::Semi && can_end_statement(token))
    {
        let end = source.len();
        output.push(Ok((end, Token::Semi, end)));
    }
    let end = source.len();
    output.push(Ok((end, Token::Eof, end)));

    output.into_iter()
}

fn can_end_statement(token: &Token) -> bool {
    matches!(
        token,
        Token::Ident(_)
            | Token::String(_)
            | Token::Number(_)
            | Token::Float(_)
            | Token::True
            | Token::False
            | Token::Empty
            | Token::RParen
            | Token::RBrace
            | Token::RBracket
    )
}

fn can_start_statement_after_newline(previous: Option<&Token>, token: &Token) -> bool {
    if matches!(previous, Some(Token::RBrace)) && matches!(token, Token::KwCatch | Token::KwElse) {
        return false;
    }

    !matches!(
        token,
        Token::Dot
            | Token::MethodDot
            | Token::Plus
            | Token::Minus
            | Token::Star
            | Token::Slash
            | Token::Percent
            | Token::EqEq
            | Token::NotEq
            | Token::Le
            | Token::Ge
            | Token::Lt
            | Token::Gt
            | Token::KwAnd
            | Token::KwOr
            | Token::Comma
            | Token::RParen
            | Token::RBracket
    )
}

fn mark_lambda_starts(mut tokens: Vec<SpannedToken>) -> Vec<SpannedToken> {
    let len = tokens.len();
    for idx in 0..len {
        let Ok((_, Token::LParen, _)) = tokens[idx] else {
            continue;
        };

        if let Some(close_idx) = matching_paren(&tokens, idx) {
            if matches!(tokens.get(close_idx + 1), Some(Ok((_, Token::FatArrow, _)))) {
                if let Ok((start, _, end)) = &mut tokens[idx] {
                    tokens[idx] = Ok((*start, Token::LambdaLParen, *end));
                }
            }
        }
    }
    tokens
}

fn mark_method_dots(mut tokens: Vec<SpannedToken>) -> Vec<SpannedToken> {
    let len = tokens.len();
    for idx in 0..len {
        let Ok((_, Token::Dot, _)) = tokens[idx] else {
            continue;
        };

        if matches!(tokens.get(idx + 1), Some(Ok((_, Token::Ident(_), _))))
            && matches!(tokens.get(idx + 2), Some(Ok((_, Token::LParen, _))))
            && !dot_is_in_object_creation_type(&tokens, idx)
        {
            if let Ok((start, _, end)) = &mut tokens[idx] {
                tokens[idx] = Ok((*start, Token::MethodDot, *end));
            }
        }
    }
    tokens
}

fn dot_is_in_object_creation_type(tokens: &[SpannedToken], dot_idx: usize) -> bool {
    let mut idx = dot_idx;
    let mut expect_ident = true;

    while idx > 0 {
        idx -= 1;
        match &tokens[idx] {
            Ok((_, Token::Ident(_), _)) if expect_ident => expect_ident = false,
            Ok((_, Token::Dot, _)) if !expect_ident => expect_ident = true,
            Ok((_, Token::KwNew, _)) if !expect_ident => return true,
            _ => return false,
        }
    }

    false
}

fn mark_type_equals(mut tokens: Vec<SpannedToken>) -> Vec<SpannedToken> {
    let len = tokens.len();
    for idx in 0..len {
        let Ok((_, Token::Eq, _)) = tokens[idx] else {
            continue;
        };

        if eq_follows_type_hint(&tokens, idx) {
            if let Ok((start, _, end)) = &mut tokens[idx] {
                tokens[idx] = Ok((*start, Token::TypeEq, *end));
            }
        }
    }
    tokens
}

fn eq_follows_type_hint(tokens: &[SpannedToken], eq_idx: usize) -> bool {
    let mut idx = eq_idx;
    let mut saw_ident = false;

    while idx > 0 {
        idx -= 1;
        match &tokens[idx] {
            Ok((_, Token::Ident(_), _)) => {
                saw_ident = true;
            }
            Ok((_, Token::Dot, _)) if saw_ident => {
                saw_ident = false;
            }
            Ok((_, Token::Colon, _)) if saw_ident => return true,
            Ok((
                _,
                Token::Newline | Token::Semi | Token::LParen | Token::LBrace | Token::Comma,
                _,
            )) => {
                return false;
            }
            Ok(_) | Err(_) => return false,
        }
    }

    false
}

fn matching_paren(tokens: &[SpannedToken], open_idx: usize) -> Option<usize> {
    let mut depth = 0usize;
    for (idx, token) in tokens.iter().enumerate().skip(open_idx) {
        match token {
            Ok((_, Token::LParen | Token::LambdaLParen, _)) => depth += 1,
            Ok((_, Token::RParen, _)) => {
                depth = depth.saturating_sub(1);
                if depth == 0 {
                    return Some(idx);
                }
            }
            Ok((_, Token::Newline, _)) if depth == 1 => return None,
            Err(_) => return None,
            _ => {}
        }
    }
    None
}

fn parse_int(lex: &mut logos::Lexer<'_, Token>) -> Option<i64> {
    lex.slice().parse().ok()
}

fn parse_float(lex: &mut logos::Lexer<'_, Token>) -> Option<f64> {
    lex.slice().parse().ok()
}

fn parse_string(lex: &mut logos::Lexer<'_, Token>) -> String {
    let slice = lex.slice();
    let raw = &slice[1..slice.len() - 1];
    let mut result = String::with_capacity(raw.len());
    let mut chars = raw.chars();
    while let Some(ch) = chars.next() {
        if ch == '\\' {
            match chars.next() {
                Some('n') => result.push('\n'),
                Some('t') => result.push('\t'),
                Some('"') => result.push('"'),
                Some('\\') => result.push('\\'),
                Some(other) => {
                    result.push('\\');
                    result.push(other);
                }
                None => result.push('\\'),
            }
        } else {
            result.push(ch);
        }
    }
    result
}
