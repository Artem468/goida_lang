use crate::ast::prelude::*;
use crate::interpreter::prelude::{Module, SharedInterner};
use crate::parser::prelude::{
    extract_last_token, translate_rule, ParseError, Parser as ParserTrait,
};
use pest::error::ErrorVariant;
use pest::Parser;
use pest_derive::Parser;
use std::path::PathBuf;

#[derive(Parser)]
#[grammar = "grammar.pest"]
struct ProgramParser;

impl ParserTrait {
    pub fn new(interner: SharedInterner, name: &str, path: PathBuf) -> Self {
        Self {
            module: Module::new(&interner, name, path),
            interner,
            nesting_level: 0,
        }
    }

    fn get_char_range(&self, s: &str, start: usize, end: usize) -> String {
        let mut indices = s
            .char_indices()
            .map(|(idx, _)| idx)
            .chain(std::iter::once(s.len()));
        let byte_start = indices.nth(start).unwrap_or(s.len());
        let byte_end = indices
            .nth(end.saturating_sub(start + 1))
            .unwrap_or(s.len());
        s[byte_start..byte_end].to_string()
    }

    pub fn parse(mut self, code: &str) -> Result<Module, ParseError> {
        let pairs = ProgramParser::parse(Rule::program, code).map_err(|e| {
            let (start, end) = match e.location {
                pest::error::InputLocation::Pos(pos) => extract_last_token(code, pos),
                pest::error::InputLocation::Span((start, end)) => (start, end),
            };

            let message = match &e.variant {
                ErrorVariant::ParsingError {
                    positives,
                    negatives,
                } => {
                    let mut parts = Vec::new();

                    if !positives.is_empty() {
                        let found = self.get_char_range(code, start, end);

                        let found = if found.is_empty() {
                            code.chars()
                                .nth(start)
                                .map(|c| c.to_string())
                                .unwrap_or_else(|| "конец файла".into())
                        } else {
                            found.to_string()
                        };

                        let expected: Vec<String> =
                            positives.iter().map(|r| translate_rule(r)).collect();
                        parts.push(format!(
                            "ожидалось: \n{} получено {}",
                            expected.join("\n"),
                            found
                        ));
                    }

                    if !negatives.is_empty() {
                        let unexpected: Vec<String> =
                            negatives.iter().map(|r| translate_rule(r)).collect();
                        parts.push(format!("неожиданный токен: \n{}", unexpected.join("\n")));
                    }

                    if parts.is_empty() {
                        "неизвестная ошибка синтаксиса".to_string()
                    } else {
                        parts.join("; ")
                    }
                }
                ErrorVariant::CustomError { message } => message.clone(),
            };

            ParseError::InvalidSyntax(ErrorData::new(
                Span::new(start, end, self.module.name),
                message,
            ))
        })?;
        self.module.arena.init_builtin_types(&self.interner);
        self.init_builtin_error_classes();
        for pair in pairs {
            if pair.as_rule() == Rule::program {
                for inner in pair.into_inner() {
                    match inner.as_rule() {
                        Rule::function => {
                            let stmt_id = self.parse_function(inner)?;
                            self.module.body.push(stmt_id);
                        }
                        Rule::class => {
                            let stmt_id = self.parse_class(inner)?;
                            self.module.body.push(stmt_id);
                        }
                        Rule::library_stmt => {
                            let stmt_id = self.parse_library_stmt(inner)?;
                            self.module.body.push(stmt_id);
                        }
                        Rule::assignment => {
                            let stmt_id = self.parse_assignment(inner)?;
                            self.module.body.push(stmt_id);
                        }
                        Rule::property_assign => {
                            let stmt_id = self.parse_property_assign(inner)?;
                            self.module.body.push(stmt_id);
                        }
                        Rule::if_stmt => {
                            let stmt_id = self.parse_if_stmt(inner)?;
                            self.module.body.push(stmt_id);
                        }
                        Rule::try_stmt => {
                            let stmt_id = self.parse_try_stmt(inner)?;
                            self.module.body.push(stmt_id);
                        }
                        Rule::raise_stmt => {
                            let stmt_id = self.parse_raise_stmt(inner)?;
                            self.module.body.push(stmt_id);
                        }
                        Rule::while_stmt => {
                            let stmt_id = self.parse_while_stmt(inner)?;
                            self.module.body.push(stmt_id);
                        }
                        Rule::for_stmt => {
                            let stmt_id = self.parse_for_stmt(inner)?;
                            self.module.body.push(stmt_id);
                        }
                        Rule::thread_stmt => {
                            let stmt_id = self.parse_thread_stmt(inner)?;
                            self.module.body.push(stmt_id);
                        }
                        Rule::return_stmt => {
                            let stmt_id = self.parse_return_stmt(inner)?;
                            self.module.body.push(stmt_id);
                        }
                        Rule::expr_stmt => {
                            let stmt_id = self.parse_expr_stmt(inner)?;
                            self.module.body.push(stmt_id);
                        }
                        Rule::import_stmt => {
                            let stmt_id = self.parse_import_stmt(inner)?;
                            self.module.body.push(stmt_id);
                        }
                        _ => {}
                    }
                }
            }
        }
        self.validate_module_names()?;
        self.module.arena.optimize_all(&self.interner);
        Ok(self.module)
    }
}
