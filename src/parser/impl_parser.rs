use crate::ast::*;
use crate::lexer::structs::{Token, TokenInfo};
use crate::parser::structs::{ParseError, Parser};

impl Parser {
    pub fn new(tokens: Vec<TokenInfo>, name: String) -> Self {
        let program = Program::new(name);
        Parser { program, tokens, current: 0 }
    }

    fn current_token(&self) -> TokenInfo {
        self.tokens.get(self.current).cloned().unwrap_or(TokenInfo {
            token: Token::EndFile,
            span: Span::default(),
        })
    }

    fn advance(&mut self) {
        if self.current < self.tokens.len() {
            self.current += 1;
        }
    }

    fn expect(&mut self, expected: Token) -> Result<(), ParseError> {
        if std::mem::discriminant(&self.current_token().token) == std::mem::discriminant(&expected) {
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

    pub fn parse(mut self) -> Result<Program, ParseError> {
        while !matches!(self.current_token().token, Token::EndFile) {
            match self.current_token().token {
                Token::Function => {
                    let function = self.parse_function()?;
                    self.program.functions.push(function);
                }
                Token::Import => {
                    let import = self.parse_import()?;
                    self.program.imports.push(import);
                }
                _ => {
                    let stmt = self.parse_statement()?;
                    self.program.statements.push(stmt);
                }
            }
        }
        Ok(self.program)
    }

    fn parse_parameter(&mut self) -> Result<Parameter, ParseError> {
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

    fn parse_function(&mut self) -> Result<Function, ParseError> {
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

    fn parse_import(&mut self) -> Result<Import, ParseError> {
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

    fn parse_type(&mut self) -> Result<TypeId, ParseError> {
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
                let element_type = self.program.arena.get_type(element_type_id)
                    .ok_or_else(|| ParseError::InternalError("Не удалось получить тип элемента".to_string()))?
                    .clone();
                self.expect(Token::RightBracket)?;
                DataType::List(Box::new(element_type))
            }
            Token::Dict => {
                self.advance();
                self.expect(Token::LeftBracket)?;
                let key_type_id = self.parse_type()?;
                let key_type = self.program.arena.get_type(key_type_id)
                    .ok_or_else(|| ParseError::InternalError("Не удалось получить тип ключа".to_string()))?
                    .clone();
                self.expect(Token::Comma)?;
                let value_type_id = self.parse_type()?;
                let value_type = self.program.arena.get_type(value_type_id)
                    .ok_or_else(|| ParseError::InternalError("Не удалось получить тип значения".to_string()))?
                    .clone();
                self.expect(Token::RightBracket)?;
                DataType::Dict {
                    key: Box::new(key_type),
                    value: Box::new(value_type),
                }
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

    fn parse_block(&mut self) -> Result<StmtId, ParseError> {
        self.expect(Token::LeftBrace)?;
        let statements = self.parse_block_statements()?;
        self.expect(Token::RightBrace)?;

        let span = self.current_token().span;
        Ok(self.program.arena.add_statement(StatementKind::Block(statements), span))
    }

    fn parse_block_statements(&mut self) -> Result<Vec<StmtId>, ParseError> {
        let mut statements = Vec::new();
        while !matches!(self.current_token().token, Token::RightBrace | Token::EndFile) {
            statements.push(self.parse_statement()?);
        }
        Ok(statements)
    }

    fn parse_statement(&mut self) -> Result<StmtId, ParseError> {
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
            Token::Identifier(_) => {
                let mut lookahead = 1;
                let mut has_index_access = false;

                while self.current + lookahead < self.tokens.len() {
                    let token = &self.tokens[self.current + lookahead].token;
                    match token {
                        Token::LeftBracket => {
                            has_index_access = true;
                            lookahead += 1;
                            let mut bracket_count = 1;
                            while bracket_count > 0 && self.current + lookahead < self.tokens.len() {
                                match &self.tokens[self.current + lookahead].token {
                                    Token::LeftBracket => bracket_count += 1,
                                    Token::RightBracket => bracket_count -= 1,
                                    _ => {}
                                }
                                lookahead += 1;
                            }
                        }
                        Token::Assign => {
                            if has_index_access {
                                return self.parse_index_assignment();
                            } else {
                                return self.parse_assignment();
                            }
                        }
                        _ => break,
                    }
                }
                
                let expr = self.parse_expression()?;
                self.expect_semicolon()?;
                Ok(self.program.arena.add_statement(StatementKind::Expression(expr), span))
            }
            _ => {
                let expr = self.parse_expression()?;
                self.expect_semicolon()?;
                Ok(self.program.arena.add_statement(StatementKind::Expression(expr), span))
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
            span
        ))
    }

    fn parse_assignment(&mut self) -> Result<StmtId, ParseError> {
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

        self.expect(Token::Assign)?;
        let value = self.parse_expression()?;
        self.expect_semicolon()?;

        Ok(self.program.arena.add_statement(
            StatementKind::Assign { name, value },
            span
        ))
    }

    fn parse_index_assignment(&mut self) -> Result<StmtId, ParseError> {
        let span = self.current_token().span;
        let object_expr = self.parse_postfix()?;
        
        let (object, index) = self.extract_index_access(object_expr)?;

        self.expect(Token::Assign)?;
        let value = self.parse_expression()?;
        self.expect_semicolon()?;

        Ok(self.program.arena.add_statement(
            StatementKind::IndexAssign { object, index, value },
            span
        ))
    }

    fn extract_index_access(&self, expr_id: ExprId) -> Result<(ExprId, ExprId), ParseError> {
        let expr = self.program.arena.get_expression(expr_id)
            .ok_or_else(|| ParseError::InternalError("Не удалось получить выражение".to_string()))?;

        match &expr.kind {
            ExpressionKind::Index { object, index } => Ok((*object, *index)),
            _ => Err(ParseError::UnexpectedToken(
                "Ожидался доступ по индексу перед присваиванием".to_string(),
            )),
        }
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
            span
        ))
    }

    fn parse_while_statement(&mut self) -> Result<StmtId, ParseError> {
        self.expect(Token::While)?;
        let span = self.current_token().span;

        self.expect(Token::LeftParentheses)?;
        let condition = self.parse_expression()?;
        self.expect(Token::RightParentheses)?;

        let body = self.parse_block()?;

        Ok(self.program.arena.add_statement(
            StatementKind::While { condition, body },
            span
        ))
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
            span
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

        Ok(self.program.arena.add_statement(
            StatementKind::Return(value),
            span
        ))
    }

    fn parse_print_statement(&mut self) -> Result<StmtId, ParseError> {
        self.expect(Token::Print)?;
        let span = self.current_token().span;

        self.expect(Token::LeftParentheses)?;
        let expression = self.parse_expression()?;
        self.expect(Token::RightParentheses)?;
        self.expect_semicolon()?;

        Ok(self.program.arena.add_statement(
            StatementKind::Print(expression),
            span
        ))
    }

    fn parse_input_statement(&mut self) -> Result<StmtId, ParseError> {
        self.expect(Token::Input)?;
        let span = self.current_token().span;

        self.expect(Token::LeftParentheses)?;
        let expression = self.parse_expression()?;
        self.expect(Token::RightParentheses)?;
        self.expect_semicolon()?;

        Ok(self.program.arena.add_statement(
            StatementKind::Input(expression),
            span
        ))
    }

    fn parse_expression(&mut self) -> Result<ExprId, ParseError> {
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
                span
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
                span
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
                span
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
                span
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
                span
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
                span
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
                    span
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
                    span
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
                        self.advance();
                        
                        if let Some(expr_node) = self.program.arena.get_expression(expr) {
                            if let ExpressionKind::Identifier(module_symbol) = expr_node.kind {
                                let module_name = self.program.arena.resolve_symbol(module_symbol).unwrap();
                                let full_name = format!("{}.{}", module_name, member_name);
                                let full_symbol = self.program.arena.intern_string(&full_name);
                                expr = self.program.arena.add_expression(
                                    ExpressionKind::Identifier(full_symbol),
                                    member_token.span
                                );
                            } else {
                                return Err(ParseError::UnexpectedToken(
                                    "Точечная нотация применима только к идентификаторам".to_string(),
                                ));
                            }
                        } else {
                            return Err(ParseError::InternalError(
                                "Не удалось получить выражение для точечной нотации".to_string(),
                            ));
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
                        span
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
                                ExpressionKind::Call { function: expr, args: arguments },
                                span
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
                Ok(self.program.arena.add_expression(
                    ExpressionKind::Literal(LiteralValue::Number(n)),
                    span
                ))
            }
            Token::FloatLiteral(f) => {
                self.advance();
                Ok(self.program.arena.add_expression(
                    ExpressionKind::Literal(LiteralValue::Float(f)),
                    span
                ))
            }
            Token::TextLiteral(s) => {
                self.advance();
                let symbol = self.program.arena.intern_string(&s);
                Ok(self.program.arena.add_expression(
                    ExpressionKind::Literal(LiteralValue::Text(symbol)),
                    span
                ))
            }
            Token::True => {
                self.advance();
                Ok(self.program.arena.add_expression(
                    ExpressionKind::Literal(LiteralValue::Boolean(true)),
                    span
                ))
            }
            Token::False => {
                self.advance();
                Ok(self.program.arena.add_expression(
                    ExpressionKind::Literal(LiteralValue::Boolean(false)),
                    span
                ))
            }
            Token::Identifier(name) => {
                let symbol = self.program.arena.intern_string(&name);
                self.advance();
                Ok(self.program.arena.add_expression(
                    ExpressionKind::Identifier(symbol),
                    span
                ))
            }
            Token::Input => {
                self.advance();
                self.expect(Token::LeftParentheses)?;
                let prompt = self.parse_expression()?;
                self.expect(Token::RightParentheses)?;
                Ok(self.program.arena.add_expression(
                    ExpressionKind::Input(prompt),
                    span
                ))
            }
            Token::LeftParentheses => {
                self.advance();
                let expr = self.parse_expression()?;
                self.expect(Token::RightParentheses)?;
                Ok(expr)
            }
            Token::LeftBracket => {
                self.advance();
                let mut elements = Vec::new();

                while !matches!(self.current_token().token, Token::RightBracket) {
                    elements.push(self.parse_expression()?);
                    if matches!(self.current_token().token, Token::Comma) {
                        self.advance();
                    }
                }

                self.expect(Token::RightBracket)?;
                Ok(self.program.arena.add_expression(
                    ExpressionKind::List(elements),
                    span
                ))
            }
            Token::LeftBrace => {
                self.advance();
                let mut pairs = Vec::new();

                while !matches!(self.current_token().token, Token::RightBrace) {
                    let key = self.parse_expression()?;
                    self.expect(Token::Colon)?;
                    let value = self.parse_expression()?;
                    pairs.push((key, value));

                    if matches!(self.current_token().token, Token::Comma) {
                        self.advance();
                    }
                }

                self.expect(Token::RightBrace)?;
                Ok(self.program.arena.add_expression(
                    ExpressionKind::Dict(pairs),
                    span
                ))
            }
            Token::Size | Token::Push | Token::Pop | Token::Remove | Token::Contains => {
                let name = match self.current_token().token {
                    Token::Size => "длина",
                    Token::Push => "добавить",
                    Token::Pop => "извлечь",
                    Token::Remove => "удалить",
                    Token::Contains => "содержит",
                    _ => unreachable!(),
                };
                let symbol = self.program.arena.intern_string(name);
                self.advance();
                Ok(self.program.arena.add_expression(
                    ExpressionKind::Identifier(symbol),
                    span
                ))
            }
            _ => Err(ParseError::UnexpectedToken(format!(
                "Неожиданный токен: {:?} в позиции {}:{}",
                self.current_token().token,
                self.current_token().span.start.line,
                self.current_token().span.start.column,
            ))),
        }
    }

    fn expect_semicolon(&mut self) -> Result<(), ParseError> {
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