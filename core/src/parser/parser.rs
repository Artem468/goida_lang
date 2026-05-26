use crate::ast::prelude::*;
use crate::interpreter::prelude::{Module, SharedInterner};
use crate::parser::grammar;
use crate::parser::lexer::{lex, LexicalError, Token};
use crate::parser::prelude::{ParseError, Parser as ParserTrait};
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
        Ok(format!("{syntax:#?}"))
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
    fn macro_expansion_preview_contains_expanded_program_without_macro_definition() {
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

        assert!(preview.contains("Binary"));
        assert!(!preview.contains("MacroDefinition"));
    }
}
