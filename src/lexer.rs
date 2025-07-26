#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    // Ключевые слова
    Let,      // let
    If,       // if
    Else,     // else
    While,    // while
    For,      // for
    Function, // function
    Return,   // return
    Number,   // int/number type
    Text,     // string type
    Boolean,  // bool type
    True,     // true
    False,    // false
    Print,    // print
    Input,    // input
    Import,   // import

    // Литералы
    NumericalLiteral(i64),
    TextLiteral(String),
    Identifier(String),

    // Операторы
    Plus,      // +
    Minus,     // -
    Multiply,  // *
    Divide,    // /
    Remainder, // %
    Equal,     // ==
    Unequal,   // !=
    More,      // >
    Less,      // <
    MoreEqual, // >=
    LessEqual, // <=
    And,       // &&
    Or,        // ||
    Not,       // !
    Assign,    // =

    // Пунктуация
    LeftParentheses,  // (
    RightParentheses, // )
    LeftBrace,        // {
    RightBrace,       // }
    LeftBracket,      // [
    RightBracket,     // ]
    SemicolonPoint,   // ;
    Comma,            // ,
    Point,            // .
    Colon,            // :

    // Специальные
    EndFile,
    NewLine,
}

pub struct Lexer {
    input: Vec<char>,
    position: usize,
    current_char: Option<char>,
}

impl Lexer {
    pub fn new(input: String) -> Self {
        let chars: Vec<char> = input.chars().collect();
        let current_char = chars.get(0).copied();

        Lexer {
            input: chars,
            position: 0,
            current_char,
        }
    }

    fn advance(&mut self) {
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

        while let Some(ch) = self.current_char {
            if ch.is_ascii_digit() {
                number.push(ch);
                self.advance();
            } else {
                break;
            }
        }

        Token::NumericalLiteral(number.parse().unwrap_or(0))
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
            "текст" => Token::Text,
            "логический" => Token::Boolean,
            "истина" => Token::True,
            "ложь" => Token::False,
            "печать" => Token::Print,
            "ввод" => Token::Input,
            "подключить" => Token::Import,
            _ => Token::Identifier(identifier),
        }
    }

    pub fn next_token(&mut self) -> Token {
        loop {
            match self.current_char {
                None => return Token::EndFile,

                Some(ch) if ch.is_whitespace() && ch != '\n' => {
                    self.skip_whitespace();
                    continue;
                }

                Some('\n') => {
                    self.advance();
                    return Token::NewLine;
                }

                Some(ch) if ch.is_ascii_digit() => {
                    return self.read_number();
                }

                Some('"') => {
                    return self.read_string();
                }

                Some(ch) if ch.is_alphabetic() || ch == '_' => {
                    return self.read_identifier();
                }

                Some('+') => {
                    self.advance();
                    return Token::Plus;
                }

                Some('-') => {
                    self.advance();
                    return Token::Minus;
                }

                Some('*') => {
                    self.advance();
                    return Token::Multiply;
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
                        return Token::Divide;
                    }
                }

                Some('%') => {
                    self.advance();
                    return Token::Remainder;
                }

                Some('=') => {
                    if self.peek() == Some('=') {
                        self.advance();
                        self.advance();
                        return Token::Equal;
                    } else {
                        self.advance();
                        return Token::Assign;
                    }
                }

                Some('!') => {
                    if self.peek() == Some('=') {
                        self.advance();
                        self.advance();
                        return Token::Unequal;
                    } else {
                        self.advance();
                        return Token::Not;
                    }
                }

                Some('>') => {
                    if self.peek() == Some('=') {
                        self.advance();
                        self.advance();
                        return Token::MoreEqual;
                    } else {
                        self.advance();
                        return Token::More;
                    }
                }

                Some('<') => {
                    if self.peek() == Some('=') {
                        self.advance();
                        self.advance();
                        return Token::LessEqual;
                    } else {
                        self.advance();
                        return Token::Less;
                    }
                }

                Some('&') => {
                    if self.peek() == Some('&') {
                        self.advance();
                        self.advance();
                        return Token::And;
                    } else {
                        self.advance();
                        continue;
                    }
                }

                Some('|') => {
                    if self.peek() == Some('|') {
                        self.advance();
                        self.advance();
                        return Token::Or;
                    } else {
                        self.advance();
                        continue;
                    }
                }

                Some('(') => {
                    self.advance();
                    return Token::LeftParentheses;
                }

                Some(')') => {
                    self.advance();
                    return Token::RightParentheses;
                }

                Some('{') => {
                    self.advance();
                    return Token::LeftBrace;
                }

                Some('}') => {
                    self.advance();
                    return Token::RightBrace;
                }

                Some('[') => {
                    self.advance();
                    return Token::LeftBracket;
                }

                Some(']') => {
                    self.advance();
                    return Token::RightBracket;
                }

                Some(';') => {
                    self.advance();
                    return Token::SemicolonPoint;
                }

                Some(',') => {
                    self.advance();
                    return Token::Comma;
                }

                Some('.') => {
                    self.advance();
                    return Token::Point;
                }

                Some(':') => {
                    self.advance();
                    return Token::Colon;
                }

                Some(_) => {
                    self.advance();
                    continue;
                }
            }
        }
    }

    pub fn tokenize(&mut self) -> Vec<Token> {
        let mut tokens = Vec::new();

        loop {
            let token = self.next_token();
            let is_eof = matches!(token, Token::EndFile);

            if !matches!(token, Token::NewLine) {
                tokens.push(token);
            }

            if is_eof {
                break;
            }
        }

        tokens
    }
}
