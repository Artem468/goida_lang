use crate::ast::prelude::{DataType, PrimitiveType, Span, TypeId};
use crate::lexer::structs::{Token, TokenInfo};
use crate::parser::structs::{ParseError, Parser};

impl Parser {
    pub(crate) fn current_token(&self) -> TokenInfo {
        self.tokens.get(self.current).cloned().unwrap_or(TokenInfo {
            token: Token::EndFile,
            span: Span::default(),
        })
    }

    pub fn advance(&mut self) {
        if self.current < self.tokens.len() {
            self.current += 1;
        }
    }

    pub fn expect(&mut self, expected: Token) -> Result<(), ParseError> {
        if std::mem::discriminant(&self.current_token().token) == std::mem::discriminant(&expected)
        {
            self.advance();
            Ok(())
        } else {
            Err(ParseError::UnexpectedToken(format!(
                "Ожидался {:?}, получен {:?} в позиции {}:{}",
                expected,
                self.current_token().token,
                self.current_token().span.start.line,
                self.current_token().span.start.column,
            )))
        }
    }

    pub(crate) fn parse_type(&mut self) -> Result<TypeId, ParseError> {
        let data_type = match self.current_token().token {
            Token::Number => {
                self.advance();
                DataType::Primitive(PrimitiveType::Number)
            }
            Token::Float => {
                self.advance();
                DataType::Primitive(PrimitiveType::Float)
            }
            Token::Text => {
                self.advance();
                DataType::Primitive(PrimitiveType::Text)
            }
            Token::Boolean => {
                self.advance();
                DataType::Primitive(PrimitiveType::Boolean)
            }
            Token::List => {
                self.advance();
                self.expect(Token::LeftBracket)?;
                let element_type_id = self.parse_type()?;
                let element_type = self
                    .program
                    .arena
                    .get_type(element_type_id)
                    .ok_or_else(|| {
                        ParseError::InternalError("Не удалось получить тип элемента".to_string())
                    })?
                    .clone();
                self.expect(Token::RightBracket)?;
                DataType::List(Box::new(element_type))
            }
            Token::Dict => {
                self.advance();
                self.expect(Token::LeftBracket)?;
                let key_type_id = self.parse_type()?;
                let key_type = self
                    .program
                    .arena
                    .get_type(key_type_id)
                    .ok_or_else(|| {
                        ParseError::InternalError("Не удалось получить тип ключа".to_string())
                    })?
                    .clone();
                self.expect(Token::Comma)?;
                let value_type_id = self.parse_type()?;
                let value_type = self
                    .program
                    .arena
                    .get_type(value_type_id)
                    .ok_or_else(|| {
                        ParseError::InternalError("Не удалось получить тип значения".to_string())
                    })?
                    .clone();
                self.expect(Token::RightBracket)?;
                DataType::Dict {
                    key: Box::new(key_type),
                    value: Box::new(value_type),
                }
            }
            Token::Identifier(type_name) => {
                let symbol = self.program.arena.intern_string(&type_name);
                self.advance();
                DataType::Object(symbol)
            }
            _ => {
                return Err(ParseError::UnexpectedToken(format!(
                    "Ожидался тип данных в позиции {}:{}",
                    self.current_token().span.start.line,
                    self.current_token().span.start.column,
                )))
            }
        };

        Ok(self.program.arena.add_type(data_type))
    }

    pub(crate) fn expect_semicolon(&mut self) -> Result<(), ParseError> {
        if matches!(self.current_token().token, Token::SemicolonPoint) {
            self.advance();
            Ok(())
        } else {
            Err(ParseError::UnexpectedToken(format!(
                "Ожидалась точка с запятой, но найден: {:?} в позиции {}:{}",
                self.current_token().token,
                self.current_token().span.start.line,
                self.current_token().span.start.column,
            )))
        }
    }
}
