mod helpers;

use crate::parser::structs::FormatLanguage;
use crate::parser::syntax as syn;
use helpers::*;

pub(super) fn format_program(program: &syn::Program, language: FormatLanguage) -> String {
    let mut formatter = SourceFormatter::new(program.comments.clone(), language);
    syn::Visitor::visit_program(&mut formatter, program);
    formatter.finish()
}

struct SourceFormatter {
    output: String,
    indent: usize,
    comments: Vec<syn::Comment>,
    next_comment: usize,
    language: FormatLanguage,
}

impl SourceFormatter {
    fn new(comments: Vec<syn::Comment>, language: FormatLanguage) -> Self {
        Self {
            output: String::new(),
            indent: 0,
            comments,
            next_comment: 0,
            language,
        }
    }

    fn keyword(&self, english: &'static str, russian: &'static str) -> &'static str {
        self.language.select(english, russian)
    }

    fn finish(self) -> String {
        self.output
    }

    fn line(&mut self, text: impl AsRef<str>) {
        self.output.push_str(&"    ".repeat(self.indent));
        self.output.push_str(text.as_ref());
        self.output.push('\n');
    }

    fn blank_lines(&mut self, count: usize) {
        if self.output.is_empty() {
            return;
        }

        let trailing_newlines = self
            .output
            .as_bytes()
            .iter()
            .rev()
            .take_while(|byte| **byte == b'\n')
            .count();
        let required_newlines = count + 1;
        for _ in trailing_newlines..required_newlines {
            self.output.push('\n');
        }
    }

    fn items(&mut self, items: &[syn::Item]) {
        for item in items {
            syn::Visitor::visit_item(self, item);
        }
    }

    fn top_level_items(&mut self, items: &[syn::Item]) {
        for (index, item) in items.iter().enumerate() {
            if index > 0
                && (is_top_level_definition(&items[index - 1]) || is_top_level_definition(item))
            {
                self.blank_lines(2);
            }
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
                    "{} {} {} {}",
                    self.keyword("import", "подключить"),
                    string_literal(&import.path),
                    self.keyword("as", "как"),
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
            "{} {}({}){} {{",
            self.keyword("function", "функция"),
            function.name,
            format_params(&function.params, self.language),
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
        self.line(format!(
            "{} {}{} {{",
            self.keyword("class", "класс"),
            class.name,
            base
        ));
        self.indent += 1;
        for (index, item) in class.items.iter().enumerate() {
            if index > 0 && is_class_method(&class.items[index - 1]) && is_class_method(item) {
                self.blank_lines(1);
            }
            self.comments_before(item.span.start);
            self.class_item(item);
        }
        self.indent -= 1;
        self.line("}");
    }

    fn class_item(&mut self, item: &syn::ClassItem) {
        match &item.node {
            syn::ClassItemKind::Field(field) => {
                let mut parts = modifiers(field.visibility.clone(), field.is_static, self.language);
                parts.push(format!("{}: {}", field.name, field.type_name));
                let mut line = parts.join(" ");
                if let Some(value) = &field.default_value {
                    line.push_str(" = ");
                    line.push_str(&expr(value, self.language));
                }
                self.line(line);
            }
            syn::ClassItemKind::Constructor(method) => {
                self.class_method(self.keyword("constructor", "конструктор"), method, true);
            }
            syn::ClassItemKind::Method(method) => {
                self.class_method(self.keyword("function", "функция"), method, false);
            }
        }
    }

    fn class_method(&mut self, keyword: &str, method: &syn::ClassMethod, is_constructor: bool) {
        let mut parts = modifiers(method.visibility.clone(), method.is_static, self.language);
        let name = if is_constructor && method.name == "new" {
            self.keyword("new", "новый")
        } else {
            &method.name
        };
        parts.push(format!(
            "{} {}({}){}",
            keyword,
            name,
            format_params(&method.params, self.language),
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
        self.line(format!(
            "{} {} {{",
            self.keyword("library", "библиотека"),
            string_literal(&library.path)
        ));
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
                        "{} {}({}){}",
                        self.keyword("function", "функция"),
                        function.name,
                        params,
                        return_type
                    ));
                }
                syn::LibraryItemKind::Global(global) => {
                    self.line(format!(
                        "{} {}: {}",
                        self.keyword("variable", "переменная"),
                        global.name,
                        global.type_name
                    ));
                }
            }
        }
        self.indent -= 1;
        self.line("}");
    }

    fn macro_definition(&mut self, definition: &syn::MacroDefinition) {
        self.line(format!(
            "{} {} {{",
            self.keyword("macro", "макрос"),
            definition.name
        ));
        self.indent += 1;
        for rule in &definition.rules {
            self.line(format!(
                "({}) => {{ {} }};",
                format_macro_matchers(&rule.matcher, self.language),
                format_macro_template(&rule.template, self.language)
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
                let prefix = if *is_const {
                    format!("{} ", self.keyword("const", "константа"))
                } else {
                    String::new()
                };
                let type_hint = type_hint
                    .as_ref()
                    .map(|ty| format!(": {ty}"))
                    .unwrap_or_default();
                self.line(format!(
                    "{prefix}{name}{type_hint} = {}",
                    expr(value, self.language)
                ));
            }
            syn::StmtKind::AssignTarget { target, value } => {
                self.line(format!(
                    "{} = {}",
                    expr(target, self.language),
                    expr(value, self.language)
                ));
            }
            syn::StmtKind::CompoundAssign { target, op, value } => {
                self.line(format!(
                    "{} {} {}",
                    expr(target, self.language),
                    compound_op(*op),
                    expr(value, self.language)
                ));
            }
            syn::StmtKind::If {
                condition,
                then_body,
                else_body,
            } => self.if_stmt(condition, then_body, else_body.as_ref()),
            syn::StmtKind::While { condition, body } => {
                self.line(format!(
                    "{} ({}) {{",
                    self.keyword("while", "пока"),
                    expr(condition, self.language)
                ));
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
                    "{} ({} = {}, {}, {}) {{",
                    self.keyword("for", "для"),
                    variable,
                    expr(init, self.language),
                    expr(condition, self.language),
                    for_update(update, self.language)
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
                self.line(format!(
                    "{} {} {} {} {{",
                    self.keyword("for", "для"),
                    variable,
                    self.keyword("from", "из"),
                    expr(iterable, self.language)
                ));
                self.indent += 1;
                self.items(body);
                self.indent -= 1;
                self.line("}");
            }
            syn::StmtKind::Thread { body } => {
                self.line(format!("{} {{", self.keyword("thread", "поток")));
                self.indent += 1;
                self.items(body);
                self.indent -= 1;
                self.line("}");
            }
            syn::StmtKind::Try { body, handlers } => {
                self.line(format!("{} {{", self.keyword("try", "попробовать")));
                self.indent += 1;
                self.items(body);
                self.indent -= 1;
                self.line("}");
                for handler in handlers {
                    self.line(format!(
                        "{}{} {{",
                        self.keyword("catch", "перехватить"),
                        catch_pattern(&handler.pattern, self.language)
                    ));
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
                    self.line(format!(
                        "{} {}({})",
                        self.keyword("raise", "выбросить"),
                        error_type,
                        expr(message, self.language)
                    ));
                } else {
                    self.line(format!(
                        "{} {error_type}",
                        self.keyword("raise", "выбросить")
                    ));
                }
            }
            syn::StmtKind::Return(value) => {
                if let Some(value) = value {
                    self.line(format!(
                        "{} {}",
                        self.keyword("return", "вернуть"),
                        expr(value, self.language)
                    ));
                } else {
                    self.line(self.keyword("return", "вернуть"));
                }
            }
            syn::StmtKind::Expr(value) => self.line(expr(value, self.language)),
        }
    }

    fn if_stmt(
        &mut self,
        condition: &syn::Expr,
        then_body: &[syn::Item],
        else_body: Option<&syn::ElseBody>,
    ) {
        self.line(format!(
            "{} ({}) {{",
            self.keyword("if", "если"),
            expr(condition, self.language)
        ));
        self.indent += 1;
        self.items(then_body);
        self.indent -= 1;
        match else_body {
            Some(syn::ElseBody::Block(body, _)) => {
                self.line(format!("}} {} {{", self.keyword("else", "иначе")));
                self.indent += 1;
                self.items(body);
                self.indent -= 1;
                self.line("}");
            }
            Some(syn::ElseBody::If(stmt)) => {
                self.output.push_str(&"    ".repeat(self.indent));
                self.output
                    .push_str(&format!("}} {} ", self.keyword("else", "иначе")));
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
        self.output.push_str(&format!(
            "{} ({}) {{\n",
            self.keyword("if", "если"),
            expr(condition, self.language)
        ));
        self.indent += 1;
        self.items(then_body);
        self.indent -= 1;
        match else_body {
            Some(syn::ElseBody::Block(body, _)) => {
                self.line(format!("}} {} {{", self.keyword("else", "иначе")));
                self.indent += 1;
                self.items(body);
                self.indent -= 1;
                self.line("}");
            }
            Some(syn::ElseBody::If(stmt)) => {
                self.output.push_str(&"    ".repeat(self.indent));
                self.output
                    .push_str(&format!("}} {} ", self.keyword("else", "иначе")));
                self.inline_if(stmt);
            }
            None => self.line("}"),
        }
    }
}

impl syn::Visitor for SourceFormatter {
    fn visit_program(&mut self, program: &syn::Program) {
        self.top_level_items(&program.items);
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

fn is_top_level_definition(item: &syn::Item) -> bool {
    matches!(
        item.node,
        syn::ItemKind::Function(_) | syn::ItemKind::Class(_)
    )
}

fn is_class_method(item: &syn::ClassItem) -> bool {
    matches!(
        item.node,
        syn::ClassItemKind::Constructor(_) | syn::ClassItemKind::Method(_)
    )
}

#[cfg(test)]
mod tests {
    use super::format_program;
    use crate::parser::grammar;
    use crate::parser::lexer::lex;
    use crate::parser::structs::FormatLanguage;

    fn format(source: &str) -> String {
        let program = grammar::ProgramParser::new()
            .parse(lex(source))
            .expect("source should parse");
        format_program(&program, FormatLanguage::English)
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
            format_program(&program, FormatLanguage::English),
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

    #[test]
    fn separates_top_level_definitions_with_two_blank_lines() {
        let source =
            "import \"mod.goida\" as mod\nfunction first() {}\nclass Item {}\nvalue = first()\n";
        let expected = "import \"mod.goida\" as mod\n\n\nfunction first() {\n}\n\n\nclass Item {\n}\n\n\nvalue = first()\n";

        assert_eq!(format(source), expected);
        assert_eq!(format(expected), expected);
    }

    #[test]
    fn separates_class_methods_with_one_blank_line() {
        let source = "class Item {\nvalue: number\nconstructor Item(this) {\n}\nfunction get(this) -> number {\nreturn this.value\n}\n}\n";
        let expected = "class Item {\n    value: number\n    constructor Item(this) {\n    }\n\n    function get(this) -> number {\n        return this.value\n    }\n}\n";

        assert_eq!(format(source), expected);
        assert_eq!(format(expected), expected);
    }

    #[test]
    fn keeps_comments_with_the_following_class_method() {
        let source = "class Item {\nfunction first(this) {\n}\n// second method\nfunction second(this) {\n}\n}\n";
        let mut program = grammar::ProgramParser::new()
            .parse(lex(source))
            .expect("source should parse");
        program.comments = crate::parser::parser::collect_comments(source);

        assert_eq!(
            format_program(&program, FormatLanguage::English),
            "class Item {\n    function first(this) {\n    }\n\n    // second method\n    function second(this) {\n    }\n}\n"
        );
    }

    #[test]
    fn renders_russian_keywords_throughout_the_ast() {
        let source = "import \"mod.goida\" as mod\nconst enabled = true and !false\nfunction make() { if (enabled) { return new Item() } else { return void } }\nclass Item {\nconstructor new(this) {}\npublic static function get(this) { return this }\n}\n";
        let program = grammar::ProgramParser::new()
            .parse(lex(source))
            .expect("source should parse");
        let formatted = format_program(&program, FormatLanguage::Russian);

        assert!(formatted.contains("подключить \"mod.goida\" как mod"));
        assert!(formatted.contains("константа enabled = истина и !ложь"));
        assert!(formatted.contains("функция make()"));
        assert!(formatted.contains("если (enabled)"));
        assert!(formatted.contains("вернуть новый Item()"));
        assert!(formatted.contains("иначе"));
        assert!(formatted.contains("вернуть пустота"));
        assert!(formatted.contains("класс Item"));
        assert!(formatted.contains("конструктор новый(this)"));
        assert!(formatted.contains("публичный статичный функция get(this)"));

        grammar::ProgramParser::new()
            .parse(lex(&formatted))
            .expect("Russian formatted source should remain parseable");
    }
}
