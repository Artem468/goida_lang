use crate::ast::prelude::Program;
use crate::lexer::structs::{Token, TokenInfo};
use crate::parser::structs::{ParseError, Parser};

impl Parser {
    pub fn new(tokens: Vec<TokenInfo>, name: String) -> Self {
        Self { program: Program::new(name), tokens, current: 0 }
    }

    pub fn parse(mut self) -> Result<Program, ParseError> {
        while !matches!(self.current_token().token, Token::EndFile) {
            match self.current_token().token {
                Token::Function => {
                    let func = self.parse_function()?;
                    self.program.functions.push(func);
                }
                Token::Import => {
                    let imp = self.parse_import()?;
                    self.program.imports.push(imp);
                }
                Token::Class => {
                    let class = self.parse_class()?;
                    self.program.classes.push(class);
                }
                _ => {
                    let stmt = self.parse_statement()?;
                    self.program.statements.push(stmt);
                }
            }
        }
        Ok(self.program)
    }
}