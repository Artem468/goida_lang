mod helpers;

use crate::parser::syntax as syn;
use helpers::*;

pub(super) fn format_program(program: &syn::Program) -> String {
    let mut formatter = SourceFormatter::new(program.comments.clone());
    syn::Visitor::visit_program(&mut formatter, program);
    formatter.finish()
}

struct SourceFormatter {
    output: String,
    indent: usize,
    comments: Vec<syn::Comment>,
    next_comment: usize,
}

impl SourceFormatter {
    fn new(comments: Vec<syn::Comment>) -> Self {
        Self {
            output: String::new(),
            indent: 0,
            comments,
            next_comment: 0,
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
            syn::Visitor::visit_item(self, item);
        }
    }

    fn comments_before(&mut self, offset: usize) {
        while self
            .comments
            .get(self.next_comment)
            .is_some_and(|comment| comment.span.start <= offset)
        {
            let comment = self.comments[self.next_comment].clone();
            self.next_comment += 1;
            syn::Visitor::visit_comment(self, &comment);
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
            syn::ItemKind::MacroDefinition(definition) => self.macro_definition(definition),
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

    fn macro_definition(&mut self, definition: &syn::MacroDefinition) {
        self.line(format!("macro {} {{", definition.name));
        self.indent += 1;
        for rule in &definition.rules {
            self.line(format!(
                "({}) => {{ {} }};",
                format_macro_matchers(&rule.matcher),
                format_macro_template(&rule.template)
            ));
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

impl syn::Visitor for SourceFormatter {
    fn visit_program(&mut self, program: &syn::Program) {
        self.items(&program.items);
        self.comments_before(usize::MAX);
    }

    fn visit_item(&mut self, item: &syn::Item) {
        self.comments_before(item.span.start);
        self.item(item);
    }

    fn visit_comment(&mut self, comment: &syn::Comment) {
        self.line(format!("// {}", comment.text));
    }
}

#[cfg(test)]
mod tests {
    use super::format_program;
    use crate::parser::grammar;
    use crate::parser::lexer::lex;

    fn format(source: &str) -> String {
        let program = grammar::ProgramParser::new()
            .parse(lex(source))
            .expect("source should parse");
        format_program(&program)
    }

    #[test]
    fn preserves_binary_operator_precedence() {
        assert_eq!(format("value = 1 + 2 * 3\n"), "value = 1 + 2 * 3\n");
        assert_eq!(format("value = (1 + 2) * 3\n"), "value = (1 + 2) * 3\n");
    }

    #[test]
    fn escapes_string_literals() {
        assert_eq!(
            format("value = \"line\\nquote\\\"\"\n"),
            "value = \"line\\nquote\\\"\"\n"
        );
    }

    #[test]
    fn preserves_comments_through_ast_visitor() {
        let source = "// before\nvalue = 1 // trailing\n// after\n";
        let mut program = grammar::ProgramParser::new()
            .parse(lex(source))
            .expect("source should parse");
        program.comments = crate::parser::parser::collect_comments(source);

        assert_eq!(
            format_program(&program),
            "// before\nvalue = 1\n// trailing\n// after\n"
        );
    }

    #[test]
    fn formatted_macro_definition_is_parseable() {
        let source = "macro twice { ($x:expr) => { $x + $x }; }\nvalue = twice!(2)\n";
        let formatted = format(source);

        grammar::ProgramParser::new()
            .parse(lex(&formatted))
            .expect("formatted macro should remain parseable");
    }
}
