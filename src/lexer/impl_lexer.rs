use crate::lexer::structs::{Lexer, Token, TokenInfo};
use crate::ast::{Span, SourceLocation};

impl Lexer {
    pub fn new(input: String) -> Self {
        let chars: Vec<char> = input.chars().collect();
        let current_char = chars.get(0).copied();

        Lexer {
            input: chars,
            position: 0,
            current_char,
            current_line: 0,
            current_column: 0,
        }
    }

    fn advance(&mut self) {
        if let Some('\n') = self.current_char {
            self.current_line += 1;
            self.current_column = 1;
        } else {
            self.current_column += 1;
        }

        self.position += 1;
        self.current_char = self.input.get(self.position).copied();
    }

    fn peek(&self) -> Option<char> {
        self.input.get(self.position + 1).copied()
    }

    fn skip_whitespace(&mut self) {
        while let Some(ch) = self.current_char {
            if ch.is_whitespace() && ch != '\n' {
                self.advance();
            } else {
                break;
            }
        }
    }

    fn read_number(&mut self) -> Token {
        let mut number = String::new();
        let mut has_dot = false;

        while let Some(ch) = self.current_char {
            if ch.is_ascii_digit() {
                number.push(ch);
                self.advance();
            } else if ch == '.' && !has_dot {
                if let Some(next) = self.peek() {
                    if next.is_ascii_digit() {
                        has_dot = true;
                        number.push(ch);
                        self.advance();
                    } else {
                        break;
                    }
                } else {
                    break;
                }
            } else {
                break;
            }
        }

        if has_dot {
            Token::FloatLiteral(number.parse::<f64>().unwrap_or(0.0))
        } else {
            Token::NumericalLiteral(number.parse::<i64>().unwrap_or(0))
        }
    }

    fn read_string(&mut self) -> Token {
        let mut string = String::new();
        self.advance();

        while let Some(ch) = self.current_char {
            if ch == '"' {
                self.advance();
                break;
            }
            if ch == '\\' {
                self.advance();
                match self.current_char {
                    Some('n') => string.push('\n'),
                    Some('t') => string.push('\t'),
                    Some('r') => string.push('\r'),
                    Some('\\') => string.push('\\'),
                    Some('"') => string.push('"'),
                    Some(c) => string.push(c),
                    None => break,
                }
            } else {
                string.push(ch);
            }
            self.advance();
        }

        Token::TextLiteral(string)
    }

    fn read_identifier(&mut self) -> Token {
        let mut identifier = String::new();

        while let Some(ch) = self.current_char {
            if ch.is_alphabetic() || ch == '_' || ch.is_numeric() {
                identifier.push(ch);
                self.advance();
            } else {
                break;
            }
        }

        match identifier.as_str() {
            "пусть" => Token::Let,
            "если" => Token::If,
            "иначе" => Token::Else,
            "пока" => Token::While,
            "для" => Token::For,
            "функция" => Token::Function,
            "вернуть" => Token::Return,
            "число" => Token::Number,
            "дробь" => Token::Float,
            "текст" => Token::Text,
            "логический" => Token::Boolean,
            "список" => Token::List,
            "словарь" => Token::Dict,
            "истина" => Token::True,
            "ложь" => Token::False,
            "печать" => Token::Print,
            "ввод" => Token::Input,
            "подключить" => Token::Import,
            _ => Token::Identifier(identifier),
        }
    }

    pub fn next_token(&mut self) -> TokenInfo {
        loop {
            let token = match self.current_char {
                None => Token::EndFile,

                Some(ch) if ch.is_whitespace() && ch != '\n' => {
                    self.skip_whitespace();
                    continue;
                }

                Some('\n') => {
                    self.advance();
                    Token::NewLine
                }

                Some(ch) if ch.is_ascii_digit() => {
                    self.read_number()
                }

                Some('"') => {
                    self.read_string()
                }

                Some(ch) if ch.is_alphabetic() || ch == '_' => {
                    self.read_identifier()
                }

                Some('+') => {
                    self.advance();
                    Token::Plus
                }

                Some('-') => {
                    self.advance();
                    Token::Minus
                }

                Some('*') => {
                    self.advance();
                    Token::Multiply
                }

                Some('/') => {
                    if self.peek() == Some('/') {
                        self.advance();
                        self.advance();
                        while let Some(ch) = self.current_char {
                            if ch == '\n' {
                                break;
                            }
                            self.advance();
                        }
                        continue;
                    } else {
                        self.advance();
                        Token::Divide
                    }
                }

                Some('%') => {
                    self.advance();
                    Token::Remainder
                }

                Some('=') => {
                    if self.peek() == Some('=') {
                        self.advance();
                        self.advance();
                        Token::Equal
                    } else {
                        self.advance();
                        Token::Assign
                    }
                }

                Some('!') => {
                    if self.peek() == Some('=') {
                        self.advance();
                        self.advance();
                        Token::Unequal
                    } else {
                        self.advance();
                        Token::Not
                    }
                }

                Some('>') => {
                    if self.peek() == Some('=') {
                        self.advance();
                        self.advance();
                        Token::MoreEqual
                    } else {
                        self.advance();
                        Token::More
                    }
                }

                Some('<') => {
                    if self.peek() == Some('=') {
                        self.advance();
                        self.advance();
                        Token::LessEqual
                    } else {
                        self.advance();
                        Token::Less
                    }
                }

                Some('&') => {
                    if self.peek() == Some('&') {
                        self.advance();
                        self.advance();
                        Token::And
                    } else {
                        self.advance();
                        continue;
                    }
                }

                Some('|') => {
                    if self.peek() == Some('|') {
                        self.advance();
                        self.advance();
                        Token::Or
                    } else {
                        self.advance();
                        continue;
                    }
                }

                Some('(') => {
                    self.advance();
                    Token::LeftParentheses
                }

                Some(')') => {
                    self.advance();
                    Token::RightParentheses
                }

                Some('{') => {
                    self.advance();
                    Token::LeftBrace
                }

                Some('}') => {
                    self.advance();
                    Token::RightBrace
                }

                Some('[') => {
                    self.advance();
                    Token::LeftBracket
                }

                Some(']') => {
                    self.advance();
                    Token::RightBracket
                }

                Some(';') => {
                    self.advance();
                    Token::SemicolonPoint
                }

                Some(',') => {
                    self.advance();
                    Token::Comma
                }

                Some('.') => {
                    self.advance();
                    Token::Point
                }

                Some(':') => {
                    self.advance();
                    Token::Colon
                }

                Some(_) => {
                    self.advance();
                    continue;
                }
            };

            return TokenInfo {
                token,
                span: Span {
                    start: SourceLocation {
                        line: self.current_line,
                        column: self.current_column,
                        offset: self.position as u32,
                    },
                    end: SourceLocation {
                        line: self.current_line,
                        column: self.current_column,
                        offset: self.position as u32,
                    },
                },
            }

        }
    }

    pub fn tokenize(&mut self) -> Vec<TokenInfo> {
        let mut tokens = Vec::new();

        loop {
            let token = self.next_token();
            let is_eof = matches!(token.token, Token::EndFile);

            if !matches!(token.token, Token::NewLine) {
                tokens.push(token);
            }

            if is_eof {
                break;
            }
        }

        tokens
    }
}