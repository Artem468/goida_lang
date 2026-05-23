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
        Rule::library_stmt => "объявление нативной библиотеки".into(),
        Rule::library_function => "функция нативной библиотеки".into(),
        Rule::library_global => "переменная нативной библиотеки".into(),
        Rule::library_param_list => "список параметров нативной функции".into(),
        Rule::library_param => "параметр нативной функции".into(),
        Rule::constructor => "конструктор".into(),
        Rule::class_method => "метод класса".into(),
        Rule::class_field => "поле класса".into(),
        Rule::inheritance_clause => "наследование класса".into(),

        // Управляющие конструкции
        Rule::if_stmt => "условие 'если'".into(),
        Rule::else_clause => "блок 'иначе'".into(),
        Rule::else_if_clause => "ветка 'иначе если'".into(),
        Rule::try_stmt => "блок 'попробовать'".into(),
        Rule::catch_clause => "блок 'перехватить'".into(),
        Rule::catch_pattern => "шаблон перехвата ошибки".into(),
        Rule::raise_stmt => "инструкция 'выбросить'".into(),
        Rule::while_stmt => "цикл 'пока'".into(),
        Rule::for_stmt => "цикл 'для'".into(),
        Rule::foreach_stmt => "цикл 'для ... в ...'".into(),
        Rule::thread_stmt => "блок потока".into(),
        Rule::return_stmt => "инструкция 'вернуть'".into(),
        Rule::expr_stmt => "выражение как инструкция".into(),
        Rule::block => "блок кода в фигурных скобках { ... }".into(),
        Rule::empty_block => "пустой блок { }".into(),

        // Выражения и литералы
        Rule::string_literal => "строка в кавычках".into(),
        Rule::number_literal => "число".into(),
        Rule::bool_literal => "логическое значение (истина/ложь)".into(),
        Rule::empty_literal => "пустота".into(),
        Rule::new_expr => "создание объекта 'новый'".into(),
        Rule::qualified_name => "составное имя".into(),
        Rule::expression => "выражение".into(),
        Rule::logical_or => "логическое 'или'".into(),
        Rule::logical_and => "логическое 'и'".into(),
        Rule::comparison => "сравнение".into(),
        Rule::addition => "сложение или вычитание".into(),
        Rule::multiplication => "умножение, деление или остаток".into(),
        Rule::unary => "унарное выражение".into(),
        Rule::postfix => "постфиксное выражение".into(),
        Rule::paren_expr => "выражение в скобках".into(),
        Rule::lambda_expr => "лямбда-выражение".into(),
        Rule::lambda_params => "параметры лямбды".into(),

        // Операторы и знаки
        Rule::assignment => "присваивание '='".into(),
        Rule::assignment_expr => "выражение присваивания".into(),
        Rule::property_assign => "присваивание свойства".into(),
        Rule::compound_assignment => "комбинированное присваивание".into(),
        Rule::compound_assign => "комбинированное присваивание (+=, -= и т.д.)".into(),
        Rule::compound_op => "оператор комбинированного присваивания".into(),
        Rule::comp_op => "оператор сравнения".into(),
        Rule::add_op => "сложение или вычитание".into(),
        Rule::mul_op => "умножение или деление".into(),
        Rule::unary_op => "унарный оператор (- или !)".into(),
        Rule::logical_or_op => "оператор 'или'".into(),
        Rule::logical_and_op => "оператор 'и'".into(),
        Rule::in_op => "оператор 'в'".into(),
        Rule::type_hint | Rule::type_name => "название типа".into(),
        Rule::function_call => "вызов функции".into(),
        Rule::return_type => "тип возвращаемого значения (->)".into(),
        Rule::const_mod => "модификатор 'константа'".into(),

        // Пунктуация
        Rule::param_list => "список параметров".into(),
        Rule::param => "параметр".into(),
        Rule::arg_list => "список аргументов".into(),
        Rule::named_arg => "именованный аргумент".into(),
        Rule::property_access => "обращение к свойству через '.'".into(),
        Rule::method_call => "вызов метода".into(),
        Rule::index_access => "доступ по индексу [ ]".into(),
        Rule::visibility => "модификатор доступа (публичный/приватный)".into(),
        Rule::static_mod => "модификатор 'статичный'".into(),

        _ => format!("{:?}", rule),
    }
}

pub fn extract_last_token(code: &str, pos: usize) -> (usize, usize) {
    if code.is_empty() {
        return (0, 0);
    }

    let mut cursor = pos.min(code.len());
    while cursor > 0 && !code.is_char_boundary(cursor) {
        cursor -= 1;
    }

    let chars: Vec<(usize, char)> = code.char_indices().collect();
    let Some(mut idx) = chars.iter().rposition(|(byte_idx, _)| {
        *byte_idx < cursor || (*byte_idx == cursor && cursor < code.len())
    }) else {
        return (0, 0);
    };

    while idx > 0 && chars[idx].1.is_whitespace() {
        idx -= 1;
    }

    let c = chars[idx].1;
    fn is_ident(c: char) -> bool {
        c.is_alphanumeric() || c == '_'
    }
    fn is_operator(c: char) -> bool {
        matches!(
            c,
            '=' | '!' | '<' | '>' | '+' | '-' | '*' | '/' | '%' | ':' | '.'
        )
    }

    if c == '"' || c == '\'' {
        let quote = c;

        let mut start_idx = idx;
        while start_idx > 0 {
            start_idx -= 1;
            if chars[start_idx].1 == quote {
                break;
            }
        }

        let mut end_idx = idx + 1;
        while end_idx < chars.len() {
            if chars[end_idx].1 == quote {
                end_idx += 1;
                break;
            }
            end_idx += 1;
        }

        return (
            chars[start_idx].0,
            chars
                .get(end_idx)
                .map(|(byte_idx, _)| *byte_idx)
                .unwrap_or(code.len()),
        );
    }

    if is_ident(c) {
        let mut start_idx = idx;
        let mut end_idx = idx + 1;

        while start_idx > 0 && is_ident(chars[start_idx - 1].1) {
            start_idx -= 1;
        }

        while end_idx < chars.len() && is_ident(chars[end_idx].1) {
            end_idx += 1;
        }

        return (
            chars[start_idx].0,
            chars
                .get(end_idx)
                .map(|(byte_idx, _)| *byte_idx)
                .unwrap_or(code.len()),
        );
    }

    if c.is_ascii_digit() {
        let mut start_idx = idx;
        let mut end_idx = idx + 1;
        let mut seen_dot = false;

        while start_idx > 0 {
            let ch = chars[start_idx - 1].1;
            if ch == '.' && !seen_dot {
                seen_dot = true;
                start_idx -= 1;
            } else if ch.is_ascii_digit() {
                start_idx -= 1;
            } else {
                break;
            }
        }

        while end_idx < chars.len() {
            let ch = chars[end_idx].1;
            if ch == '.' && !seen_dot {
                seen_dot = true;
                end_idx += 1;
            } else if ch.is_ascii_digit() {
                end_idx += 1;
            } else {
                break;
            }
        }

        return (
            chars[start_idx].0,
            chars
                .get(end_idx)
                .map(|(byte_idx, _)| *byte_idx)
                .unwrap_or(code.len()),
        );
    }

    if is_operator(c) {
        let mut start_idx = idx;
        let mut end_idx = idx + 1;

        while start_idx > 0 && is_operator(chars[start_idx - 1].1) {
            start_idx -= 1;
        }

        while end_idx < chars.len() && is_operator(chars[end_idx].1) {
            end_idx += 1;
        }

        return (
            chars[start_idx].0,
            chars
                .get(end_idx)
                .map(|(byte_idx, _)| *byte_idx)
                .unwrap_or(code.len()),
        );
    }

    (
        chars[idx].0,
        chars
            .get(idx + 1)
            .map(|(byte_idx, _)| *byte_idx)
            .unwrap_or(code.len()),
    )
}
