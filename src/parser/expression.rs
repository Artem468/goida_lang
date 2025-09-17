use crate::ast::expr::{ExprId, ExpressionKind, LiteralValue};
use crate::ast::prelude::{BinaryOperator, UnaryOperator};
use crate::lexer::structs::Token;
use crate::parser::structs::{ParseError, Parser};

impl Parser {
    pub(crate) fn parse_expression(&mut self) -> Result<ExprId, ParseError> {
        self.parse_logical_or()
    }
    fn parse_logical_or(&mut self) -> Result<ExprId, ParseError> {
        let mut expr = self.parse_logical_and()?;

        while matches!(self.current_token().token, Token::Or) {
            let span = self.current_token().span;
            self.advance();
            let right = self.parse_logical_and()?;
            expr = self.program.arena.add_expression(
                ExpressionKind::Binary {
                    op: BinaryOperator::Or,
                    left: expr,
                    right,
                },
                span,
            );
        }

        Ok(expr)
    }

    fn parse_logical_and(&mut self) -> Result<ExprId, ParseError> {
        let mut expr = self.parse_equality()?;

        while matches!(self.current_token().token, Token::And) {
            let span = self.current_token().span;
            self.advance();
            let right = self.parse_equality()?;
            expr = self.program.arena.add_expression(
                ExpressionKind::Binary {
                    op: BinaryOperator::And,
                    left: expr,
                    right,
                },
                span,
            );
        }

        Ok(expr)
    }

    fn parse_equality(&mut self) -> Result<ExprId, ParseError> {
        let mut expr = self.parse_comparison()?;

        while matches!(self.current_token().token, Token::Equal | Token::Unequal) {
            let span = self.current_token().span;
            let op = match self.current_token().token {
                Token::Equal => BinaryOperator::Eq,
                Token::Unequal => BinaryOperator::Ne,
                _ => unreachable!(),
            };
            self.advance();
            let right = self.parse_comparison()?;
            expr = self.program.arena.add_expression(
                ExpressionKind::Binary {
                    op,
                    left: expr,
                    right,
                },
                span,
            );
        }

        Ok(expr)
    }

    fn parse_comparison(&mut self) -> Result<ExprId, ParseError> {
        let mut expr = self.parse_term()?;

        while matches!(
            self.current_token().token,
            Token::More | Token::Less | Token::MoreEqual | Token::LessEqual
        ) {
            let span = self.current_token().span;
            let op = match self.current_token().token {
                Token::More => BinaryOperator::Gt,
                Token::Less => BinaryOperator::Lt,
                Token::MoreEqual => BinaryOperator::Ge,
                Token::LessEqual => BinaryOperator::Le,
                _ => unreachable!(),
            };
            self.advance();
            let right = self.parse_term()?;
            expr = self.program.arena.add_expression(
                ExpressionKind::Binary {
                    op,
                    left: expr,
                    right,
                },
                span,
            );
        }

        Ok(expr)
    }

    fn parse_term(&mut self) -> Result<ExprId, ParseError> {
        let mut expr = self.parse_factor()?;

        while matches!(self.current_token().token, Token::Plus | Token::Minus) {
            let span = self.current_token().span;
            let op = match self.current_token().token {
                Token::Plus => BinaryOperator::Add,
                Token::Minus => BinaryOperator::Sub,
                _ => unreachable!(),
            };
            self.advance();
            let right = self.parse_factor()?;
            expr = self.program.arena.add_expression(
                ExpressionKind::Binary {
                    op,
                    left: expr,
                    right,
                },
                span,
            );
        }

        Ok(expr)
    }

    fn parse_factor(&mut self) -> Result<ExprId, ParseError> {
        let mut expr = self.parse_unary()?;

        while matches!(
            self.current_token().token,
            Token::Multiply | Token::Divide | Token::Remainder
        ) {
            let span = self.current_token().span;
            let op = match self.current_token().token {
                Token::Multiply => BinaryOperator::Mul,
                Token::Divide => BinaryOperator::Div,
                Token::Remainder => BinaryOperator::Mod,
                _ => unreachable!(),
            };
            self.advance();
            let right = self.parse_unary()?;
            expr = self.program.arena.add_expression(
                ExpressionKind::Binary {
                    op,
                    left: expr,
                    right,
                },
                span,
            );
        }

        Ok(expr)
    }

    fn parse_unary(&mut self) -> Result<ExprId, ParseError> {
        match self.current_token().token {
            Token::Minus => {
                let span = self.current_token().span;
                self.advance();
                let expr = self.parse_unary()?;
                Ok(self.program.arena.add_expression(
                    ExpressionKind::Unary {
                        op: UnaryOperator::Negative,
                        operand: expr,
                    },
                    span,
                ))
            }
            Token::Not => {
                let span = self.current_token().span;
                self.advance();
                let expr = self.parse_unary()?;
                Ok(self.program.arena.add_expression(
                    ExpressionKind::Unary {
                        op: UnaryOperator::Not,
                        operand: expr,
                    },
                    span,
                ))
            }
            _ => self.parse_postfix(),
        }
    }

    fn parse_postfix(&mut self) -> Result<ExprId, ParseError> {
        let mut expr = self.parse_primary()?;

        loop {
            match self.current_token().token {
                Token::Point => {
                    self.advance();
                    let member_token = self.current_token();
                    if let Token::Identifier(member_name) = member_token.token {
                        let property_symbol = self.program.arena.intern_string(&member_name);
                        self.advance();

                        if matches!(self.current_token().token, Token::LeftParentheses) {
                            self.advance();
                            let mut args = Vec::new();

                            while !matches!(self.current_token().token, Token::RightParentheses) {
                                args.push(self.parse_expression()?);
                                if matches!(self.current_token().token, Token::Comma) {
                                    self.advance();
                                } else if !matches!(
                                    self.current_token().token,
                                    Token::RightParentheses
                                ) {
                                    return Err(ParseError::UnexpectedToken(
                                        "Ожидалась запятая или закрывающая скобка".to_string(),
                                    ));
                                }
                            }

                            self.expect(Token::RightParentheses)?;

                            expr = self.program.arena.add_expression(
                                ExpressionKind::MethodCall {
                                    object: expr,
                                    method: property_symbol,
                                    args,
                                },
                                member_token.span,
                            );
                        } else {
                            expr = self.program.arena.add_expression(
                                ExpressionKind::PropertyAccess {
                                    object: expr,
                                    property: property_symbol,
                                },
                                member_token.span,
                            );
                        }
                    } else {
                        return Err(ParseError::UnexpectedToken(
                            "Ожидался идентификатор после точки".to_string(),
                        ));
                    }
                }
                Token::LeftBracket => {
                    let span = self.current_token().span;
                    self.advance();
                    let index = self.parse_expression()?;
                    self.expect(Token::RightBracket)?;
                    expr = self.program.arena.add_expression(
                        ExpressionKind::Index {
                            object: expr,
                            index,
                        },
                        span,
                    );
                }
                Token::LeftParentheses => {
                    let span = self.current_token().span;
                    self.advance();
                    let mut arguments = Vec::new();

                    while !matches!(self.current_token().token, Token::RightParentheses) {
                        arguments.push(self.parse_expression()?);
                        if matches!(self.current_token().token, Token::Comma) {
                            self.advance();
                        }
                    }

                    self.expect(Token::RightParentheses)?;

                    if let Some(expr_node) = self.program.arena.get_expression(expr) {
                        if let ExpressionKind::Identifier(_name) = expr_node.kind {
                            expr = self.program.arena.add_expression(
                                ExpressionKind::Call {
                                    function: expr,
                                    args: arguments,
                                },
                                span,
                            );
                        } else {
                            return Err(ParseError::UnexpectedToken(
                                "Вызов функции возможен только для идентификаторов".to_string(),
                            ));
                        }
                    } else {
                        return Err(ParseError::UnexpectedToken(
                            "Не удалось получить выражение для вызова функции".to_string(),
                        ));
                    }
                }
                _ => break,
            }
        }

        Ok(expr)
    }

    fn parse_primary(&mut self) -> Result<ExprId, ParseError> {
        let span = self.current_token().span;

        match self.current_token().token.clone() {
            Token::NumericalLiteral(n) => {
                self.advance();
                Ok(self
                    .program
                    .arena
                    .add_expression(ExpressionKind::Literal(LiteralValue::Number(n)), span))
            }
            Token::FloatLiteral(f) => {
                self.advance();
                Ok(self
                    .program
                    .arena
                    .add_expression(ExpressionKind::Literal(LiteralValue::Float(f)), span))
            }
            Token::TextLiteral(s) => {
                self.advance();
                let symbol = self.program.arena.intern_string(&s);
                Ok(self
                    .program
                    .arena
                    .add_expression(ExpressionKind::Literal(LiteralValue::Text(symbol)), span))
            }
            Token::True => {
                self.advance();
                Ok(self
                    .program
                    .arena
                    .add_expression(ExpressionKind::Literal(LiteralValue::Boolean(true)), span))
            }
            Token::False => {
                self.advance();
                Ok(self
                    .program
                    .arena
                    .add_expression(ExpressionKind::Literal(LiteralValue::Boolean(false)), span))
            }
            Token::Identifier(name) => {
                let symbol = self.program.arena.intern_string(&name);
                self.advance();
                Ok(self
                    .program
                    .arena
                    .add_expression(ExpressionKind::Identifier(symbol), span))
            }
            Token::New => {
                self.advance();
                self.parse_object_creation(span)
            }
            Token::This => {
                self.advance();
                Ok(self
                    .program
                    .arena
                    .add_expression(ExpressionKind::This, span))
            }
            Token::Input => {
                self.advance();
                self.expect(Token::LeftParentheses)?;
                let prompt = self.parse_expression()?;
                self.expect(Token::RightParentheses)?;
                Ok(self
                    .program
                    .arena
                    .add_expression(ExpressionKind::Input(prompt), span))
            }
            Token::LeftParentheses => {
                self.advance();
                let expr = self.parse_expression()?;
                self.expect(Token::RightParentheses)?;
                Ok(expr)
            }
            _ => Err(ParseError::UnexpectedToken(format!(
                "Неожиданный токен: {:?} в позиции {}:{}",
                self.current_token().token,
                self.current_token().span.start.line,
                self.current_token().span.start.column,
            ))),
        }
    }
}
