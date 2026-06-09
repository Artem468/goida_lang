use crate::document::char_offset_to_position;
use goida_model::SharedInterner;
use goida_runtime::interpreter::prelude::Module;
use goida_syntax::ast::prelude::{ExpressionKind, Span, StatementKind, StmtId};
use goida_syntax::ast::program::MethodType;
use std::cmp::min;
use tower_lsp::lsp_types::{SemanticToken, SemanticTokenModifier, SemanticTokenType};

pub(crate) const TOKEN_TYPES: &[SemanticTokenType] = &[
    SemanticTokenType::FUNCTION,
    SemanticTokenType::VARIABLE,
    SemanticTokenType::PARAMETER,
    SemanticTokenType::CLASS,
    SemanticTokenType::PROPERTY,
];
pub(crate) const TOKEN_MODIFIERS: &[SemanticTokenModifier] = &[SemanticTokenModifier::DECLARATION];

#[derive(Clone, Copy)]
pub(crate) struct SemanticTokenAbsolute {
    line: u32,
    start: u32,
    length: u32,
    token_type: u32,
    token_modifiers_bitset: u32,
}

pub(crate) fn collect_semantic_tokens(
    module: &Module,
    interner: &SharedInterner,
    text: &str,
    line_starts: &[usize],
) -> Vec<SemanticTokenAbsolute> {
    let mut tokens = Vec::new();

    for function in module.functions.values() {
        if let Some(name) = module.arena.resolve_symbol(interner, function.name) {
            push_name_token(
                &mut tokens,
                text,
                line_starts,
                function.span,
                &name,
                0,
                true,
            );
        }
        for param in &function.params {
            if let Some(name) = module.arena.resolve_symbol(interner, param.name) {
                push_name_token(&mut tokens, text, line_starts, param.span, &name, 2, true);
            }
        }
    }

    for class in module.classes.values() {
        class.read(|class_def| {
            if let Some(name) = module.arena.resolve_symbol(interner, class_def.name) {
                push_name_token(
                    &mut tokens,
                    text,
                    line_starts,
                    class_def.span,
                    &name,
                    3,
                    true,
                );
            }
        });
    }

    collect_statement_tokens(
        module,
        interner,
        &module.body,
        text,
        line_starts,
        &mut tokens,
    );
    tokens.sort_by_key(|tok| (tok.line, tok.start, tok.length, tok.token_type));
    tokens
}

fn push_name_token(
    out: &mut Vec<SemanticTokenAbsolute>,
    text: &str,
    line_starts: &[usize],
    span: Span,
    name: &str,
    token_type: u32,
    declaration: bool,
) {
    if let Some(range) = find_name_char_range(text, span, name) {
        if let Some(token) = char_range_to_semantic_token(
            line_starts,
            range.start,
            range.end,
            token_type,
            declaration,
        ) {
            out.push(token);
        }
    }
}

fn collect_statement_tokens(
    module: &Module,
    interner: &SharedInterner,
    statement_ids: &[StmtId],
    text: &str,
    line_starts: &[usize],
    out: &mut Vec<SemanticTokenAbsolute>,
) {
    for stmt_id in statement_ids {
        let Some(statement) = module.arena.get_statement(*stmt_id) else {
            continue;
        };

        match &statement.kind {
            StatementKind::Assign { name, value, .. } => {
                if let Some(name) = module.arena.resolve_symbol(interner, *name) {
                    push_name_token(out, text, line_starts, statement.span, &name, 1, true);
                }
                collect_expression_tokens(module, interner, *value, text, line_starts, out);
            }
            StatementKind::CompoundAssign { target, value, .. } => {
                collect_expression_tokens(module, interner, *target, text, line_starts, out);
                collect_expression_tokens(module, interner, *value, text, line_starts, out);
            }
            StatementKind::IndexAssign {
                object,
                index,
                value,
            } => {
                collect_expression_tokens(module, interner, *object, text, line_starts, out);
                collect_expression_tokens(module, interner, *index, text, line_starts, out);
                collect_expression_tokens(module, interner, *value, text, line_starts, out);
            }
            StatementKind::PropertyAssign {
                object,
                property,
                value,
            } => {
                collect_expression_tokens(module, interner, *object, text, line_starts, out);
                collect_expression_tokens(module, interner, *value, text, line_starts, out);
                if let Some(name) = module.arena.resolve_symbol(interner, *property) {
                    push_name_token(out, text, line_starts, statement.span, &name, 4, false);
                }
            }
            StatementKind::Expression(expr) => {
                collect_expression_tokens(module, interner, *expr, text, line_starts, out)
            }
            StatementKind::If {
                condition,
                then_body,
                else_body,
            } => {
                collect_expression_tokens(module, interner, *condition, text, line_starts, out);
                collect_statement_tokens(module, interner, &[*then_body], text, line_starts, out);
                if let Some(else_stmt) = else_body {
                    collect_statement_tokens(
                        module,
                        interner,
                        &[*else_stmt],
                        text,
                        line_starts,
                        out,
                    );
                }
            }
            StatementKind::While { condition, body } => {
                collect_expression_tokens(module, interner, *condition, text, line_starts, out);
                collect_statement_tokens(module, interner, &[*body], text, line_starts, out);
            }
            StatementKind::For {
                variable,
                init,
                condition,
                update,
                body,
            } => {
                if let Some(name) = module.arena.resolve_symbol(interner, *variable) {
                    push_name_token(out, text, line_starts, statement.span, &name, 1, true);
                }
                collect_expression_tokens(module, interner, *init, text, line_starts, out);
                collect_expression_tokens(module, interner, *condition, text, line_starts, out);
                collect_statement_tokens(module, interner, &[*update], text, line_starts, out);
                collect_statement_tokens(module, interner, &[*body], text, line_starts, out);
            }
            StatementKind::ForEach {
                variable,
                iterable,
                body,
            } => {
                if let Some(name) = module.arena.resolve_symbol(interner, *variable) {
                    push_name_token(out, text, line_starts, statement.span, &name, 1, true);
                }
                collect_expression_tokens(module, interner, *iterable, text, line_starts, out);
                collect_statement_tokens(module, interner, &[*body], text, line_starts, out);
            }
            StatementKind::Thread { body } => {
                collect_statement_tokens(module, interner, &[*body], text, line_starts, out);
            }
            StatementKind::Try { body, handlers } => {
                collect_statement_tokens(module, interner, &[*body], text, line_starts, out);
                for handler in handlers {
                    collect_statement_tokens(
                        module,
                        interner,
                        &[handler.body],
                        text,
                        line_starts,
                        out,
                    );
                }
            }
            StatementKind::Raise { message, .. } => {
                if let StatementKind::Raise { error_type, .. } = &statement.kind {
                    if let Some(name) = module.arena.resolve_symbol(interner, *error_type) {
                        push_name_token(out, text, line_starts, statement.span, &name, 3, false);
                    }
                }
                if let Some(message) = message {
                    collect_expression_tokens(module, interner, *message, text, line_starts, out);
                }
            }
            StatementKind::Block(items) => {
                collect_statement_tokens(module, interner, items, text, line_starts, out)
            }
            StatementKind::Return(expr) => {
                if let Some(expr_id) = expr {
                    collect_expression_tokens(module, interner, *expr_id, text, line_starts, out);
                }
            }
            StatementKind::FunctionDefinition(function) => {
                if let Some(name) = module.arena.resolve_symbol(interner, function.name) {
                    push_name_token(out, text, line_starts, function.span, &name, 0, true);
                }
                for param in &function.params {
                    if let Some(name) = module.arena.resolve_symbol(interner, param.name) {
                        push_name_token(out, text, line_starts, param.span, &name, 2, true);
                    }
                }
                collect_statement_tokens(
                    module,
                    interner,
                    &[function.body],
                    text,
                    line_starts,
                    out,
                );
            }
            StatementKind::ClassDefinition(class_def) => {
                if let Some(name) = module.arena.resolve_symbol(interner, class_def.name) {
                    push_name_token(out, text, line_starts, class_def.span, &name, 3, true);
                }
                if let Some(base_class) = class_def.base_class {
                    if let Some(name) = module.arena.resolve_symbol(interner, base_class) {
                        push_name_token(out, text, line_starts, class_def.span, &name, 3, false);
                    }
                }
                if let Some(MethodType::User(constructor)) = &class_def.constructor {
                    for param in &constructor.params {
                        if let Some(name) = module.arena.resolve_symbol(interner, param.name) {
                            push_name_token(out, text, line_starts, param.span, &name, 2, true);
                        }
                    }
                    collect_statement_tokens(
                        module,
                        interner,
                        &[constructor.body],
                        text,
                        line_starts,
                        out,
                    );
                }
                for (method_name, (_, _, method_type)) in &class_def.methods {
                    if let (Some(name), MethodType::User(user_method)) = (
                        module.arena.resolve_symbol(interner, *method_name),
                        method_type,
                    ) {
                        push_name_token(out, text, line_starts, user_method.span, &name, 0, true);
                        for param in &user_method.params {
                            if let Some(param_name) =
                                module.arena.resolve_symbol(interner, param.name)
                            {
                                push_name_token(
                                    out,
                                    text,
                                    line_starts,
                                    param.span,
                                    &param_name,
                                    2,
                                    true,
                                );
                            }
                        }
                        collect_statement_tokens(
                            module,
                            interner,
                            &[user_method.body],
                            text,
                            line_starts,
                            out,
                        );
                    }
                }
            }
            StatementKind::NativeLibraryDefinition(_) | StatementKind::Empty => {}
        }
    }
}

fn collect_expression_tokens(
    module: &Module,
    interner: &SharedInterner,
    expr_id: u32,
    text: &str,
    line_starts: &[usize],
    out: &mut Vec<SemanticTokenAbsolute>,
) {
    let Some(expr) = module.arena.get_expression(expr_id) else {
        return;
    };
    match &expr.kind {
        ExpressionKind::Identifier(symbol) => {
            if let Some(name) = module.arena.resolve_symbol(interner, *symbol) {
                push_name_token(out, text, line_starts, expr.span, &name, 1, false);
            }
        }
        ExpressionKind::FunctionCall { function, args } => {
            if let Some(function_expr) = module.arena.get_expression(*function) {
                if let ExpressionKind::Identifier(symbol) = function_expr.kind {
                    if let Some(name) = module.arena.resolve_symbol(interner, symbol) {
                        push_name_token(
                            out,
                            text,
                            line_starts,
                            function_expr.span,
                            &name,
                            0,
                            false,
                        );
                    }
                } else {
                    collect_expression_tokens(module, interner, *function, text, line_starts, out);
                }
            }
            for arg in args {
                collect_expression_tokens(module, interner, arg.value, text, line_starts, out);
            }
        }
        ExpressionKind::MethodCall {
            object,
            method,
            args,
        } => {
            collect_expression_tokens(module, interner, *object, text, line_starts, out);
            if let Some(name) = module.arena.resolve_symbol(interner, *method) {
                push_name_token(out, text, line_starts, expr.span, &name, 4, false);
            }
            for arg in args {
                collect_expression_tokens(module, interner, arg.value, text, line_starts, out);
            }
        }
        ExpressionKind::PropertyAccess { object, property } => {
            collect_expression_tokens(module, interner, *object, text, line_starts, out);
            if let Some(name) = module.arena.resolve_symbol(interner, *property) {
                push_name_token(out, text, line_starts, expr.span, &name, 4, false);
            }
        }
        ExpressionKind::ObjectCreation { class_name, args } => {
            if let Some(name) = module.arena.resolve_symbol(interner, *class_name) {
                push_name_token(out, text, line_starts, expr.span, &name, 3, false);
            }
            for arg in args {
                collect_expression_tokens(module, interner, arg.value, text, line_starts, out);
            }
        }
        ExpressionKind::Binary { left, right, .. } => {
            collect_expression_tokens(module, interner, *left, text, line_starts, out);
            collect_expression_tokens(module, interner, *right, text, line_starts, out);
        }
        ExpressionKind::Unary { operand, .. } => {
            collect_expression_tokens(module, interner, *operand, text, line_starts, out);
        }
        ExpressionKind::Index { object, index } => {
            collect_expression_tokens(module, interner, *object, text, line_starts, out);
            collect_expression_tokens(module, interner, *index, text, line_starts, out);
        }
        ExpressionKind::Lambda { params, body } => {
            for param in params {
                if let Some(name) = module.arena.resolve_symbol(interner, param.name) {
                    push_name_token(out, text, line_starts, param.span, &name, 2, true);
                }
                if let Some(default_value) = param.default_value {
                    collect_expression_tokens(
                        module,
                        interner,
                        default_value,
                        text,
                        line_starts,
                        out,
                    );
                }
            }
            collect_statement_tokens(module, interner, &[*body], text, line_starts, out);
        }
        ExpressionKind::Literal(_) | ExpressionKind::This => {}
    }
}

fn find_name_char_range(text: &str, span: Span, name: &str) -> Option<std::ops::Range<usize>> {
    let start = span.start as usize;
    let end = min(span.end as usize, text.len());
    if start >= end || !text.is_char_boundary(start) || !text.is_char_boundary(end) {
        return None;
    }

    let segment = &text[start..end];
    let local_match = segment.find(name)?;
    let absolute_start = start + local_match;
    let absolute_end = absolute_start + name.len();

    if !text.is_char_boundary(absolute_start) || !text.is_char_boundary(absolute_end) {
        return None;
    }

    let char_start = text[..absolute_start].chars().count();
    let char_end = text[..absolute_end].chars().count();
    Some(char_start..char_end)
}

fn char_range_to_semantic_token(
    line_starts: &[usize],
    start_char: usize,
    end_char: usize,
    token_type: u32,
    declaration: bool,
) -> Option<SemanticTokenAbsolute> {
    if start_char >= end_char {
        return None;
    }
    let start = char_offset_to_position(line_starts, start_char)?;
    let end = char_offset_to_position(line_starts, end_char)?;
    if start.line != end.line {
        return None;
    }

    Some(SemanticTokenAbsolute {
        line: start.line,
        start: start.character,
        length: end.character.saturating_sub(start.character),
        token_type,
        token_modifiers_bitset: if declaration { 1 } else { 0 },
    })
}

pub(crate) fn encode_semantic_tokens(mut tokens: Vec<SemanticTokenAbsolute>) -> Vec<SemanticToken> {
    tokens.sort_by_key(|tok| (tok.line, tok.start, tok.length, tok.token_type));
    tokens.dedup_by_key(|tok| (tok.line, tok.start, tok.length, tok.token_type));

    let mut encoded = Vec::with_capacity(tokens.len());
    let mut prev_line = 0u32;
    let mut prev_start = 0u32;

    for (idx, tok) in tokens.into_iter().enumerate() {
        let (delta_line, delta_start) = if idx == 0 {
            (tok.line, tok.start)
        } else if tok.line == prev_line {
            (0, tok.start.saturating_sub(prev_start))
        } else {
            (tok.line.saturating_sub(prev_line), tok.start)
        };

        encoded.push(SemanticToken {
            delta_line,
            delta_start,
            length: tok.length,
            token_type: tok.token_type,
            token_modifiers_bitset: tok.token_modifiers_bitset,
        });

        prev_line = tok.line;
        prev_start = tok.start;
    }

    encoded
}
