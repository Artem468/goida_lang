use crate::ast::class::{ClassDefinition, ClassField, ClassMethod, FieldVisibility};
use crate::ast::expr::{ExprId, ExpressionKind};
use crate::ast::prelude::Span;
use crate::lexer::structs::Token;
use crate::parser::structs::{ParseError, Parser};

impl Parser {


    /// Парсинг определения класса
    pub(crate) fn parse_class(&mut self) -> Result<ClassDefinition, ParseError> {
        self.expect(Token::Class)?;
        let span = self.current_token().span;


        let name = match &self.current_token().token {
            Token::Identifier(name) => {
                let symbol = self.program.arena.intern_string(name);
                self.advance();
                symbol
            }
            _ => {
                return Err(ParseError::UnexpectedToken(format!(
                    "Ожидался идентификатор класса в позиции {}:{}",
                    self.current_token().span.start.line,
                    self.current_token().span.start.column,
                )))
            }
        };

        self.expect(Token::LeftBrace)?;

        let mut fields = Vec::new();
        let mut methods = Vec::new();


        while !matches!(self.current_token().token, Token::RightBrace | Token::EndFile) {
            let visibility = self.parse_visibility()?;

            match self.current_token().token {
                Token::Function | Token::Constructor => {
                    let method = self.parse_class_method(visibility)?;
                    methods.push(method);
                }
                Token::Identifier(_) => {
                    let field = self.parse_class_field(visibility)?;
                    fields.push(field);
                }
                _ => {
                    return Err(ParseError::UnexpectedToken(format!(
                        "Неожиданный токен в определении класса: {:?} в позиции {}:{}",
                        self.current_token().token,
                        self.current_token().span.start.line,
                        self.current_token().span.start.column,
                    )))
                }
            }
        }

        self.expect(Token::RightBrace)?;

        Ok(ClassDefinition {
            name,
            fields,
            methods,
            span,
        })
    }

    /// Парсинг видимости (приватный/публичный)
    fn parse_visibility(&mut self) -> Result<FieldVisibility, ParseError> {
        match self.current_token().token {
            Token::Private => {
                self.advance();
                Ok(FieldVisibility::Private)
            }
            Token::Public => {
                self.advance();
                Ok(FieldVisibility::Public)
            }
            _ => Ok(FieldVisibility::Private)
        }
    }

    /// Парсинг поля класса
    fn parse_class_field(&mut self, visibility: FieldVisibility) -> Result<ClassField, ParseError> {
        let span = self.current_token().span;

        let name = match &self.current_token().token {
            Token::Identifier(name) => {
                let symbol = self.program.arena.intern_string(name);
                self.advance();
                symbol
            }
            _ => {
                return Err(ParseError::UnexpectedToken(format!(
                    "Ожидался идентификатор поля в позиции {}:{}",
                    self.current_token().span.start.line,
                    self.current_token().span.start.column,
                )))
            }
        };


        let field_type = if matches!(self.current_token().token, Token::Colon) {
            self.advance();
            Some(self.parse_type()?)
        } else {
            None
        };


        let default_value = if matches!(self.current_token().token, Token::Assign) {
            self.advance();
            Some(self.parse_expression()?)
        } else {
            None
        };

        self.expect_semicolon()?;

        Ok(ClassField {
            name,
            field_type,
            visibility,
            default_value,
            span,
        })
    }

    /// Парсинг метода класса
    fn parse_class_method(&mut self, visibility: FieldVisibility) -> Result<ClassMethod, ParseError> {
        let span = self.current_token().span;
        let is_constructor = matches!(self.current_token().token, Token::Constructor);

        if is_constructor {
            self.expect(Token::Constructor)?;
        } else {
            self.expect(Token::Function)?;
        }

        let name = match &self.current_token().token {
            Token::Identifier(name) => {
                let symbol = self.program.arena.intern_string(name);
                self.advance();
                symbol
            }
            _ => {
                return Err(ParseError::UnexpectedToken(format!(
                    "Ожидался идентификатор метода в позиции {}:{}",
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

        Ok(ClassMethod {
            name,
            params: parameters,
            return_type,
            body,
            visibility,
            is_constructor,
            span,
        })
    }

    /// Парсинг создания объекта (новый Класс(...) или новый модуль.Класс(...))
    pub(crate) fn parse_object_creation(&mut self, span: Span) -> Result<ExprId, ParseError> {
        let mut class_name_parts = Vec::new();


        match &self.current_token().token {
            Token::Identifier(name) => {
                class_name_parts.push(name.clone());
                self.advance();
            }
            _ => {
                return Err(ParseError::UnexpectedToken(format!(
                    "Ожидался идентификатор класса после 'новый' в позиции {}:{}",
                    self.current_token().span.start.line,
                    self.current_token().span.start.column,
                )))
            }
        }


        while matches!(self.current_token().token, Token::Point) {
            self.advance();

            match &self.current_token().token {
                Token::Identifier(name) => {
                    class_name_parts.push(name.clone());
                    self.advance();
                }
                _ => {
                    return Err(ParseError::UnexpectedToken(format!(
                        "Ожидался идентификатор после '.' в позиции {}:{}",
                        self.current_token().span.start.line,
                        self.current_token().span.start.column,
                    )))
                }
            }
        }


        let full_class_name = class_name_parts.join(".");
        let class_name = self.program.arena.intern_string(&full_class_name);

        self.expect(Token::LeftParentheses)?;

        let mut args = Vec::new();
        while !matches!(self.current_token().token, Token::RightParentheses) {
            args.push(self.parse_expression()?);

            if matches!(self.current_token().token, Token::Comma) {
                self.advance();
            } else if !matches!(self.current_token().token, Token::RightParentheses) {
                return Err(ParseError::UnexpectedToken(
                    "Ожидалась запятая или закрывающая скобка".to_string(),
                ));
            }
        }

        self.expect(Token::RightParentheses)?;

        Ok(self.program.arena.add_expression(
            ExpressionKind::ObjectCreation {
                class_name,
                args,
            },
            span
        ))
    }
}