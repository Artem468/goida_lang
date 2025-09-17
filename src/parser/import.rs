use crate::ast::prelude::Import;
use crate::lexer::structs::Token;
use crate::parser::structs::{ParseError, Parser};

impl Parser {
    pub(crate) fn parse_import(&mut self) -> Result<Import, ParseError> {
        self.expect(Token::Import)?;
        let span = self.current_token().span;

        let mut files = Vec::new();

        loop {
            match &self.current_token().token {
                Token::TextLiteral(filename) => {
                    let symbol = self.program.arena.intern_string(filename);
                    files.push(symbol);
                    self.advance();
                }
                _ => {
                    return Err(ParseError::UnexpectedToken(format!(
                        "Ожидалось имя файла в кавычках в позиции {}:{}",
                        self.current_token().span.start.line,
                        self.current_token().span.start.column,
                    )))
                }
            }

            if matches!(self.current_token().token, Token::Comma) {
                self.advance();
            } else {
                break;
            }
        }

        self.expect_semicolon()?;

        Ok(Import { files, span })
    }
}