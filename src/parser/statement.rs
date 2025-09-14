use crate::ast::expr::ExpressionKind;
use crate::ast::prelude::{StatementKind, StmtId};
use crate::lexer::structs::Token;
use crate::parser::structs::{ParseError, Parser};

impl Parser {
    pub(crate) fn parse_block(&mut self) -> Result<StmtId, ParseError> {
        self.expect(Token::LeftBrace)?;
        let statements = self.parse_block_statements()?;
        self.expect(Token::RightBrace)?;

        let span = self.current_token().span;
        Ok(self
            .program
            .arena
            .add_statement(StatementKind::Block(statements), span))
    }

    fn parse_block_statements(&mut self) -> Result<Vec<StmtId>, ParseError> {
        let mut statements = Vec::new();
        while !matches!(
            self.current_token().token,
            Token::RightBrace | Token::EndFile
        ) {
            statements.push(self.parse_statement()?);
        }
        Ok(statements)
    }
    pub(crate) fn parse_statement(&mut self) -> Result<StmtId, ParseError> {
        let span = self.current_token().span;

        match self.current_token().token {
            Token::Let => self.parse_declaration(),
            Token::If => self.parse_if_statement(),
            Token::While => self.parse_while_statement(),
            Token::For => self.parse_for_statement(),
            Token::Return => self.parse_return_statement(),
            Token::Print => self.parse_print_statement(),
            Token::Input => self.parse_input_statement(),
            Token::LeftBrace => self.parse_block(),
            Token::Identifier(_) | Token::This => {
                let expr = self.parse_expression()?;

                if matches!(self.current_token().token, Token::Assign) {
                    self.advance();
                    let value = self.parse_expression()?;
                    self.expect_semicolon()?;

                    if let Some(expr_node) = self.program.arena.get_expression(expr) {
                        match &expr_node.kind {
                            ExpressionKind::Identifier(name) => {
                                return Ok(self.program.arena.add_statement(
                                    StatementKind::Assign { name: *name, value },
                                    span,
                                ));
                            }
                            ExpressionKind::Index { object, index } => {
                                return Ok(self.program.arena.add_statement(
                                    StatementKind::IndexAssign {
                                        object: *object,
                                        index: *index,
                                        value,
                                    },
                                    span,
                                ));
                            }
                            ExpressionKind::PropertyAccess { object, property } => {
                                return Ok(self.program.arena.add_statement(
                                    StatementKind::PropertyAssign {
                                        object: *object,
                                        property: *property,
                                        value,
                                    },
                                    span,
                                ));
                            }
                            _ => {
                                return Err(ParseError::UnexpectedToken(
                                    "Недопустимое выражение для назначения".to_string(),
                                ));
                            }
                        }
                    } else {
                        return Err(ParseError::InternalError(
                            "Не удалось получить выражение для назначения".to_string(),
                        ));
                    }
                }

                self.expect_semicolon()?;
                Ok(self
                    .program
                    .arena
                    .add_statement(StatementKind::Expression(expr), span))
            }
            _ => {
                let expr = self.parse_expression()?;
                self.expect_semicolon()?;
                Ok(self
                    .program
                    .arena
                    .add_statement(StatementKind::Expression(expr), span))
            }
        }
    }

    fn parse_declaration(&mut self) -> Result<StmtId, ParseError> {
        self.expect(Token::Let)?;
        let span = self.current_token().span;

        let name = match &self.current_token().token {
            Token::Identifier(name) => {
                let symbol = self.program.arena.intern_string(name);
                self.advance();
                symbol
            }
            _ => {
                return Err(ParseError::UnexpectedToken(format!(
                    "Ожидался идентификатор переменной в позиции {}:{}",
                    self.current_token().span.start.line,
                    self.current_token().span.start.column,
                )))
            }
        };

        let type_hint = if matches!(self.current_token().token, Token::Colon) {
            self.advance();
            Some(self.parse_type()?)
        } else {
            None
        };

        let value = if matches!(self.current_token().token, Token::Assign) {
            self.advance();
            Some(self.parse_expression()?)
        } else {
            None
        };

        self.expect_semicolon()?;

        Ok(self.program.arena.add_statement(
            StatementKind::Let {
                name,
                type_hint,
                value,
            },
            span,
        ))
    }

    fn parse_if_statement(&mut self) -> Result<StmtId, ParseError> {
        self.expect(Token::If)?;
        let span = self.current_token().span;

        self.expect(Token::LeftParentheses)?;
        let condition = self.parse_expression()?;
        self.expect(Token::RightParentheses)?;

        let then_body = self.parse_block()?;

        let else_body = if matches!(self.current_token().token, Token::Else) {
            self.advance();
            Some(self.parse_block()?)
        } else {
            None
        };

        Ok(self.program.arena.add_statement(
            StatementKind::If {
                condition,
                then_body,
                else_body,
            },
            span,
        ))
    }

    fn parse_while_statement(&mut self) -> Result<StmtId, ParseError> {
        self.expect(Token::While)?;
        let span = self.current_token().span;

        self.expect(Token::LeftParentheses)?;
        let condition = self.parse_expression()?;
        self.expect(Token::RightParentheses)?;

        let body = self.parse_block()?;

        Ok(self
            .program
            .arena
            .add_statement(StatementKind::While { condition, body }, span))
    }

    fn parse_for_statement(&mut self) -> Result<StmtId, ParseError> {
        self.expect(Token::For)?;
        let span = self.current_token().span;

        self.expect(Token::LeftParentheses)?;

        let variable = match &self.current_token().token {
            Token::Identifier(name) => {
                let symbol = self.program.arena.intern_string(name);
                self.advance();
                symbol
            }
            _ => {
                return Err(ParseError::UnexpectedToken(format!(
                    "Ожидался идентификатор переменной цикла в позиции {}:{}",
                    self.current_token().span.start.line,
                    self.current_token().span.start.column,
                )))
            }
        };

        self.expect(Token::Assign)?;
        let start = self.parse_expression()?;
        self.expect(Token::SemicolonPoint)?;
        let end = self.parse_expression()?;
        self.expect(Token::RightParentheses)?;

        let body = self.parse_block()?;

        Ok(self.program.arena.add_statement(
            StatementKind::For {
                variable,
                start,
                end,
                body,
            },
            span,
        ))
    }

    fn parse_return_statement(&mut self) -> Result<StmtId, ParseError> {
        self.expect(Token::Return)?;
        let span = self.current_token().span;

        let value = if matches!(self.current_token().token, Token::SemicolonPoint) {
            None
        } else {
            Some(self.parse_expression()?)
        };

        self.expect_semicolon()?;

        Ok(self
            .program
            .arena
            .add_statement(StatementKind::Return(value), span))
    }

    fn parse_print_statement(&mut self) -> Result<StmtId, ParseError> {
        self.expect(Token::Print)?;
        let span = self.current_token().span;

        self.expect(Token::LeftParentheses)?;
        let expression = self.parse_expression()?;
        self.expect(Token::RightParentheses)?;
        self.expect_semicolon()?;

        Ok(self
            .program
            .arena
            .add_statement(StatementKind::Print(expression), span))
    }

    fn parse_input_statement(&mut self) -> Result<StmtId, ParseError> {
        self.expect(Token::Input)?;
        let span = self.current_token().span;

        self.expect(Token::LeftParentheses)?;
        let expression = self.parse_expression()?;
        self.expect(Token::RightParentheses)?;
        self.expect_semicolon()?;

        Ok(self
            .program
            .arena
            .add_statement(StatementKind::Input(expression), span))
    }
}
