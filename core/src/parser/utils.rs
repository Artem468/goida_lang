use crate::parser::parser::Rule;
pub fn translate_rule(rule: &Rule) -> String {
    match rule {
        // Служебные терминалы
        Rule::EOI => "конец файла".into(),
        Rule::identifier => "идентификатор (имя)".into(),

        // Объявления (Ключевые слова)
        Rule::import_stmt => "инструкция 'подключить'".into(),
        Rule::function => "объявление функции".into(),
        Rule::class => "объявление класса".into(),
        Rule::constructor => "конструктор".into(),
        Rule::class_method => "метод класса".into(),
        Rule::class_field => "поле класса".into(),

        // Управляющие конструкции
        Rule::if_stmt => "условие 'если'".into(),
        Rule::else_clause => "блок 'иначе'".into(),
        Rule::while_stmt => "цикл 'пока'".into(),
        Rule::for_stmt => "цикл 'для'".into(),
        Rule::return_stmt => "инструкция 'вернуть'".into(),
        Rule::block => "блок кода в фигурных скобках { ... }".into(),

        // Выражения и литералы
        Rule::string_literal => "строка в кавычках".into(),
        Rule::number_literal => "число".into(),
        Rule::bool_literal => "логическое значение (истина/ложь)".into(),
        Rule::empty_literal => "пустота".into(),
        Rule::this_expr => "ключевое слово 'это'".into(),
        Rule::new_expr => "создание объекта 'новый'".into(),
        Rule::expression => "выражение".into(),
        Rule::paren_expr => "выражение в скобках".into(),

        // Операторы и знаки
        Rule::assignment => "присваивание '='".into(),
        Rule::compound_assign => "комбинированное присваивание (+=, -= и т.д.)".into(),
        Rule::comp_op => "оператор сравнения".into(),
        Rule::add_op => "сложение или вычитание".into(),
        Rule::mul_op => "умножение или деление".into(),
        Rule::unary_op => "унарный оператор (- или !)".into(),
        Rule::type_hint | Rule::type_name => "название типа".into(),
        Rule::function_call => "вызов функции".into(),
        Rule::return_type => "тип возвращаемого значения (->)".into(),

        // Пунктуация
        Rule::param_list => "список параметров".into(),
        Rule::arg_list => "список аргументов".into(),
        Rule::property_access => "обращение к свойству через '.'".into(),
        Rule::method_call => "вызов метода".into(),
        Rule::index_access => "доступ по индексу [ ]".into(),
        Rule::visibility => "модификатор доступа (публичный/приватный)".into(),
        Rule::static_mod => "модификатор 'статичный'".into(),

        _ => format!("{:?}", rule),
    }
}

pub fn extract_last_token(code: &str, pos: usize) -> (usize, usize) {
    let bytes = code.as_bytes();
    let len = bytes.len();

    if len == 0 {
        return (0, 0);
    }
    let mut pos = pos.min(len.saturating_sub(1));

    while pos > 0 && bytes[pos].is_ascii_whitespace() {
        pos -= 1;
    }

    let c = bytes[pos];
    fn is_ident(c: u8) -> bool {
        c.is_ascii_alphanumeric() || c == b'_'
    }
    fn is_operator(c: u8) -> bool {
        matches!(
            c,
            b'=' | b'!' | b'<' | b'>' | b'+' | b'-' | b'*' | b'/' | b'%' | b':' | b'.'
        )
    }

    if c == b'"' || c == b'\'' {
        let quote = c;

        let mut start = pos;
        while start > 0 {
            start -= 1;
            if bytes[start] == quote {
                break;
            }
        }

        let mut end = pos + 1;
        while end < len {
            if bytes[end] == quote {
                end += 1;
                break;
            }
            end += 1;
        }

        return (start, end);
    }

    if is_ident(c) {
        let mut start = pos;
        let mut end = pos;

        while start > 0 && is_ident(bytes[start - 1]) {
            start -= 1;
        }

        while end < len && is_ident(bytes[end]) {
            end += 1;
        }

        return (start, end);
    }

    if c.is_ascii_digit() {
        let mut start = pos;
        let mut end = pos;
        let mut seen_dot = false;

        while start > 0 {
            let ch = bytes[start - 1];
            if ch == b'.' && !seen_dot {
                seen_dot = true;
                start -= 1;
            } else if ch.is_ascii_digit() {
                start -= 1;
            } else {
                break;
            }
        }

        while end < len {
            let ch = bytes[end];
            if ch == b'.' && !seen_dot {
                seen_dot = true;
                end += 1;
            } else if ch.is_ascii_digit() {
                end += 1;
            } else {
                break;
            }
        }

        return (start, end);
    }

    if is_operator(c) {
        let mut start = pos;
        let mut end = pos + 1;

        while start > 0 && is_operator(bytes[start - 1]) {
            start -= 1;
        }

        while end < len && is_operator(bytes[end]) {
            end += 1;
        }

        return (start, end);
    }

    (pos, pos + 1)
}
