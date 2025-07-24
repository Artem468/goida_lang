use crate::lexer::Token;
use crate::ast::*;

pub struct Parser {
    tokens: Vec<Token>,
    current: usize,
}

#[derive(Debug)]
pub enum ParseError {
    UnexpectedToken(String),
    UnexpectedEndOfFile,
    InvalidSyntax(String),
}

impl Parser {
    pub fn new(tokens: Vec<Token>) -> Self {
        Parser { tokens, current: 0 }
    }
    
    fn current_token(&self) -> &Token {
        self.tokens.get(self.current).unwrap_or(&Token::КонецФайла)
    }
    
    fn peek_token(&self) -> &Token {
        self.tokens.get(self.current + 1).unwrap_or(&Token::КонецФайла)
    }
    
    fn advance(&mut self) -> &Token {
        if self.current < self.tokens.len() {
            self.current += 1;
        }
        self.current_token()
    }
    
    fn expect(&mut self, expected: Token) -> Result<(), ParseError> {
        if std::mem::discriminant(self.current_token()) == std::mem::discriminant(&expected) {
            self.advance();
            Ok(())
        } else {
            Err(ParseError::UnexpectedToken(format!(
                "Ожидался {:?}, получен {:?}",
                expected,
                self.current_token()
            )))
        }
    }
    
    pub fn parse(&mut self) -> Result<Program, ParseError> {
        let mut функции = Vec::new();
        let mut операторы = Vec::new();
        
        while !matches!(self.current_token(), Token::КонецФайла) {
            match self.current_token() {
                Token::Функция => {
                    функции.push(self.parse_function()?);
                }
                _ => {
                    операторы.push(self.parse_statement()?);
                }
            }
        }
        
        Ok(Program { функции, операторы })
    }
    
    fn parse_function(&mut self) -> Result<Function, ParseError> {
        self.expect(Token::Функция)?;
        
        let имя = match self.current_token() {
            Token::Идентификатор(name) => {
                let name = name.clone();
                self.advance();
                name
            }
            _ => return Err(ParseError::UnexpectedToken("Ожидался идентификатор функции".to_string())),
        };
        
        self.expect(Token::ЛеваяСкобка)?;
        
        let mut параметры = Vec::new();
        while !matches!(self.current_token(), Token::ПраваяСкобка) {
            let param_name = match self.current_token() {
                Token::Идентификатор(name) => {
                    let name = name.clone();
                    self.advance();
                    name
                }
                _ => return Err(ParseError::UnexpectedToken("Ожидался идентификатор параметра".to_string())),
            };
            
            self.expect(Token::Двоеточие)?;
            
            let param_type = self.parse_type()?;
            параметры.push(Parameter { имя: param_name, тип: param_type });
            
            if matches!(self.current_token(), Token::Запятая) {
                self.advance();
            }
        }
        
        self.expect(Token::ПраваяСкобка)?;
        
        let возвращаемый_тип = if matches!(self.current_token(), Token::Двоеточие) {
            self.advance();
            Some(self.parse_type()?)
        } else {
            None
        };
        
        let тело = self.parse_block()?;
        
        Ok(Function {
            имя,
            параметры,
            возвращаемый_тип,
            тело,
        })
    }
    
    fn parse_type(&mut self) -> Result<DataType, ParseError> {
        match self.current_token() {
            Token::Число => {
                self.advance();
                Ok(DataType::Число)
            }
            Token::Текст => {
                self.advance();
                Ok(DataType::Текст)
            }
            Token::Логический => {
                self.advance();
                Ok(DataType::Логический)
            }
            _ => Err(ParseError::UnexpectedToken("Ожидался тип данных".to_string())),
        }
    }
    
    fn parse_block(&mut self) -> Result<Vec<Statement>, ParseError> {
        self.expect(Token::ЛеваяФигурная)?;
        
        let mut statements = Vec::new();
        while !matches!(self.current_token(), Token::ПраваяФигурная) {
            statements.push(self.parse_statement()?);
        }
        
        self.expect(Token::ПраваяФигурная)?;
        Ok(statements)
    }
    
    fn parse_statement(&mut self) -> Result<Statement, ParseError> {
        match self.current_token() {
            Token::Пусть => self.parse_declaration(),
            Token::Если => self.parse_if_statement(),
            Token::Пока => self.parse_while_statement(),
            Token::Для => self.parse_for_statement(),
            Token::Вернуть => self.parse_return_statement(),
            Token::Печать => self.parse_print_statement(),
            Token::ЛеваяФигурная => {
                let block = self.parse_block()?;
                Ok(Statement::Блок(block))
            }
            Token::Идентификатор(_) => {
                if matches!(self.peek_token(), Token::Присвоить) {
                    self.parse_assignment()
                } else {
                    let expr = self.parse_expression()?;
                    self.expect_semicolon()?;
                    Ok(Statement::Выражение(expr))
                }
            }
            _ => {
                let expr = self.parse_expression()?;
                self.expect_semicolon()?;
                Ok(Statement::Выражение(expr))
            }
        }
    }
    
    fn parse_declaration(&mut self) -> Result<Statement, ParseError> {
        self.expect(Token::Пусть)?;
        
        let имя = match self.current_token() {
            Token::Идентификатор(name) => {
                let name = name.clone();
                self.advance();
                name
            }
            _ => return Err(ParseError::UnexpectedToken(format!("Ожидался идентификатор переменной, но получен токен: {:?}", self.current_token()))),
        };
        
        let тип = if matches!(self.current_token(), Token::Двоеточие) {
            self.advance();
            Some(self.parse_type()?)
        } else {
            None
        };
        
        let значение = if matches!(self.current_token(), Token::Присвоить) {
            self.advance();
            Some(self.parse_expression()?)
        } else {
            None
        };
        
        self.expect_semicolon()?;
        
        Ok(Statement::Объявление { имя, тип, значение })
    }
    
    fn parse_assignment(&mut self) -> Result<Statement, ParseError> {
        let имя = match self.current_token() {
            Token::Идентификатор(name) => {
                let name = name.clone();
                self.advance();
                name
            }
            _ => return Err(ParseError::UnexpectedToken("Ожидался идентификатор переменной".to_string())),
        };
        
        self.expect(Token::Присвоить)?;
        let значение = self.parse_expression()?;
        self.expect_semicolon()?;
        
        Ok(Statement::Присваивание { имя, значение })
    }
    
    fn parse_if_statement(&mut self) -> Result<Statement, ParseError> {
        self.expect(Token::Если)?;
        self.expect(Token::ЛеваяСкобка)?;
        let условие = self.parse_expression()?;
        self.expect(Token::ПраваяСкобка)?;
        
        let тело = self.parse_block()?;
        
        let иначе = if matches!(self.current_token(), Token::Иначе) {
            self.advance();
            Some(self.parse_block()?)
        } else {
            None
        };
        
        Ok(Statement::Если { условие, тело, иначе })
    }
    
    fn parse_while_statement(&mut self) -> Result<Statement, ParseError> {
        self.expect(Token::Пока)?;
        self.expect(Token::ЛеваяСкобка)?;
        let условие = self.parse_expression()?;
        self.expect(Token::ПраваяСкобка)?;
        
        let тело = self.parse_block()?;
        
        Ok(Statement::Пока { условие, тело })
    }
    
    fn parse_for_statement(&mut self) -> Result<Statement, ParseError> {
        self.expect(Token::Для)?;
        self.expect(Token::ЛеваяСкобка)?;
        
        let переменная = match self.current_token() {
            Token::Идентификатор(name) => {
                let name = name.clone();
                self.advance();
                name
            }
            _ => return Err(ParseError::UnexpectedToken("Ожидался идентификатор переменной цикла".to_string())),
        };
        
        self.expect(Token::Присвоить)?;
        let начало = self.parse_expression()?;
        self.expect(Token::ТочкаСЗапятой)?;
        let конец = self.parse_expression()?;
        self.expect(Token::ПраваяСкобка)?;
        
        let тело = self.parse_block()?;
        
        Ok(Statement::Для { переменная, начало, конец, тело })
    }
    
    fn parse_return_statement(&mut self) -> Result<Statement, ParseError> {
        self.expect(Token::Вернуть)?;
        
        let значение = if matches!(self.current_token(), Token::ТочкаСЗапятой) {
            None
        } else {
            Some(self.parse_expression()?)
        };
        
        self.expect_semicolon()?;
        Ok(Statement::Возврат(значение))
    }
    
    fn parse_print_statement(&mut self) -> Result<Statement, ParseError> {
        self.expect(Token::Печать)?;
        self.expect(Token::ЛеваяСкобка)?;
        let выражение = self.parse_expression()?;
        self.expect(Token::ПраваяСкобка)?;
        self.expect_semicolon()?;
        
        Ok(Statement::Печать(выражение))
    }
    
    fn parse_expression(&mut self) -> Result<Expression, ParseError> {
        self.parse_logical_or()
    }
    
    fn parse_logical_or(&mut self) -> Result<Expression, ParseError> {
        let mut expr = self.parse_logical_and()?;
        
        while matches!(self.current_token(), Token::Или) {
            self.advance();
            let right = self.parse_logical_and()?;
            expr = Expression::БинарнаяОперация {
                левый: Box::new(expr),
                оператор: BinaryOperator::Или,
                правый: Box::new(right),
            };
        }
        
        Ok(expr)
    }
    
    fn parse_logical_and(&mut self) -> Result<Expression, ParseError> {
        let mut expr = self.parse_equality()?;
        
        while matches!(self.current_token(), Token::И) {
            self.advance();
            let right = self.parse_equality()?;
            expr = Expression::БинарнаяОперация {
                левый: Box::new(expr),
                оператор: BinaryOperator::И,
                правый: Box::new(right),
            };
        }
        
        Ok(expr)
    }
    
    fn parse_equality(&mut self) -> Result<Expression, ParseError> {
        let mut expr = self.parse_comparison()?;
        
        while matches!(self.current_token(), Token::Равно | Token::НеРавно) {
            let op = match self.current_token() {
                Token::Равно => BinaryOperator::Равно,
                Token::НеРавно => BinaryOperator::НеРавно,
                _ => unreachable!(),
            };
            self.advance();
            let right = self.parse_comparison()?;
            expr = Expression::БинарнаяОперация {
                левый: Box::new(expr),
                оператор: op,
                правый: Box::new(right),
            };
        }
        
        Ok(expr)
    }
    
    fn parse_comparison(&mut self) -> Result<Expression, ParseError> {
        let mut expr = self.parse_term()?;
        
        while matches!(self.current_token(), Token::Больше | Token::Меньше | Token::БольшеРавно | Token::МеньшеРавно) {
            let op = match self.current_token() {
                Token::Больше => BinaryOperator::Больше,
                Token::Меньше => BinaryOperator::Меньше,
                Token::БольшеРавно => BinaryOperator::БольшеРавно,
                Token::МеньшеРавно => BinaryOperator::МеньшеРавно,
                _ => unreachable!(),
            };
            self.advance();
            let right = self.parse_term()?;
            expr = Expression::БинарнаяОперация {
                левый: Box::new(expr),
                оператор: op,
                правый: Box::new(right),
            };
        }
        
        Ok(expr)
    }
    
    fn parse_term(&mut self) -> Result<Expression, ParseError> {
        let mut expr = self.parse_factor()?;
        
        while matches!(self.current_token(), Token::Плюс | Token::Минус) {
            let op = match self.current_token() {
                Token::Плюс => BinaryOperator::Плюс,
                Token::Минус => BinaryOperator::Минус,
                _ => unreachable!(),
            };
            self.advance();
            let right = self.parse_factor()?;
            expr = Expression::БинарнаяОперация {
                левый: Box::new(expr),
                оператор: op,
                правый: Box::new(right),
            };
        }
        
        Ok(expr)
    }
    
    fn parse_factor(&mut self) -> Result<Expression, ParseError> {
        let mut expr = self.parse_unary()?;
        
        while matches!(self.current_token(), Token::Умножить | Token::Разделить | Token::Остаток) {
            let op = match self.current_token() {
                Token::Умножить => BinaryOperator::Умножить,
                Token::Разделить => BinaryOperator::Разделить,
                Token::Остаток => BinaryOperator::Остаток,
                _ => unreachable!(),
            };
            self.advance();
            let right = self.parse_unary()?;
            expr = Expression::БинарнаяОперация {
                левый: Box::new(expr),
                оператор: op,
                правый: Box::new(right),
            };
        }
        
        Ok(expr)
    }
    
    fn parse_unary(&mut self) -> Result<Expression, ParseError> {
        match self.current_token() {
            Token::Минус => {
                self.advance();
                let expr = self.parse_unary()?;
                Ok(Expression::УнарнаяОперация {
                    оператор: UnaryOperator::Минус,
                    операнд: Box::new(expr),
                })
            }
            Token::Не => {
                self.advance();
                let expr = self.parse_unary()?;
                Ok(Expression::УнарнаяОперация {
                    оператор: UnaryOperator::Не,
                    операнд: Box::new(expr),
                })
            }
            _ => self.parse_primary(),
        }
    }
    
    fn parse_primary(&mut self) -> Result<Expression, ParseError> {
        match self.current_token().clone() {
            Token::ЧисловойЛитерал(n) => {
                self.advance();
                Ok(Expression::Число(n))
            }
            Token::ТекстовыйЛитерал(s) => {
                self.advance();
                Ok(Expression::Текст(s))
            }
            Token::Истина => {
                self.advance();
                Ok(Expression::Логический(true))
            }
            Token::Ложь => {
                self.advance();
                Ok(Expression::Логический(false))
            }
            Token::Идентификатор(name) => {
                self.advance();
                if matches!(self.current_token(), Token::ЛеваяСкобка) {
                    // Вызов функции
                    self.advance();
                    let mut аргументы = Vec::new();
                    
                    while !matches!(self.current_token(), Token::ПраваяСкобка) {
                        аргументы.push(self.parse_expression()?);
                        if matches!(self.current_token(), Token::Запятая) {
                            self.advance();
                        }
                    }
                    
                    self.expect(Token::ПраваяСкобка)?;
                    Ok(Expression::ВызовФункции { имя: name, аргументы })
                } else {
                    Ok(Expression::Идентификатор(name))
                }
            }
            Token::ЛеваяСкобка => {
                self.advance();
                let expr = self.parse_expression()?;
                self.expect(Token::ПраваяСкобка)?;
                Ok(expr)
            }
            Token::Ввод => {
                self.advance();
                self.expect(Token::ЛеваяСкобка)?;
                self.expect(Token::ПраваяСкобка)?;
                Ok(Expression::ВызовФункции {
                    имя: "ввод".to_string(),
                    аргументы: vec![],
                })
            }
            _ => Err(ParseError::UnexpectedToken(format!(
                "Неожиданный токен: {:?}",
                self.current_token()
            ))),
        }
    }
    
    fn expect_semicolon(&mut self) -> Result<(), ParseError> {
        if matches!(self.current_token(), Token::ТочкаСЗапятой) {
            self.advance();
            Ok(())
        } else {
            Err(ParseError::UnexpectedToken(format!(
                "Ожидалась точка с запятой, но найден: {:?}",
                self.current_token()
            )))
        }
    }
}
