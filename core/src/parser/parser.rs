use crate::ast::prelude::*;
use crate::interpreter::prelude::{Module, SharedInterner};
use crate::parser::grammar;
use crate::parser::lexer::{lex, LexicalError, Token};
use crate::parser::prelude::{ParseError, Parser as ParserTrait};
use crate::parser::syntax as syn;
use lalrpop_util::ParseError as LalrpopParseError;
use std::path::PathBuf;

impl ParserTrait {
    pub fn new(interner: SharedInterner, name: &str, path: PathBuf) -> Self {
        Self {
            module: Module::new(&interner, name, path),
            interner,
        }
    }

    pub fn parse(mut self, code: &str) -> Result<Module, ParseError> {
        self.module.arena.init_builtin_types(&self.interner);
        self.init_builtin_error_classes();

        self.parse_into_module(code)?;
        self.validate_module_names()?;
        self.module.arena.optimize_all(&self.interner);
        Ok(self.module)
    }

    pub fn parse_unvalidated(mut self, code: &str) -> Result<Module, ParseError> {
        self.module.arena.init_builtin_types(&self.interner);
        self.init_builtin_error_classes();
        self.parse_into_module(code)?;
        self.module.arena.optimize_all(&self.interner);
        Ok(self.module)
    }

    pub fn macro_expansion_preview(&self, code: &str) -> Result<String, ParseError> {
        let syntax = grammar::ProgramParser::new()
            .parse(lex(code))
            .map_err(|err| self.convert_parse_error(code, err))?;
        let syntax = self.expand_macros(syntax)?;
        Ok(format_program(&syntax))
    }

    fn parse_into_module(&mut self, code: &str) -> Result<(), ParseError> {
        let syntax = grammar::ProgramParser::new()
            .parse(lex(code))
            .map_err(|err| self.convert_parse_error(code, err))?;
        let syntax = self.expand_macros(syntax)?;
        self.build_program(syntax)
    }

    fn convert_parse_error(
        &self,
        code: &str,
        err: LalrpopParseError<usize, Token, LexicalError>,
    ) -> ParseError {
        match err {
            LalrpopParseError::InvalidToken { location } => {
                let (start, end) = token_range_at(code, location);
                ParseError::InvalidSyntax(ErrorData::new(
                    Span::new(start, end, self.module.name),
                    "Некорректный токен".into(),
                ))
            }
            LalrpopParseError::UnrecognizedEof { location, expected } => {
                ParseError::InvalidSyntax(ErrorData::new(
                    Span::new(location, location, self.module.name),
                    format_expected("Неожиданный конец файла", expected),
                ))
            }
            LalrpopParseError::UnrecognizedToken { token, expected } => {
                let (start, found, end) = token;
                ParseError::InvalidSyntax(ErrorData::new(
                    Span::new(start, end, self.module.name),
                    format_expected(
                        format!("Неожиданный токен {}", token_name(&found)),
                        expected,
                    ),
                ))
            }
            LalrpopParseError::ExtraToken { token } => {
                let (start, found, end) = token;
                ParseError::InvalidSyntax(ErrorData::new(
                    Span::new(start, end, self.module.name),
                    format!("Лишний токен {}", token_name(&found)),
                ))
            }
            LalrpopParseError::User { error } => ParseError::InvalidSyntax(ErrorData::new(
                Span::new(error.span.start, error.span.end, self.module.name),
                error.message,
            )),
        }
    }
}

fn format_program(program: &syn::Program) -> String {
    let mut formatter = SourceFormatter::new();
    formatter.items(&program.items);
    formatter.finish()
}

struct SourceFormatter {
    output: String,
    indent: usize,
}

impl SourceFormatter {
    fn new() -> Self {
        Self {
            output: String::new(),
            indent: 0,
        }
    }

    fn finish(self) -> String {
        self.output
    }

    fn line(&mut self, text: impl AsRef<str>) {
        self.output.push_str(&"    ".repeat(self.indent));
        self.output.push_str(text.as_ref());
        self.output.push('\n');
    }

    fn items(&mut self, items: &[syn::Item]) {
        for item in items {
            self.item(item);
        }
    }

    fn item(&mut self, item: &syn::Item) {
        match &item.node {
            syn::ItemKind::Import(import) => {
                self.line(format!(
                    "import {} as {}",
                    string_literal(&import.path),
                    import.alias
                ));
            }
            syn::ItemKind::Function(function) => self.function(function),
            syn::ItemKind::Class(class) => self.class(class),
            syn::ItemKind::Library(library) => self.library(library),
            syn::ItemKind::MacroDefinition(_) => {}
            syn::ItemKind::Statement(stmt) => self.stmt(stmt),
        }
    }

    fn function(&mut self, function: &syn::Function) {
        let return_type = function
            .return_type
            .as_ref()
            .map(|ty| format!(" -> {ty}"))
            .unwrap_or_default();
        self.line(format!(
            "function {}({}){} {{",
            function.name,
            format_params(&function.params),
            return_type
        ));
        self.indent += 1;
        self.items(&function.body);
        self.indent -= 1;
        self.line("}");
    }

    fn class(&mut self, class: &syn::Class) {
        let base = class
            .base
            .as_ref()
            .map(|base| format!("({base})"))
            .unwrap_or_default();
        self.line(format!("class {}{} {{", class.name, base));
        self.indent += 1;
        for item in &class.items {
            self.class_item(item);
        }
        self.indent -= 1;
        self.line("}");
    }

    fn class_item(&mut self, item: &syn::ClassItem) {
        match &item.node {
            syn::ClassItemKind::Field(field) => {
                let mut parts = modifiers(field.visibility.clone(), field.is_static);
                parts.push(format!("{}: {}", field.name, field.type_name));
                let mut line = parts.join(" ");
                if let Some(value) = &field.default_value {
                    line.push_str(" = ");
                    line.push_str(&expr(value));
                }
                self.line(line);
            }
            syn::ClassItemKind::Constructor(method) => {
                self.class_method("constructor", method);
            }
            syn::ClassItemKind::Method(method) => {
                self.class_method("function", method);
            }
        }
    }

    fn class_method(&mut self, keyword: &str, method: &syn::ClassMethod) {
        let mut parts = modifiers(method.visibility.clone(), method.is_static);
        parts.push(format!(
            "{} {}({}){}",
            keyword,
            method.name,
            format_params(&method.params),
            method
                .return_type
                .as_ref()
                .map(|ty| format!(" -> {ty}"))
                .unwrap_or_default()
        ));
        self.line(format!("{} {{", parts.join(" ")));
        self.indent += 1;
        self.items(&method.body);
        self.indent -= 1;
        self.line("}");
    }

    fn library(&mut self, library: &syn::Library) {
        self.line(format!("library {} {{", string_literal(&library.path)));
        self.indent += 1;
        for item in &library.items {
            match &item.node {
                syn::LibraryItemKind::Function(function) => {
                    let return_type = function
                        .return_type
                        .as_ref()
                        .map(|ty| format!(" -> {ty}"))
                        .unwrap_or_default();
                    let params = function
                        .params
                        .iter()
                        .map(|param| format!("{}: {}", param.name, param.type_name))
                        .collect::<Vec<_>>()
                        .join(", ");
                    self.line(format!(
                        "function {}({}){}",
                        function.name, params, return_type
                    ));
                }
                syn::LibraryItemKind::Global(global) => {
                    self.line(format!("variable {}: {}", global.name, global.type_name));
                }
            }
        }
        self.indent -= 1;
        self.line("}");
    }

    fn stmt(&mut self, stmt: &syn::Stmt) {
        match &stmt.node {
            syn::StmtKind::Assign {
                name,
                is_const,
                type_hint,
                value,
            } => {
                let prefix = if *is_const { "const " } else { "" };
                let type_hint = type_hint
                    .as_ref()
                    .map(|ty| format!(": {ty}"))
                    .unwrap_or_default();
                self.line(format!("{prefix}{name}{type_hint} = {}", expr(value)));
            }
            syn::StmtKind::AssignTarget { target, value } => {
                self.line(format!("{} = {}", expr(target), expr(value)));
            }
            syn::StmtKind::CompoundAssign { target, op, value } => {
                self.line(format!(
                    "{} {} {}",
                    expr(target),
                    compound_op(*op),
                    expr(value)
                ));
            }
            syn::StmtKind::If {
                condition,
                then_body,
                else_body,
            } => self.if_stmt(condition, then_body, else_body.as_ref()),
            syn::StmtKind::While { condition, body } => {
                self.line(format!("while ({}) {{", expr(condition)));
                self.indent += 1;
                self.items(body);
                self.indent -= 1;
                self.line("}");
            }
            syn::StmtKind::For {
                variable,
                init,
                condition,
                update,
                body,
            } => {
                self.line(format!(
                    "for ({} = {}, {}, {}) {{",
                    variable,
                    expr(init),
                    expr(condition),
                    for_update(update)
                ));
                self.indent += 1;
                self.items(body);
                self.indent -= 1;
                self.line("}");
            }
            syn::StmtKind::ForEach {
                variable,
                iterable,
                body,
            } => {
                self.line(format!("for {} from {} {{", variable, expr(iterable)));
                self.indent += 1;
                self.items(body);
                self.indent -= 1;
                self.line("}");
            }
            syn::StmtKind::Thread { body } => {
                self.line("thread {");
                self.indent += 1;
                self.items(body);
                self.indent -= 1;
                self.line("}");
            }
            syn::StmtKind::Try { body, handlers } => {
                self.line("try {");
                self.indent += 1;
                self.items(body);
                self.indent -= 1;
                self.line("}");
                for handler in handlers {
                    self.line(format!("catch{} {{", catch_pattern(&handler.pattern)));
                    self.indent += 1;
                    self.items(&handler.body);
                    self.indent -= 1;
                    self.line("}");
                }
            }
            syn::StmtKind::Raise {
                error_type,
                message,
            } => {
                if let Some(message) = message {
                    self.line(format!("raise {}({})", error_type, expr(message)));
                } else {
                    self.line(format!("raise {error_type}"));
                }
            }
            syn::StmtKind::Return(value) => {
                if let Some(value) = value {
                    self.line(format!("return {}", expr(value)));
                } else {
                    self.line("return");
                }
            }
            syn::StmtKind::Expr(value) => self.line(expr(value)),
        }
    }

    fn if_stmt(
        &mut self,
        condition: &syn::Expr,
        then_body: &[syn::Item],
        else_body: Option<&syn::ElseBody>,
    ) {
        self.line(format!("if ({}) {{", expr(condition)));
        self.indent += 1;
        self.items(then_body);
        self.indent -= 1;
        match else_body {
            Some(syn::ElseBody::Block(body, _)) => {
                self.line("} else {");
                self.indent += 1;
                self.items(body);
                self.indent -= 1;
                self.line("}");
            }
            Some(syn::ElseBody::If(stmt)) => {
                self.output.push_str(&"    ".repeat(self.indent));
                self.output.push_str("} else ");
                self.inline_if(stmt);
            }
            None => self.line("}"),
        }
    }

    fn inline_if(&mut self, stmt: &syn::Stmt) {
        let syn::StmtKind::If {
            condition,
            then_body,
            else_body,
        } = &stmt.node
        else {
            self.output.push('\n');
            self.stmt(stmt);
            return;
        };
        self.output
            .push_str(&format!("if ({}) {{\n", expr(condition)));
        self.indent += 1;
        self.items(then_body);
        self.indent -= 1;
        match else_body {
            Some(syn::ElseBody::Block(body, _)) => {
                self.line("} else {");
                self.indent += 1;
                self.items(body);
                self.indent -= 1;
                self.line("}");
            }
            Some(syn::ElseBody::If(stmt)) => {
                self.output.push_str(&"    ".repeat(self.indent));
                self.output.push_str("} else ");
                self.inline_if(stmt);
            }
            None => self.line("}"),
        }
    }
}

fn format_params(params: &[syn::Param]) -> String {
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

fn modifiers(visibility: Option<syn::Visibility>, is_static: bool) -> Vec<String> {
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

fn catch_pattern(pattern: &Option<syn::CatchPattern>) -> String {
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

fn for_update(update: &syn::ForUpdate) -> String {
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

fn compound_op(op: syn::CompoundOp) -> &'static str {
    match op {
        syn::CompoundOp::Add => "+=",
        syn::CompoundOp::Sub => "-=",
        syn::CompoundOp::Mul => "*=",
        syn::CompoundOp::Div => "/=",
        syn::CompoundOp::Mod => "%=",
    }
}

fn expr(value: &syn::Expr) -> String {
    expr_with_parent_prec(value, 0, false)
}

fn expr_with_parent_prec(value: &syn::Expr, parent_prec: u8, is_right: bool) -> String {
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
                    let mut formatter = SourceFormatter::new();
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

fn expr_prec(expr: &syn::Expr) -> u8 {
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

fn binary_prec(op: syn::BinaryOp) -> u8 {
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

fn binary_op(op: syn::BinaryOp) -> &'static str {
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

fn unary_op(op: syn::UnaryOp) -> &'static str {
    match op {
        syn::UnaryOp::Negative => "-",
        syn::UnaryOp::Not => "!",
    }
}

fn format_args(args: &[syn::CallArg]) -> String {
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

fn macro_call_args(call: &syn::MacroCall) -> String {
    let (open, close) = match call.delimiter {
        syn::MacroDelimiter::Paren => ('(', ')'),
        syn::MacroDelimiter::Bracket => ('[', ']'),
        syn::MacroDelimiter::Brace => ('{', '}'),
    };
    let args = call
        .args
        .iter()
        .map(|token| token_name(&token.token))
        .collect::<Vec<_>>()
        .join(" ");
    format!("{open}{args}{close}")
}

fn string_literal(value: &str) -> String {
    format!("{value:?}")
}

fn token_range_at(code: &str, location: usize) -> (usize, usize) {
    let start = previous_char_boundary(code, location.min(code.len()));
    let mut end = next_char_boundary(code, location.min(code.len()));
    if end == start && end < code.len() {
        end = next_char_boundary(code, end + 1);
    }
    (start, end)
}

fn previous_char_boundary(s: &str, mut index: usize) -> usize {
    index = index.min(s.len());
    while index > 0 && !s.is_char_boundary(index) {
        index -= 1;
    }
    index
}

fn next_char_boundary(s: &str, mut index: usize) -> usize {
    index = index.min(s.len());
    while index < s.len() && !s.is_char_boundary(index) {
        index += 1;
    }
    index
}

fn format_expected(prefix: impl Into<String>, expected: Vec<String>) -> String {
    if expected.is_empty() {
        prefix.into()
    } else {
        format!("{}; ожидалось: {}", prefix.into(), expected.join(", "))
    }
}

fn token_name(token: &Token) -> String {
    match token {
        Token::Ident(value) => format!("'{}'", value),
        Token::String(value) => format!("\"{}\"", value),
        Token::Number(value) => value.to_string(),
        Token::Float(value) => value.to_string(),
        other => format!("{:?}", other),
    }
}

#[cfg(test)]
mod tests {
    use crate::interpreter::prelude::SharedInterner;
    use crate::parser::prelude::Parser;
    use crate::shared::SharedMut;
    use std::path::PathBuf;
    use string_interner::StringInterner;

    #[test]
    fn macro_expansion_preview_contains_expanded_source_without_macro_definition() {
        let interner: SharedInterner = SharedMut::new(StringInterner::new());
        let parser = Parser::new(interner, "preview_test", PathBuf::from("preview.goida"));
        let preview = parser
            .macro_expansion_preview(
                r#"
macro twice {
    ($x:expr) => { $x + $x };
}

value = twice!(2)
"#,
            )
            .expect("macro preview should expand");

        assert!(preview.contains("value = 2 + 2"));
        assert!(!preview.contains("macro twice"));
    }
}
