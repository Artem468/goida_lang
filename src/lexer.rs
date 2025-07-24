#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    // Ключевые слова
    Пусть,      // let
    Если,       // if
    Иначе,      // else
    Пока,       // while
    Для,        // for
    Функция,    // function
    Вернуть,    // return
    Число,      // int/number type
    Текст,      // string type
    Логический, // bool type
    Истина,     // true
    Ложь,       // false
    Печать,     // print
    Ввод,       // input
    
    // Литералы
    ЧисловойЛитерал(i64),
    ТекстовыйЛитерал(String),
    Идентификатор(String),
    
    // Операторы
    Плюс,           // +
    Минус,          // -
    Умножить,       // *
    Разделить,      // /
    Остаток,        // %
    Равно,          // ==
    НеРавно,        // !=
    Больше,         // >
    Меньше,         // <
    БольшеРавно,    // >=
    МеньшеРавно,    // <=
    И,              // &&
    Или,            // ||
    Не,             // !
    Присвоить,      // =
    
    // Пунктуация
    ЛеваяСкобка,    // (
    ПраваяСкобка,   // )
    ЛеваяФигурная,  // {
    ПраваяФигурная, // }
    ЛеваяКвадратная, // [
    ПраваяКвадратная, // ]
    ТочкаСЗапятой,  // ;
    Запятая,        // ,
    Точка,          // .
    Двоеточие,      // :
    
    // Специальные
    КонецФайла,
    НоваяСтрока,
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
        
        Token::ЧисловойЛитерал(number.parse().unwrap_or(0))
    }
    
    fn read_string(&mut self) -> Token {
        let mut string = String::new();
        self.advance(); // Пропускаем открывающую кавычку
        
        while let Some(ch) = self.current_char {
            if ch == '"' {
                self.advance(); // Пропускаем закрывающую кавычку
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
        
        Token::ТекстовыйЛитерал(string)
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
        
        // Проверяем ключевые слова
        match identifier.as_str() {
            "пусть" => Token::Пусть,
            "если" => Token::Если,
            "иначе" => Token::Иначе,
            "пока" => Token::Пока,
            "для" => Token::Для,
            "функция" => Token::Функция,
            "вернуть" => Token::Вернуть,
            "число" => Token::Число,
            "текст" => Token::Текст,
            "логический" => Token::Логический,
            "истина" => Token::Истина,
            "ложь" => Token::Ложь,
            "печать" => Token::Печать,
            "ввод" => Token::Ввод,
            _ => Token::Идентификатор(identifier),
        }
    }
    
    pub fn next_token(&mut self) -> Token {
        loop {
            match self.current_char {
                None => return Token::КонецФайла,
                
                Some(ch) if ch.is_whitespace() && ch != '\n' => {
                    self.skip_whitespace();
                    continue;
                }
                
                Some('\n') => {
                    self.advance();
                    return Token::НоваяСтрока;
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
                    return Token::Плюс;
                }
                
                Some('-') => {
                    self.advance();
                    return Token::Минус;
                }
                
                Some('*') => {
                    self.advance();
                    return Token::Умножить;
                }
                
                Some('/') => {
                    if self.peek() == Some('/') {
                        // Однострочный комментарий
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
                        return Token::Разделить;
                    }
                }
                
                Some('%') => {
                    self.advance();
                    return Token::Остаток;
                }
                
                Some('=') => {
                    if self.peek() == Some('=') {
                        self.advance();
                        self.advance();
                        return Token::Равно;
                    } else {
                        self.advance();
                        return Token::Присвоить;
                    }
                }
                
                Some('!') => {
                    if self.peek() == Some('=') {
                        self.advance();
                        self.advance();
                        return Token::НеРавно;
                    } else {
                        self.advance();
                        return Token::Не;
                    }
                }
                
                Some('>') => {
                    if self.peek() == Some('=') {
                        self.advance();
                        self.advance();
                        return Token::БольшеРавно;
                    } else {
                        self.advance();
                        return Token::Больше;
                    }
                }
                
                Some('<') => {
                    if self.peek() == Some('=') {
                        self.advance();
                        self.advance();
                        return Token::МеньшеРавно;
                    } else {
                        self.advance();
                        return Token::Меньше;
                    }
                }
                
                Some('&') => {
                    if self.peek() == Some('&') {
                        self.advance();
                        self.advance();
                        return Token::И;
                    } else {
                        self.advance();
                        continue; // Пропускаем одиночный &
                    }
                }
                
                Some('|') => {
                    if self.peek() == Some('|') {
                        self.advance();
                        self.advance();
                        return Token::Или;
                    } else {
                        self.advance();
                        continue; // Пропускаем одиночный |
                    }
                }
                
                Some('(') => {
                    self.advance();
                    return Token::ЛеваяСкобка;
                }
                
                Some(')') => {
                    self.advance();
                    return Token::ПраваяСкобка;
                }
                
                Some('{') => {
                    self.advance();
                    return Token::ЛеваяФигурная;
                }
                
                Some('}') => {
                    self.advance();
                    return Token::ПраваяФигурная;
                }
                
                Some('[') => {
                    self.advance();
                    return Token::ЛеваяКвадратная;
                }
                
                Some(']') => {
                    self.advance();
                    return Token::ПраваяКвадратная;
                }
                
                Some(';') => {
                    self.advance();
                    return Token::ТочкаСЗапятой;
                }
                
                Some(',') => {
                    self.advance();
                    return Token::Запятая;
                }
                
                Some('.') => {
                    self.advance();
                    return Token::Точка;
                }
                
                Some(':') => {
                    self.advance();
                    return Token::Двоеточие;
                }
                
                Some(_) => {
                    self.advance();
                    continue; // Пропускаем неизвестные символы
                }
            }
        }
    }
    
    pub fn tokenize(&mut self) -> Vec<Token> {
        let mut tokens = Vec::new();
        
        loop {
            let token = self.next_token();
            let is_eof = matches!(token, Token::КонецФайла);
            
            if !matches!(token, Token::НоваяСтрока) {
                tokens.push(token);
            }
            
            if is_eof {
                break;
            }
        }
        
        tokens
    }
}
