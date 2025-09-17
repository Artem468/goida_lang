use crate::ast::prelude::{Function, Parameter};
use crate::lexer::structs::Token;
use crate::parser::structs::{ParseError, Parser};

impl Parser {
    pub(crate) fn parse_parameter(&mut self) -> Result<Parameter, ParseError> {
        let name = match &self.current_token().token {
            Token::Identifier(name) => {
                let symbol = self.program.arena.intern_string(name);
                let span = self.current_token().span;
                self.advance();
                (symbol, span)
            }
            _ => {
                return Err(ParseError::UnexpectedToken(format!(
                    "Ожидался идентификатор параметра в позиции {}:{}",
                    self.current_token().span.start.line,
                    self.current_token().span.start.column,
                )))
            }
        };

        self.expect(Token::Colon)?;
        let param_type = self.parse_type()?;

        Ok(Parameter {
            name: name.0,
            param_type,
            span: name.1,
        })
    }

    pub(crate) fn parse_function(&mut self) -> Result<Function, ParseError> {
        self.expect(Token::Function)?;
        let span = self.current_token().span;

        let name = match &self.current_token().token {
            Token::Identifier(name) => {
                let symbol = self.program.arena.intern_string(name);
                self.advance();
                symbol
            }
            _ => {
                return Err(ParseError::UnexpectedToken(format!(
                    "Ожидался идентификатор функции в позиции {}:{}",
                    self.current_token().span.start.line,
                    self.current_token().span.start.column,
                )))
            }
        };

        self.expect(Token::LeftParentheses)?;

        let mut parameters = Vec::new();
        while !matches!(self.current_token().token, Token::RightParentheses) {
            let param = self.parse_parameter()?;
            parameters.push(param);

            if matches!(self.current_token().token, Token::Comma) {
                self.advance();
            } else if !matches!(self.current_token().token, Token::RightParentheses) {
                return Err(ParseError::UnexpectedToken(
                    "Ожидалась запятая или закрывающая скобка".to_string(),
                ));
            }
        }

        self.expect(Token::RightParentheses)?;

        let return_type = if matches!(self.current_token().token, Token::Colon) {
            self.advance();
            Some(self.parse_type()?)
        } else {
            None
        };

        let body = self.parse_block()?;

        Ok(Function {
            name,
            params: parameters,
            return_type,
            body,
            span,
            module: Some(self.program.name),
        })
    }
}