use crate::ast::*;
use crate::lexer::structs::{Span, Token, TokenInfo};
use crate::parser::structs::{ParseError, Parser};

impl Parser {
    pub fn new(tokens: Vec<TokenInfo>) -> Self {
        Parser { tokens, current: 0 }
    }

    fn current_token(&self) -> &TokenInfo {
        self.tokens.get(self.current).unwrap_or(&TokenInfo {
            token: Token::EndFile,
            span: Span { column: 0, line: 0 },
        })
    }

    fn peek_token(&self) -> &TokenInfo {
        self.tokens.get(self.current + 1).unwrap_or(&TokenInfo {
            token: Token::EndFile,
            span: Span { column: 0, line: 0 },
        })
    }

    fn advance(&mut self) -> &TokenInfo {
        if self.current < self.tokens.len() {
            self.current += 1;
        }
        self.current_token()
    }

    fn expect(&mut self, expected: Token) -> Result<(), ParseError> {
        if std::mem::discriminant(&self.current_token().token) == std::mem::discriminant(&expected)
        {
            self.advance();
            Ok(())
        } else {
            Err(ParseError::UnexpectedToken(format!(
                "Ожидался {:?}, получен {:?} (строка: {}, столбец: {})",
                expected,
                self.current_token().token,
                self.current_token().span.line,
                self.current_token().span.column,
            )))
        }
    }

    pub fn parse(&mut self) -> Result<Program, ParseError> {
        let mut functions = Vec::new();
        let mut operators = Vec::new();
        let mut imports = Vec::new();

        while !matches!(self.current_token().token, Token::EndFile) {
            match self.current_token().token {
                Token::Function => {
                    functions.push(self.parse_function()?);
                }
                Token::Import => imports.push(self.parse_import()?),
                _ => {
                    operators.push(self.parse_statement()?);
                }
            }
        }

        Ok(Program {
            functions,
            operators,
            imports,
        })
    }

    fn parse_function(&mut self) -> Result<Function, ParseError> {
        self.expect(Token::Function)?;

        let name = match &self.current_token().token {
            Token::Identifier(_name) => {
                let _name = _name.clone();
                self.advance();
                _name
            }
            _ => {
                return Err(ParseError::UnexpectedToken(format!(
                    "Ожидался идентификатор функции (строка: {}, столбец: {})",
                    self.current_token().span.line,
                    self.current_token().span.column,
                )))
            }
        };

        self.expect(Token::LeftParentheses)?;

        let mut parameters = Vec::new();
        while !matches!(self.current_token().token, Token::RightParentheses) {
            let param_name = match &self.current_token().token {
                Token::Identifier(name) => {
                    let name = name.clone();
                    self.advance();
                    name
                }
                _ => {
                    return Err(ParseError::UnexpectedToken(format!(
                        "Ожидался идентификатор параметра (строка: {}, столбец: {})",
                        self.current_token().span.line,
                        self.current_token().span.column,
                    )))
                }
            };

            self.expect(Token::Colon)?;

            let param_type = self.parse_type()?;
            parameters.push(Parameter {
                name: param_name,
                type_of: param_type,
            });

            if matches!(self.current_token().token, Token::Comma) {
                self.advance();
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
            parameters,
            return_type,
            body,
            module: None,
        })
    }

    fn parse_import(&mut self) -> Result<Import, ParseError> {
        self.expect(Token::Import)?;

        let mut files = Vec::new();

        loop {
            match &self.current_token().token {
                Token::TextLiteral(filename) => {
                    files.push(filename.clone());
                    self.advance();
                }
                _ => {
                    return Err(ParseError::UnexpectedToken(format!(
                        "Ожидалось имя файла в кавычках (строка: {}, столбец: {})",
                        self.current_token().span.line,
                        self.current_token().span.column,
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

        Ok(Import { files })
    }
    fn parse_type(&mut self) -> Result<DataType, ParseError> {
        match self.current_token().token {
            Token::Number => {
                self.advance();
                Ok(DataType::Number)
            }
            Token::Text => {
                self.advance();
                Ok(DataType::Text)
            }
            Token::Boolean => {
                self.advance();
                Ok(DataType::Boolean)
            }
            _ => Err(ParseError::UnexpectedToken(format!(
                "Ожидался тип данных (строка: {}, столбец: {})",
                self.current_token().span.line,
                self.current_token().span.column,
            ))),
        }
    }

    fn parse_block(&mut self) -> Result<Vec<Statement>, ParseError> {
        self.expect(Token::LeftBrace)?;

        let mut statements = Vec::new();
        while !matches!(self.current_token().token, Token::RightBrace) {
            statements.push(self.parse_statement()?);
        }

        self.expect(Token::RightBrace)?;
        Ok(statements)
    }

    fn parse_statement(&mut self) -> Result<Statement, ParseError> {
        match self.current_token().token {
            Token::Let => self.parse_declaration(),
            Token::If => self.parse_if_statement(),
            Token::While => self.parse_while_statement(),
            Token::For => self.parse_for_statement(),
            Token::Return => self.parse_return_statement(),
            Token::Print => self.parse_print_statement(),
            Token::Input => self.parse_input_statement(),
            Token::LeftBrace => {
                let block = self.parse_block()?;
                Ok(Statement::Block(block))
            }
            Token::Identifier(_) => {
                if matches!(self.peek_token().token, Token::Assign) {
                    self.parse_assignment()
                } else {
                    let expr = self.parse_expression()?;
                    self.expect_semicolon()?;
                    Ok(Statement::Expression(expr))
                }
            }
            _ => {
                let expr = self.parse_expression()?;
                self.expect_semicolon()?;
                Ok(Statement::Expression(expr))
            }
        }
    }

    fn parse_declaration(&mut self) -> Result<Statement, ParseError> {
        self.expect(Token::Let)?;

        let name = match &self.current_token().token {
            Token::Identifier(_name) => {
                let _name = _name.clone();
                self.advance();
                _name
            }
            _ => {
                return Err(ParseError::UnexpectedToken(format!(
                    "Ожидался идентификатор переменной, но получен токен: {:?} (строка: {}, столбец: {})",
                    self.current_token().token,
                    self.current_token().span.line,
                    self.current_token().span.column,
                )))
            }
        };

        let type_of = if matches!(self.current_token().token, Token::Colon) {
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

        Ok(Statement::Declaration {
            name,
            type_of,
            value,
        })
    }
    fn parse_assignment(&mut self) -> Result<Statement, ParseError> {
        let name = match &self.current_token().token {
            Token::Identifier(name) => {
                let name = name.clone();
                self.advance();
                name
            }
            _ => {
                return Err(ParseError::UnexpectedToken(format!(
                    "Ожидался идентификатор переменной (строка: {}, столбец: {})",
                    self.current_token().span.line,
                    self.current_token().span.column,
                )))
            }
        };

        self.expect(Token::Assign)?;
        let value = self.parse_expression()?;
        self.expect_semicolon()?;

        Ok(Statement::Assignment { name, value })
    }

    fn parse_if_statement(&mut self) -> Result<Statement, ParseError> {
        self.expect(Token::If)?;
        self.expect(Token::LeftParentheses)?;
        let condition = self.parse_expression()?;
        self.expect(Token::RightParentheses)?;

        let body = self.parse_block()?;

        let another = if matches!(self.current_token().token, Token::Else) {
            self.advance();
            Some(self.parse_block()?)
        } else {
            None
        };

        Ok(Statement::If {
            condition,
            body,
            another,
        })
    }

    fn parse_while_statement(&mut self) -> Result<Statement, ParseError> {
        self.expect(Token::While)?;
        self.expect(Token::LeftParentheses)?;
        let condition = self.parse_expression()?;
        self.expect(Token::RightParentheses)?;

        let body = self.parse_block()?;

        Ok(Statement::While { condition, body })
    }

    fn parse_for_statement(&mut self) -> Result<Statement, ParseError> {
        self.expect(Token::For)?;
        self.expect(Token::LeftParentheses)?;

        let variable = match &self.current_token().token {
            Token::Identifier(name) => {
                let name = name.clone();
                self.advance();
                name
            }
            _ => {
                return Err(ParseError::UnexpectedToken(format!(
                    "Ожидался идентификатор переменной цикла (строка: {}, столбец: {})",
                    self.current_token().span.line,
                    self.current_token().span.column,
                )))
            }
        };

        self.expect(Token::Assign)?;
        let start = self.parse_expression()?;
        self.expect(Token::SemicolonPoint)?;
        let end = self.parse_expression()?;
        self.expect(Token::RightParentheses)?;

        let body = self.parse_block()?;

        Ok(Statement::For {
            variable,
            start,
            end,
            body,
        })
    }

    fn parse_return_statement(&mut self) -> Result<Statement, ParseError> {
        self.expect(Token::Return)?;

        let value = if matches!(self.current_token().token, Token::SemicolonPoint) {
            None
        } else {
            Some(self.parse_expression()?)
        };

        self.expect_semicolon()?;
        Ok(Statement::Return(value))
    }

    fn parse_print_statement(&mut self) -> Result<Statement, ParseError> {
        self.expect(Token::Print)?;
        self.expect(Token::LeftParentheses)?;
        let expression = self.parse_expression()?;
        self.expect(Token::RightParentheses)?;
        self.expect_semicolon()?;

        Ok(Statement::Print(expression))
    }

    fn parse_input_statement(&mut self) -> Result<Statement, ParseError> {
        self.expect(Token::Input)?;
        self.expect(Token::LeftParentheses)?;
        let expression = self.parse_expression()?;
        self.expect(Token::RightParentheses)?;
        self.expect_semicolon()?;

        Ok(Statement::Input(expression))
    }

    fn parse_expression(&mut self) -> Result<Expression, ParseError> {
        self.parse_logical_or()
    }

    fn parse_logical_or(&mut self) -> Result<Expression, ParseError> {
        let mut expr = self.parse_logical_and()?;

        while matches!(self.current_token().token, Token::Or) {
            self.advance();
            let right = self.parse_logical_and()?;
            expr = Expression::BinaryOperation {
                left: Box::new(expr),
                operator: BinaryOperator::Or,
                right: Box::new(right),
            };
        }

        Ok(expr)
    }

    fn parse_logical_and(&mut self) -> Result<Expression, ParseError> {
        let mut expr = self.parse_equality()?;

        while matches!(self.current_token().token, Token::And) {
            self.advance();
            let right = self.parse_equality()?;
            expr = Expression::BinaryOperation {
                left: Box::new(expr),
                operator: BinaryOperator::And,
                right: Box::new(right),
            };
        }

        Ok(expr)
    }

    fn parse_equality(&mut self) -> Result<Expression, ParseError> {
        let mut expr = self.parse_comparison()?;

        while matches!(self.current_token().token, Token::Equal | Token::Unequal) {
            let op = match self.current_token().token {
                Token::Equal => BinaryOperator::Equal,
                Token::Unequal => BinaryOperator::Unequal,
                _ => unreachable!(),
            };
            self.advance();
            let right = self.parse_comparison()?;
            expr = Expression::BinaryOperation {
                left: Box::new(expr),
                operator: op,
                right: Box::new(right),
            };
        }

        Ok(expr)
    }

    fn parse_comparison(&mut self) -> Result<Expression, ParseError> {
        let mut expr = self.parse_term()?;

        while matches!(
            self.current_token().token,
            Token::More | Token::Less | Token::MoreEqual | Token::LessEqual
        ) {
            let op = match self.current_token().token {
                Token::More => BinaryOperator::More,
                Token::Less => BinaryOperator::Less,
                Token::MoreEqual => BinaryOperator::MoreEqual,
                Token::LessEqual => BinaryOperator::LessEqual,
                _ => unreachable!(),
            };
            self.advance();
            let right = self.parse_term()?;
            expr = Expression::BinaryOperation {
                left: Box::new(expr),
                operator: op,
                right: Box::new(right),
            };
        }

        Ok(expr)
    }

    fn parse_term(&mut self) -> Result<Expression, ParseError> {
        let mut expr = self.parse_factor()?;

        while matches!(self.current_token().token, Token::Plus | Token::Minus) {
            let op = match self.current_token().token {
                Token::Plus => BinaryOperator::Plus,
                Token::Minus => BinaryOperator::Minus,
                _ => unreachable!(),
            };
            self.advance();
            let right = self.parse_factor()?;
            expr = Expression::BinaryOperation {
                left: Box::new(expr),
                operator: op,
                right: Box::new(right),
            };
        }

        Ok(expr)
    }

    fn parse_factor(&mut self) -> Result<Expression, ParseError> {
        let mut expr = self.parse_unary()?;

        while matches!(
            self.current_token().token,
            Token::Multiply | Token::Divide | Token::Remainder
        ) {
            let op = match self.current_token().token {
                Token::Multiply => BinaryOperator::Multiply,
                Token::Divide => BinaryOperator::Divide,
                Token::Remainder => BinaryOperator::Remainder,
                _ => unreachable!(),
            };
            self.advance();
            let right = self.parse_unary()?;
            expr = Expression::BinaryOperation {
                left: Box::new(expr),
                operator: op,
                right: Box::new(right),
            };
        }

        Ok(expr)
    }

    fn parse_unary(&mut self) -> Result<Expression, ParseError> {
        match self.current_token().token {
            Token::Minus => {
                self.advance();
                let expr = self.parse_unary()?;
                Ok(Expression::UnaryOperation {
                    operator: UnaryOperator::Negative,
                    operand: Box::new(expr),
                })
            }
            Token::Not => {
                self.advance();
                let expr = self.parse_unary()?;
                Ok(Expression::UnaryOperation {
                    operator: UnaryOperator::Not,
                    operand: Box::new(expr),
                })
            }
            _ => self.parse_primary(),
        }
    }

    fn parse_primary(&mut self) -> Result<Expression, ParseError> {
        match self.current_token().token.clone() {
            Token::NumericalLiteral(n) => {
                self.advance();
                Ok(Expression::Number(n))
            }
            Token::TextLiteral(s) => {
                self.advance();
                Ok(Expression::Text(s))
            }
            Token::True => {
                self.advance();
                Ok(Expression::Boolean(true))
            }
            Token::False => {
                self.advance();
                Ok(Expression::Boolean(false))
            }
            Token::Identifier(mut name) => {
                self.advance();

                // Поддержка составных имён: module.function.subfunction
                while matches!(self.current_token().token, Token::Point) {
                    self.advance();
                    if let Token::Identifier(next_part) = &self.current_token().token {
                        name = format!("{}.{}", name, next_part);
                        self.advance();
                    } else {
                        return Err(ParseError::UnexpectedToken(format!(
                            "Ожидался идентификатор после точки (строка: {}, столбец: {})",
                            self.current_token().span.line,
                            self.current_token().span.column,
                        )));
                    }
                }

                if matches!(self.current_token().token, Token::LeftParentheses) {
                    self.advance();
                    let mut arguments = Vec::new();

                    while !matches!(self.current_token().token, Token::RightParentheses) {
                        arguments.push(self.parse_expression()?);
                        if matches!(self.current_token().token, Token::Comma) {
                            self.advance();
                        }
                    }

                    self.expect(Token::RightParentheses)?;
                    Ok(Expression::CallingFunction { name, arguments })
                } else {
                    Ok(Expression::Identifier(name))
                }
            }
            Token::Input => {
                self.advance();
                self.expect(Token::LeftParentheses)?;
                let expression = self
                    .parse_expression()
                    .unwrap_or_else(|_| Expression::Text("".to_string()));
                self.expect(Token::RightParentheses)?;
                Ok(Expression::Input {
                    argument: Box::from(expression),
                })
            }
            Token::LeftParentheses => {
                self.advance();
                let expr = self.parse_expression()?;
                self.expect(Token::RightParentheses)?;
                Ok(expr)
            }

            _ => Err(ParseError::UnexpectedToken(format!(
                "Неожиданный токен: {:?} (строка: {}, столбец: {})",
                self.current_token().token,
                self.current_token().span.line,
                self.current_token().span.column,
            ))),
        }
    }

    fn expect_semicolon(&mut self) -> Result<(), ParseError> {
        if matches!(self.current_token().token, Token::SemicolonPoint) {
            self.advance();
            Ok(())
        } else {
            Err(ParseError::UnexpectedToken(format!(
                "Ожидалась точка с запятой, но найден: {:?} (строка: {}, столбец: {})",
                self.current_token().token,
                self.current_token().span.line,
                self.current_token().span.column,
            )))
        }
    }
}
