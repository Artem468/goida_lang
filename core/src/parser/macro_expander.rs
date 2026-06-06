use crate::ast::prelude::{ErrorData, Span};
use crate::parser::grammar;
use crate::parser::lexer::{SpannedToken, Token};
use crate::parser::prelude::{ParseError, Parser as ParserTrait};
use crate::parser::syntax as syn;
use std::collections::HashMap;
use std::ops::Range;
use string_interner::Symbol;

pub(crate) type NativeMacroExpander =
    fn(&syn::MacroCall, &[syn::MacroToken]) -> Result<Vec<syn::MacroToken>, ParseError>;

#[derive(Default)]
pub(crate) struct MacroExpander {
    definitions: HashMap<String, syn::MacroDefinition>,
    native_definitions: HashMap<String, NativeMacroExpander>,
}

#[derive(Clone, Debug)]
enum Capture {
    Single(Vec<syn::MacroToken>),
    Repeated(Vec<Vec<syn::MacroToken>>),
}

type Captures = HashMap<String, Capture>;

impl ParserTrait {
    pub(crate) fn expand_macros(&self, program: syn::Program) -> Result<syn::Program, ParseError> {
        let mut expander = MacroExpander::default();
        expander.register_builtins()?;
        expander.expand_program(program, self.module.name)
    }
}

impl MacroExpander {
    fn register_builtins(&mut self) -> Result<(), ParseError> {
        crate::builtins::macros::setup_macro_builtins(self)
    }

    pub(crate) fn register_native(&mut self, name: &str, expander: NativeMacroExpander) {
        self.native_definitions.insert(name.to_string(), expander);
    }

    #[allow(dead_code)]
    pub(crate) fn register_builtin(
        &mut self,
        name: &str,
        rules: &[(&str, &str)],
    ) -> Result<(), ParseError> {
        let mut parsed_rules = Vec::with_capacity(rules.len());
        for (matcher, template) in rules {
            let source = format!("macro {name} {{ ({matcher}) => {{ {template} }}; }}");
            let program = grammar::ProgramParser::new()
                .parse(crate::parser::lexer::lex(&source))
                .map_err(|_| {
                    macro_error(
                        0..source.len(),
                        "Некорректное объявление встроенного макроса",
                    )
                })?;
            let Some(item) = program.items.into_iter().next() else {
                return Err(macro_error(
                    0..source.len(),
                    "Пустое объявление встроенного макроса",
                ));
            };
            let syn::ItemKind::MacroDefinition(definition) = item.node else {
                return Err(macro_error(0..source.len(), "Ожидалось объявление макроса"));
            };
            parsed_rules.extend(definition.rules);
        }
        self.definitions.insert(
            name.to_string(),
            syn::MacroDefinition {
                name: name.to_string(),
                rules: parsed_rules,
            },
        );
        Ok(())
    }

    fn expand_program(
        &mut self,
        program: syn::Program,
        module_name: string_interner::DefaultSymbol,
    ) -> Result<syn::Program, ParseError> {
        let items = self.expand_items(program.items, module_name)?;
        Ok(syn::Program { items })
    }

    fn expand_items(
        &mut self,
        items: Vec<syn::Item>,
        module_name: string_interner::DefaultSymbol,
    ) -> Result<Vec<syn::Item>, ParseError> {
        let mut expanded = Vec::new();
        for item in items {
            match item.node {
                syn::ItemKind::MacroDefinition(definition) => {
                    self.definitions.insert(definition.name.clone(), definition);
                }
                syn::ItemKind::Function(mut function) => {
                    function.body = self.expand_items(function.body, module_name)?;
                    expanded.push(syn::Spanned::new(
                        syn::ItemKind::Function(function),
                        item.span.start,
                        item.span.end,
                    ));
                }
                syn::ItemKind::Class(mut class) => {
                    for class_item in &mut class.items {
                        match &mut class_item.node {
                            syn::ClassItemKind::Constructor(method)
                            | syn::ClassItemKind::Method(method) => {
                                method.body = self
                                    .expand_items(std::mem::take(&mut method.body), module_name)?;
                            }
                            syn::ClassItemKind::Field(field) => {
                                if let Some(expr) = field.default_value.take() {
                                    field.default_value =
                                        Some(self.expand_expr(expr, module_name)?);
                                }
                            }
                        }
                    }
                    expanded.push(syn::Spanned::new(
                        syn::ItemKind::Class(class),
                        item.span.start,
                        item.span.end,
                    ));
                }
                syn::ItemKind::Statement(stmt) => {
                    if let syn::StmtKind::Expr(syn::Spanned {
                        node: syn::ExprKind::MacroCall(call),
                        ..
                    }) = &stmt.node
                    {
                        let tokens = self.expand_call(call)?;
                        let mut items = parse_items(tokens, module_name, call.span.clone())?;
                        items = self.expand_items(items, module_name)?;
                        expanded.extend(items);
                        continue;
                    }
                    let stmt = self.expand_stmt(*stmt, module_name)?;
                    expanded.push(syn::Spanned::new(
                        syn::ItemKind::Statement(Box::new(stmt)),
                        item.span.start,
                        item.span.end,
                    ));
                }
                node => expanded.push(syn::Spanned::new(node, item.span.start, item.span.end)),
            }
        }
        Ok(expanded)
    }

    fn expand_stmt(
        &mut self,
        stmt: syn::Stmt,
        module_name: string_interner::DefaultSymbol,
    ) -> Result<syn::Stmt, ParseError> {
        let span = stmt.span.clone();
        let node = match stmt.node {
            syn::StmtKind::Assign {
                name,
                is_const,
                type_hint,
                value,
            } => syn::StmtKind::Assign {
                name,
                is_const,
                type_hint,
                value: self.expand_expr(value, module_name)?,
            },
            syn::StmtKind::AssignTarget { target, value } => syn::StmtKind::AssignTarget {
                target: self.expand_expr(target, module_name)?,
                value: self.expand_expr(value, module_name)?,
            },
            syn::StmtKind::CompoundAssign { target, op, value } => syn::StmtKind::CompoundAssign {
                target: self.expand_expr(target, module_name)?,
                op,
                value: self.expand_expr(value, module_name)?,
            },
            syn::StmtKind::If {
                condition,
                then_body,
                else_body,
            } => syn::StmtKind::If {
                condition: self.expand_expr(condition, module_name)?,
                then_body: self.expand_items(then_body, module_name)?,
                else_body: match else_body {
                    Some(syn::ElseBody::Block(items, span)) => Some(syn::ElseBody::Block(
                        self.expand_items(items, module_name)?,
                        span,
                    )),
                    Some(syn::ElseBody::If(stmt)) => Some(syn::ElseBody::If(Box::new(
                        self.expand_stmt(*stmt, module_name)?,
                    ))),
                    None => None,
                },
            },
            syn::StmtKind::While { condition, body } => syn::StmtKind::While {
                condition: self.expand_expr(condition, module_name)?,
                body: self.expand_items(body, module_name)?,
            },
            syn::StmtKind::For {
                variable,
                init,
                condition,
                update,
                body,
            } => syn::StmtKind::For {
                variable,
                init: self.expand_expr(init, module_name)?,
                condition: self.expand_expr(condition, module_name)?,
                update: Box::new(self.expand_for_update(*update, module_name)?),
                body: self.expand_items(body, module_name)?,
            },
            syn::StmtKind::ForEach {
                variable,
                iterable,
                body,
            } => syn::StmtKind::ForEach {
                variable,
                iterable: self.expand_expr(iterable, module_name)?,
                body: self.expand_items(body, module_name)?,
            },
            syn::StmtKind::Thread { body } => syn::StmtKind::Thread {
                body: self.expand_items(body, module_name)?,
            },
            syn::StmtKind::Try { body, mut handlers } => {
                for handler in &mut handlers {
                    handler.body =
                        self.expand_items(std::mem::take(&mut handler.body), module_name)?;
                }
                syn::StmtKind::Try {
                    body: self.expand_items(body, module_name)?,
                    handlers,
                }
            }
            syn::StmtKind::Raise {
                error_type,
                message,
            } => syn::StmtKind::Raise {
                error_type,
                message: message
                    .map(|expr| self.expand_expr(expr, module_name))
                    .transpose()?,
            },
            syn::StmtKind::Return(expr) => syn::StmtKind::Return(
                expr.map(|expr| self.expand_expr(expr, module_name))
                    .transpose()?,
            ),
            syn::StmtKind::Expr(expr) => syn::StmtKind::Expr(self.expand_expr(expr, module_name)?),
        };
        Ok(syn::Spanned { node, span })
    }

    fn expand_for_update(
        &mut self,
        update: syn::ForUpdate,
        module_name: string_interner::DefaultSymbol,
    ) -> Result<syn::ForUpdate, ParseError> {
        Ok(match update {
            syn::ForUpdate::Assign { name, value, span } => syn::ForUpdate::Assign {
                name,
                value: self.expand_expr(value, module_name)?,
                span,
            },
            syn::ForUpdate::AssignTarget {
                target,
                value,
                span,
            } => syn::ForUpdate::AssignTarget {
                target: self.expand_expr(target, module_name)?,
                value: self.expand_expr(value, module_name)?,
                span,
            },
            syn::ForUpdate::Compound {
                target,
                op,
                value,
                span,
            } => syn::ForUpdate::Compound {
                target: self.expand_expr(target, module_name)?,
                op,
                value: self.expand_expr(value, module_name)?,
                span,
            },
            syn::ForUpdate::Expr(expr) => {
                syn::ForUpdate::Expr(self.expand_expr(expr, module_name)?)
            }
        })
    }

    fn expand_expr(
        &mut self,
        expr: syn::Expr,
        module_name: string_interner::DefaultSymbol,
    ) -> Result<syn::Expr, ParseError> {
        let span = expr.span.clone();
        let node = match expr.node {
            syn::ExprKind::MacroCall(call) => {
                let tokens = self.expand_call(&call)?;
                return parse_expr(tokens, module_name, call.span);
            }
            syn::ExprKind::Binary { op, left, right } => syn::ExprKind::Binary {
                op,
                left: Box::new(self.expand_expr(*left, module_name)?),
                right: Box::new(self.expand_expr(*right, module_name)?),
            },
            syn::ExprKind::Unary { op, operand } => syn::ExprKind::Unary {
                op,
                operand: Box::new(self.expand_expr(*operand, module_name)?),
            },
            syn::ExprKind::FunctionCall { function, args } => syn::ExprKind::FunctionCall {
                function: Box::new(self.expand_expr(*function, module_name)?),
                args: self.expand_call_args(args, module_name)?,
            },
            syn::ExprKind::MethodCall {
                object,
                method,
                args,
            } => syn::ExprKind::MethodCall {
                object: Box::new(self.expand_expr(*object, module_name)?),
                method,
                args: self.expand_call_args(args, module_name)?,
            },
            syn::ExprKind::PropertyAccess { object, property } => syn::ExprKind::PropertyAccess {
                object: Box::new(self.expand_expr(*object, module_name)?),
                property,
            },
            syn::ExprKind::Index { object, index } => syn::ExprKind::Index {
                object: Box::new(self.expand_expr(*object, module_name)?),
                index: Box::new(self.expand_expr(*index, module_name)?),
            },
            syn::ExprKind::ObjectCreation { class_name, args } => syn::ExprKind::ObjectCreation {
                class_name,
                args: self.expand_call_args(args, module_name)?,
            },
            syn::ExprKind::Lambda { params, body } => syn::ExprKind::Lambda {
                params,
                body: match body {
                    syn::LambdaBody::Expr(expr) => {
                        syn::LambdaBody::Expr(Box::new(self.expand_expr(*expr, module_name)?))
                    }
                    syn::LambdaBody::Block(items, span) => {
                        syn::LambdaBody::Block(self.expand_items(items, module_name)?, span)
                    }
                },
            },
            other => other,
        };
        Ok(syn::Spanned { node, span })
    }

    fn expand_call_args(
        &mut self,
        args: Vec<syn::CallArg>,
        module_name: string_interner::DefaultSymbol,
    ) -> Result<Vec<syn::CallArg>, ParseError> {
        args.into_iter()
            .map(|arg| {
                Ok(syn::CallArg {
                    name: arg.name,
                    value: self.expand_expr(arg.value, module_name)?,
                })
            })
            .collect()
    }

    fn expand_call(&self, call: &syn::MacroCall) -> Result<Vec<syn::MacroToken>, ParseError> {
        let args = normalized_call_args(call);
        if let Some(expander) = self.native_definitions.get(&call.name) {
            return expander(call, &args);
        }

        let definition = self.definitions.get(&call.name).ok_or_else(|| {
            macro_error(
                call.span.clone(),
                format!("Макрос '{}' не найден", call.name),
            )
        })?;
        for rule in &definition.rules {
            let captures = Captures::new();
            if let Some((pos, captures)) = match_sequence(&rule.matcher, &args, 0, captures) {
                if pos == args.len() {
                    return expand_template(&rule.template, &captures, None);
                }
            }
        }
        Err(macro_error(
            call.span.clone(),
            format!(
                "Аргументы не подходят ни под одно правило макроса '{}'",
                call.name
            ),
        ))
    }
}

fn normalized_call_args(call: &syn::MacroCall) -> Vec<syn::MacroToken> {
    let mut args = call.args.clone();
    if call.delimiter == syn::MacroDelimiter::Brace {
        while matches!(args.last().map(|token| &token.token), Some(Token::Semi)) {
            args.pop();
        }
    }
    args
}

fn match_sequence(
    matcher: &[syn::MacroMatcher],
    tokens: &[syn::MacroToken],
    pos: usize,
    captures: Captures,
) -> Option<(usize, Captures)> {
    if matcher.is_empty() {
        return Some((pos, captures));
    }
    match &matcher[0] {
        syn::MacroMatcher::Token(expected) => {
            let actual = tokens.get(pos)?;
            if same_token(&expected.token, &actual.token) {
                match_sequence(&matcher[1..], tokens, pos + 1, captures)
            } else {
                None
            }
        }
        syn::MacroMatcher::Fragment { name, kind } => {
            for end in (pos + 1..=tokens.len()).rev() {
                let fragment = &tokens[pos..end];
                if !valid_fragment(*kind, fragment) {
                    continue;
                }
                let mut captures = captures.clone();
                captures.insert(name.clone(), Capture::Single(fragment.to_vec()));
                if let Some(found) = match_sequence(&matcher[1..], tokens, end, captures) {
                    return Some(found);
                }
            }
            None
        }
        syn::MacroMatcher::Repeat {
            matcher: inner,
            separator,
            op,
        } => {
            let mut pos = pos;
            let mut captures = captures;
            let mut count = 0usize;
            loop {
                let local = Captures::new();
                let Some((next_pos, local)) = match_sequence(inner, tokens, pos, local) else {
                    break;
                };
                if next_pos == pos {
                    break;
                }
                merge_repeated(&mut captures, local);
                pos = next_pos;
                count += 1;
                if !separator.is_empty() && tokens_match_at(tokens, pos, separator) {
                    pos += separator.len();
                } else if !separator.is_empty() {
                    break;
                }
            }
            if *op == syn::MacroRepeatOp::OneOrMore && count == 0 {
                return None;
            }
            match_sequence(&matcher[1..], tokens, pos, captures)
        }
    }
}

fn merge_repeated(target: &mut Captures, source: Captures) {
    for (name, capture) in source {
        let Capture::Single(tokens) = capture else {
            continue;
        };
        match target.get_mut(&name) {
            Some(Capture::Repeated(items)) => items.push(tokens),
            _ => {
                target.insert(name, Capture::Repeated(vec![tokens]));
            }
        }
    }
}

fn expand_template(
    template: &[syn::MacroTemplate],
    captures: &Captures,
    index: Option<usize>,
) -> Result<Vec<syn::MacroToken>, ParseError> {
    let mut output = Vec::new();
    for item in template {
        match item {
            syn::MacroTemplate::Token(token) => output.push(token.clone()),
            syn::MacroTemplate::Variable(name) => {
                output.extend(resolve_capture(name, captures, index)?);
            }
            syn::MacroTemplate::Delimited {
                delimiter,
                template,
                span,
            } => {
                let (open, close) = delimiter_tokens(*delimiter);
                output.push(syn::MacroToken {
                    token: open,
                    span: span.start..span.start.saturating_add(1),
                });
                output.extend(expand_template(template, captures, index)?);
                output.push(syn::MacroToken {
                    token: close,
                    span: span.end.saturating_sub(1)..span.end,
                });
            }
            syn::MacroTemplate::Repeat {
                template,
                separator,
                ..
            } => {
                let count = repeated_len(template, captures).unwrap_or(0);
                for idx in 0..count {
                    if idx > 0 {
                        output.extend(separator.iter().cloned());
                    }
                    output.extend(expand_template(template, captures, Some(idx))?);
                }
            }
        }
    }
    Ok(output)
}

fn resolve_capture(
    name: &str,
    captures: &Captures,
    index: Option<usize>,
) -> Result<Vec<syn::MacroToken>, ParseError> {
    match captures.get(name) {
        Some(Capture::Single(tokens)) if index.is_none() => Ok(tokens.clone()),
        Some(Capture::Repeated(items)) => {
            let idx = index.ok_or_else(|| {
                macro_error(0..0, "Повторяемая переменная использована вне повторения")
            })?;
            items
                .get(idx)
                .cloned()
                .ok_or_else(|| macro_error(0..0, "Индекс повторения макроса вне диапазона"))
        }
        Some(Capture::Single(_)) => Err(macro_error(
            0..0,
            format!("Переменная макроса '${name}' не является повторяемой"),
        )),
        None => Err(macro_error(
            0..0,
            format!("Переменная макроса '${name}' не найдена"),
        )),
    }
}

fn repeated_len(template: &[syn::MacroTemplate], captures: &Captures) -> Option<usize> {
    for item in template {
        match item {
            syn::MacroTemplate::Variable(name) => {
                if let Some(Capture::Repeated(items)) = captures.get(name) {
                    return Some(items.len());
                }
            }
            syn::MacroTemplate::Delimited { template, .. } => {
                if let Some(len) = repeated_len(template, captures) {
                    return Some(len);
                }
            }
            syn::MacroTemplate::Repeat { template, .. } => {
                if let Some(len) = repeated_len(template, captures) {
                    return Some(len);
                }
            }
            syn::MacroTemplate::Token(_) => {}
        }
    }
    None
}

fn delimiter_tokens(delimiter: syn::MacroDelimiter) -> (Token, Token) {
    match delimiter {
        syn::MacroDelimiter::Paren => (Token::LParen, Token::RParen),
        syn::MacroDelimiter::Bracket => (Token::LBracket, Token::RBracket),
        syn::MacroDelimiter::Brace => (Token::LBrace, Token::RBrace),
    }
}

fn valid_fragment(kind: syn::MacroFragmentKind, tokens: &[syn::MacroToken]) -> bool {
    match kind {
        syn::MacroFragmentKind::Ident => matches!(
            tokens,
            [syn::MacroToken {
                token: Token::Ident(_),
                ..
            }]
        ),
        syn::MacroFragmentKind::Block => {
            matches!(tokens.first().map(|t| &t.token), Some(Token::LBrace))
                && matches!(tokens.last().map(|t| &t.token), Some(Token::RBrace))
                && delimiters_balanced(tokens)
        }
        syn::MacroFragmentKind::Expr => parse_expr(tokens.to_vec(), dummy_symbol(), 0..0).is_ok(),
        syn::MacroFragmentKind::Stmt => parse_stmt(tokens.to_vec(), dummy_symbol(), 0..0).is_ok(),
    }
}

fn delimiters_balanced(tokens: &[syn::MacroToken]) -> bool {
    let mut stack = Vec::new();
    for token in tokens {
        match token.token {
            Token::LParen => stack.push(Token::RParen),
            Token::LBracket => stack.push(Token::RBracket),
            Token::LBrace => stack.push(Token::RBrace),
            Token::RParen | Token::RBracket | Token::RBrace => {
                if stack.pop().as_ref() != Some(&token.token) {
                    return false;
                }
            }
            _ => {}
        }
    }
    stack.is_empty()
}

fn tokens_match_at(tokens: &[syn::MacroToken], pos: usize, expected: &[syn::MacroToken]) -> bool {
    expected.iter().enumerate().all(|(idx, expected)| {
        tokens
            .get(pos + idx)
            .is_some_and(|actual| same_token(&actual.token, &expected.token))
    })
}

fn same_token(left: &Token, right: &Token) -> bool {
    std::mem::discriminant(left) == std::mem::discriminant(right)
        && match (left, right) {
            (Token::Ident(a), Token::Ident(b)) => a == b,
            (Token::String(a), Token::String(b)) => a == b,
            (Token::Number(a), Token::Number(b)) => a == b,
            (Token::Float(a), Token::Float(b)) => a == b,
            _ => true,
        }
}

fn parse_items(
    tokens: Vec<syn::MacroToken>,
    module_name: string_interner::DefaultSymbol,
    span: Range<usize>,
) -> Result<Vec<syn::Item>, ParseError> {
    let stream = expansion_stream(tokens, true);
    grammar::ProgramParser::new()
        .parse(stream)
        .map(|program| program.items)
        .map_err(|_| {
            macro_error_with_module(
                module_name,
                span,
                "Макрос развернулся в некорректные элементы AST",
            )
        })
}

fn parse_stmt(
    tokens: Vec<syn::MacroToken>,
    module_name: string_interner::DefaultSymbol,
    span: Range<usize>,
) -> Result<syn::Stmt, ParseError> {
    let stream = expansion_stream(tokens, true);
    grammar::StmtExpansionParser::new()
        .parse(stream)
        .map_err(|_| {
            macro_error_with_module(
                module_name,
                span,
                "Макрос развернулся в некорректную инструкцию",
            )
        })
}

fn parse_expr(
    mut tokens: Vec<syn::MacroToken>,
    module_name: string_interner::DefaultSymbol,
    span: Range<usize>,
) -> Result<syn::Expr, ParseError> {
    while matches!(tokens.last().map(|token| &token.token), Some(Token::Semi)) {
        tokens.pop();
    }
    let stream = expansion_stream(tokens, false);
    grammar::ExprExpansionParser::new()
        .parse(stream)
        .map_err(|_| {
            macro_error_with_module(
                module_name,
                span,
                "Макрос развернулся в некорректное выражение",
            )
        })
}

fn expansion_stream(
    mut tokens: Vec<syn::MacroToken>,
    statement_context: bool,
) -> Vec<SpannedToken> {
    let end = tokens.last().map_or(0, |token| token.span.end);
    if statement_context
        && !matches!(
            tokens.last().map(|token| &token.token),
            Some(Token::Semi | Token::RBrace)
        )
    {
        tokens.push(syn::MacroToken {
            token: Token::Semi,
            span: end..end,
        });
    }
    tokens.push(syn::MacroToken {
        token: Token::Eof,
        span: end..end,
    });
    tokens
        .into_iter()
        .map(|token| Ok((token.span.start, token.token, token.span.end)))
        .collect()
}

fn macro_error(span: Range<usize>, message: impl Into<String>) -> ParseError {
    macro_error_with_module(dummy_symbol(), span, message)
}

fn macro_error_with_module(
    module_name: string_interner::DefaultSymbol,
    span: Range<usize>,
    message: impl Into<String>,
) -> ParseError {
    ParseError::InvalidSyntax(ErrorData::new(
        Span::new(span.start, span.end, module_name),
        message.into(),
    ))
}

fn dummy_symbol() -> string_interner::DefaultSymbol {
    string_interner::DefaultSymbol::try_from_usize(0).unwrap()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::prelude::{BinaryOperator, ExpressionKind, LiteralValue, StatementKind};
    use crate::interpreter::prelude::SharedInterner;
    use crate::parser::prelude::Parser as ProgramParser;
    use crate::shared::SharedMut;
    use std::path::PathBuf;
    use string_interner::StringInterner;

    #[test]
    fn define_builtin_macro_registers_rules() {
        let mut expander = MacroExpander::default();
        let result: Result<(), ParseError> = (|| {
            crate::define_builtin_macro!(expander, "twice" => {
                ("$x:expr") => { "$x + $x" };
            });
            Ok(())
        })();

        assert!(result.is_ok(), "{result:?}");
        let definition = expander
            .definitions
            .get("twice")
            .expect("builtin macro should be registered");
        assert_eq!(definition.rules.len(), 1);
    }

    #[test]
    fn format_macro_expands_to_expected_ast() {
        let interner = SharedMut::new(StringInterner::new());
        let actual = parse_for_ast(
            interner.clone(),
            r#"
имя = "Анна"
возраст = 21
результат = format!("Привет, {}, тебе {} лет", имя, возраст)
"#,
        );
        let expected = parse_for_ast(
            interner.clone(),
            r#"
имя = "Анна"
возраст = 21
результат = "Привет, " + имя + ", тебе " + возраст + " лет"
"#,
        );

        assert_eq!(
            normalize_statements(&actual, &interner),
            normalize_statements(&expected, &interner)
        );
    }

    #[test]
    fn single_english_expr_macro_expands_to_expected_ast() {
        assert_same_ast(
            r#"
macro twice {
    ($x:expr) => { $x + $x };
}

value = twice!(input + 1)
"#,
            r#"
value = input + 1 + input + 1
"#,
        );
    }

    #[test]
    fn multiple_english_macro_rules_pick_first_full_match_for_ast() {
        assert_same_ast(
            r#"
macro choose {
    () => { "empty" };
    ($value:expr) => { $value };
}

empty_value = choose!()
filled_value = choose!("filled")
"#,
            r#"
empty_value = "empty"
filled_value = "filled"
"#,
        );
    }

    #[test]
    fn russian_macro_keyword_and_fragment_aliases_expand_to_expected_ast() {
        assert_same_ast(
            r#"
macro choose_ru {
    ($name:имя) => { $name };
    ($value:выр) => { $value };
}

source = 9
same = choose_ru!(source)
computed = choose_ru!(source + 1)
"#,
            r#"
source = 9
same = source
computed = source + 1
"#,
        );
    }

    #[test]
    fn repetition_macro_with_zero_one_and_many_items_expands_to_expected_ast() {
        assert_same_ast(
            r#"
macro make_list {
    ($( $x:expr ),*) => { list($( $x ),*) };
}

empty = make_list!()
single = make_list!(1)
many = make_list!(1, 2, 3)
"#,
            r#"
empty = list()
single = list(1)
many = list(1, 2, 3)
"#,
        );
    }

    #[test]
    fn one_or_more_repetition_macro_expands_to_expected_ast() {
        assert_same_ast(
            r#"
macro sum {
    ($first:expr $( + $rest:expr )+) => { $first $( + $rest )+ };
}

value = sum!(1 + 2 + 3)
"#,
            r#"
value = 1 + 2 + 3
"#,
        );
    }

    #[test]
    fn statement_and_block_fragments_expand_to_expected_ast() {
        assert_same_ast(
            r#"
macro when {
    ($condition:expr, $statement:stmt) => { if ($condition) { $statement } };
}

macro guarded {
    ($condition:expr, $body:block) => { if ($condition) $body };
}

value = 0
when!(true, value = 1)
guarded!(false, { value = 2 })
"#,
            r#"
value = 0
if (true) {
    value = 1
}
if (false) {
    value = 2
}
"#,
        );
    }

    fn parse_for_ast(
        interner: SharedInterner,
        source: &str,
    ) -> crate::interpreter::prelude::Module {
        ProgramParser::new(
            interner,
            "macro_ast_test",
            PathBuf::from("macro_ast_test.goida"),
        )
        .parse(source)
        .expect("source should parse")
    }

    fn assert_same_ast(actual_source: &str, expected_source: &str) {
        let interner = SharedMut::new(StringInterner::new());
        let actual = parse_for_ast(interner.clone(), actual_source);
        let expected = parse_for_ast(interner.clone(), expected_source);

        assert_eq!(
            normalize_statements(&actual, &interner),
            normalize_statements(&expected, &interner)
        );
    }

    fn normalize_statements(
        module: &crate::interpreter::prelude::Module,
        interner: &SharedInterner,
    ) -> Vec<String> {
        module
            .body
            .iter()
            .map(|stmt| normalize_statement(module, interner, *stmt))
            .collect()
    }

    fn normalize_statement(
        module: &crate::interpreter::prelude::Module,
        interner: &SharedInterner,
        stmt_id: crate::ast::prelude::StmtId,
    ) -> String {
        let stmt = module
            .arena
            .get_statement(stmt_id)
            .expect("statement should exist");
        match &stmt.kind {
            StatementKind::Assign {
                name,
                is_const,
                type_hint,
                value,
            } => format!(
                "assign({},{is_const},{:?},{})",
                resolve(interner, *name),
                type_hint,
                normalize_expr(module, interner, *value)
            ),
            StatementKind::Expression(expr) => {
                format!("expr({})", normalize_expr(module, interner, *expr))
            }
            StatementKind::Block(items) => format!(
                "block({})",
                items
                    .iter()
                    .map(|stmt| normalize_statement(module, interner, *stmt))
                    .collect::<Vec<_>>()
                    .join(",")
            ),
            StatementKind::If {
                condition,
                then_body,
                else_body,
            } => format!(
                "if({},{},{})",
                normalize_expr(module, interner, *condition),
                normalize_statement(module, interner, *then_body),
                else_body
                    .map(|stmt| normalize_statement(module, interner, stmt))
                    .unwrap_or_else(|| "none".to_string())
            ),
            other => format!("{other:?}"),
        }
    }

    fn normalize_expr(
        module: &crate::interpreter::prelude::Module,
        interner: &SharedInterner,
        expr_id: crate::ast::prelude::ExprId,
    ) -> String {
        let expr = module
            .arena
            .get_expression(expr_id)
            .expect("expression should exist");
        match &expr.kind {
            ExpressionKind::Literal(LiteralValue::Text(symbol)) => {
                format!("text({:?})", resolve(interner, *symbol))
            }
            ExpressionKind::Literal(LiteralValue::Number(value)) => format!("number({value})"),
            ExpressionKind::Literal(LiteralValue::Float(value)) => format!("float({value})"),
            ExpressionKind::Literal(LiteralValue::Boolean(value)) => format!("bool({value})"),
            ExpressionKind::Literal(LiteralValue::Unit) => "unit".to_string(),
            ExpressionKind::Identifier(symbol) => format!("ident({})", resolve(interner, *symbol)),
            ExpressionKind::FunctionCall { function, args } => format!(
                "call({},{})",
                normalize_expr(module, interner, *function),
                normalize_args(module, interner, args)
            ),
            ExpressionKind::MethodCall {
                object,
                method,
                args,
            } => format!(
                "method({},{},{})",
                normalize_expr(module, interner, *object),
                resolve(interner, *method),
                normalize_args(module, interner, args)
            ),
            ExpressionKind::PropertyAccess { object, property } => format!(
                "prop({},{})",
                normalize_expr(module, interner, *object),
                resolve(interner, *property)
            ),
            ExpressionKind::Binary {
                op: BinaryOperator::Add,
                left,
                right,
            } => format!(
                "add({},{})",
                normalize_expr(module, interner, *left),
                normalize_expr(module, interner, *right)
            ),
            ExpressionKind::Binary { op, left, right } => format!(
                "binary({op:?},{},{})",
                normalize_expr(module, interner, *left),
                normalize_expr(module, interner, *right)
            ),
            other => format!("{other:?}"),
        }
    }

    fn normalize_args(
        module: &crate::interpreter::prelude::Module,
        interner: &SharedInterner,
        args: &[crate::ast::prelude::CallArg],
    ) -> String {
        args.iter()
            .map(|arg| {
                let name = arg
                    .name
                    .map(|name| resolve(interner, name))
                    .unwrap_or_else(|| "_".to_string());
                format!("{name}:{}", normalize_expr(module, interner, arg.value))
            })
            .collect::<Vec<_>>()
            .join(",")
    }

    fn resolve(interner: &SharedInterner, symbol: string_interner::DefaultSymbol) -> String {
        interner
            .read(|i| i.resolve(symbol).map(str::to_string))
            .unwrap_or_else(|| format!("<{:?}>", symbol))
    }
}
