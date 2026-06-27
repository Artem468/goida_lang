use crate::parser::prelude::FormatLanguage;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiagnosticLanguage {
    English,
    Russian,
}

impl DiagnosticLanguage {
    pub fn detect(source: &str) -> Self {
        match FormatLanguage::detect(source) {
            FormatLanguage::English => Self::English,
            FormatLanguage::Russian => Self::Russian,
        }
    }

    pub fn select(self, english: &'static str, russian: &'static str) -> &'static str {
        match self {
            Self::English => english,
            Self::Russian => russian,
        }
    }
}

impl From<FormatLanguage> for DiagnosticLanguage {
    fn from(value: FormatLanguage) -> Self {
        match value {
            FormatLanguage::English => Self::English,
            FormatLanguage::Russian => Self::Russian,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiagnosticMessage {
    StaticMethodRequiresDoubleColon,
    InstanceMethodRequiresDot,
}

impl DiagnosticMessage {
    pub fn render(self, language: DiagnosticLanguage) -> String {
        let text = match self {
            Self::StaticMethodRequiresDoubleColon => language.select(
                "Static method must be called with '::'",
                "Статический метод нужно вызывать через '::'",
            ),
            Self::InstanceMethodRequiresDot => language.select(
                "Instance method must be called with '.'",
                "Метод объекта нужно вызывать через '.'",
            ),
        };
        text.to_string()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiagnosticTitle {
    ParseTypeError,
    TypeError,
    SyntaxError,
    ImportError,
    UndefinedVariable,
    UndefinedFunction,
    UndefinedMethod,
    TypeMismatch,
    Panic,
    DivisionByZero,
    InvalidOperation,
    IoError,
    UnexpectedReturn,
    StackTrace,
    At,
}

impl DiagnosticTitle {
    pub fn render(self, language: DiagnosticLanguage) -> &'static str {
        match self {
            Self::ParseTypeError => language.select("Type error", "Ошибка типов"),
            Self::TypeError => language.select("Type error", "Ошибка типа"),
            Self::SyntaxError => language.select("Syntax error", "Ошибка синтаксиса"),
            Self::ImportError => language.select("Import error", "Ошибка импорта"),
            Self::UndefinedVariable => {
                language.select("Undefined variable", "Неопределенная переменная")
            }
            Self::UndefinedFunction => {
                language.select("Undefined function", "Неопределенная функция")
            }
            Self::UndefinedMethod => language.select("Undefined method", "Неопределенный метод"),
            Self::TypeMismatch => language.select("Type mismatch", "Несоответствие типов"),
            Self::Panic => language.select("Panic", "Паника"),
            Self::DivisionByZero => language.select("Division by zero", "Деление на ноль"),
            Self::InvalidOperation => language.select("Invalid operation", "Недопустимая операция"),
            Self::IoError => language.select("I/O error", "Ошибка ввода-вывода"),
            Self::UnexpectedReturn => language.select("Unexpected return", "Неожиданный return"),
            Self::StackTrace => language.select("Stack trace:", "Стек вызовов:"),
            Self::At => language.select("at", "в"),
        }
    }
}

pub fn localize_message(message: &str, language: DiagnosticLanguage) -> String {
    if message.is_empty() {
        return String::new();
    }

    if let Some(translated) = translate_exact(message, language) {
        return translated.to_string();
    }

    if let Some(rest) = message.strip_prefix("Использование: ") {
        return match language {
            DiagnosticLanguage::English => format!("Usage: {}", translate_usage(rest, language)),
            DiagnosticLanguage::Russian => message.to_string(),
        };
    }
    if let Some(rest) = message.strip_prefix("Usage: ") {
        return match language {
            DiagnosticLanguage::English => message.to_string(),
            DiagnosticLanguage::Russian => {
                format!("Использование: {}", translate_usage(rest, language))
            }
        };
    }

    for (from, to) in PREFIX_TRANSLATIONS {
        let (source, target) = match language {
            DiagnosticLanguage::English => (from.russian, from.english),
            DiagnosticLanguage::Russian => (from.english, from.russian),
        };
        if let Some(rest) = message.strip_prefix(source) {
            return translate_fragments(&format!("{target}{rest}"), language);
        }
        let (source, target) = match language {
            DiagnosticLanguage::English => (to.russian, to.english),
            DiagnosticLanguage::Russian => (to.english, to.russian),
        };
        if let Some(rest) = message.strip_prefix(source) {
            return translate_fragments(&format!("{target}{rest}"), language);
        }
    }
    for entry in PREFIX_TRANSLATIONS_SIMPLE {
        let (source, target) = match language {
            DiagnosticLanguage::English => (entry.russian, entry.english),
            DiagnosticLanguage::Russian => (entry.english, entry.russian),
        };
        if let Some(rest) = message.strip_prefix(source) {
            return translate_fragments(&format!("{target}{rest}"), language);
        }
    }

    translate_fragments(message, language)
}

fn translate_exact(message: &str, language: DiagnosticLanguage) -> Option<&'static str> {
    EXACT_TRANSLATIONS.iter().find_map(|entry| {
        if message == entry.english || message == entry.russian {
            Some(match language {
                DiagnosticLanguage::English => entry.english,
                DiagnosticLanguage::Russian => entry.russian,
            })
        } else {
            None
        }
    })
}

fn translate_usage(message: &str, language: DiagnosticLanguage) -> String {
    let mut translated = message.to_string();
    for entry in USAGE_TERMS {
        match language {
            DiagnosticLanguage::English => {
                translated = translated.replace(entry.russian, entry.english);
            }
            DiagnosticLanguage::Russian => {
                translated = translated.replace(entry.english, entry.russian);
            }
        }
    }
    translated
}

fn translate_fragments(message: &str, language: DiagnosticLanguage) -> String {
    let mut translated = message.to_string();
    for entry in FRAGMENT_TRANSLATIONS {
        match language {
            DiagnosticLanguage::English => {
                translated = translated.replace(entry.russian, entry.english);
            }
            DiagnosticLanguage::Russian => {
                translated = translated.replace(entry.english, entry.russian);
            }
        }
    }
    translated
}

#[derive(Clone, Copy)]
struct Translation {
    english: &'static str,
    russian: &'static str,
}

const EXACT_TRANSLATIONS: &[Translation] = &[
    Translation {
        english: "Static method must be called with '::'",
        russian: "Статический метод нужно вызывать через '::'",
    },
    Translation {
        english: "Instance method must be called with '.'",
        russian: "Метод объекта нужно вызывать через '.'",
    },
    Translation {
        english: "Instance method needs an object",
        russian: "Методу объекта нужен объект",
    },
    Translation {
        english: "Division by zero",
        russian: "Деление на ноль",
    },
    Translation {
        english: "Division by 0 is forbidden",
        russian: "Деление на 0 запрещено",
    },
    Translation {
        english: "Current module is missing",
        russian: "Текущий модуль не найден",
    },
    Translation {
        english: "Module is missing",
        russian: "Модуль не найден",
    },
    Translation {
        english: "Module member is missing",
        russian: "Элемент модуля не найден",
    },
    Translation {
        english: "Module member is not callable",
        russian: "Элемент модуля нельзя вызвать",
    },
    Translation {
        english: "Class is missing",
        russian: "Класс не найден",
    },
    Translation {
        english: "Method is missing",
        russian: "Метод не найден",
    },
    Translation {
        english: "Method is private",
        russian: "Метод приватный",
    },
    Translation {
        english: "Property is missing",
        russian: "Свойство не найдено",
    },
    Translation {
        english: "Property is private",
        russian: "Свойство приватное",
    },
    Translation {
        english: "Property is not accessible",
        russian: "Свойство недоступно",
    },
    Translation {
        english: "Property is not static",
        russian: "Свойство не статическое",
    },
    Translation {
        english: "Static property is not initialized",
        russian: "Статическое свойство не инициализировано",
    },
    Translation {
        english: "Property receiver is invalid",
        russian: "Некорректный получатель свойства",
    },
    Translation {
        english: "Expected object",
        russian: "Ожидался объект",
    },
    Translation {
        english: "Value cannot be indexed",
        russian: "Значение нельзя индексировать",
    },
    Translation {
        english: "Value cannot be assigned by index",
        russian: "Значению нельзя присвоить по индексу",
    },
    Translation {
        english: "Index out of bounds",
        russian: "Индекс вне диапазона",
    },
    Translation {
        english: "Dictionary key is missing",
        russian: "Ключ словаря не найден",
    },
    Translation {
        english: "Value is not callable",
        russian: "Значение нельзя вызвать",
    },
    Translation {
        english: "Value is not iterable",
        russian: "Значение не является итерируемым",
    },
    Translation {
        english: "Cannot redefine a constant",
        russian: "Нельзя переопределить константу",
    },
    Translation {
        english: "Cannot assign to a constant",
        russian: "Нельзя изменить константу",
    },
    Translation {
        english: "Variable is missing",
        russian: "Переменная не найдена",
    },
    Translation {
        english: "Compiled function body is missing",
        russian: "Тело скомпилированной функции не найдено",
    },
    Translation {
        english: "Compiled method body is missing",
        russian: "Тело скомпилированного метода не найдено",
    },
    Translation {
        english: "Thread lock is poisoned",
        russian: "Блокировка потока повреждена",
    },
    Translation {
        english: "Thread panicked",
        russian: "Поток завершился паникой",
    },
    Translation {
        english: "Thread can only be created from a function",
        russian: "Поток можно создать только из функции",
    },
    Translation {
        english: "Thread expects a function",
        russian: "Поток ожидает функцию",
    },
    Translation {
        english: "Mutex is poisoned",
        russian: "Мьютекс поврежден",
    },
    Translation {
        english: "Read-write lock is poisoned",
        russian: "Блокировка чтения-записи повреждена",
    },
    Translation {
        english: "Expected Thread object",
        russian: "Ожидался объект Поток",
    },
    Translation {
        english: "Expected Thread",
        russian: "Ожидался Поток",
    },
    Translation {
        english: "Expected Mutex object",
        russian: "Ожидался объект Мьютекс",
    },
    Translation {
        english: "Expected Mutex",
        russian: "Ожидался Мьютекс",
    },
    Translation {
        english: "Expected RwLock object",
        russian: "Ожидался объект БлокировкаЧтенияЗаписи",
    },
    Translation {
        english: "Expected RwLock",
        russian: "Ожидалась БлокировкаЧтенияЗаписи",
    },
    Translation {
        english: "Expected list",
        russian: "Ожидался список",
    },
    Translation {
        english: "Expected array",
        russian: "Ожидался массив",
    },
    Translation {
        english: "Expected dictionary",
        russian: "Ожидался словарь",
    },
    Translation {
        english: "Expected string",
        russian: "Ожидалась строка",
    },
    Translation {
        english: "Expected iterator",
        russian: "Ожидался итератор",
    },
    Translation {
        english: "Expected function",
        russian: "Ожидалась функция",
    },
    Translation {
        english: "Empty list cannot be removed from",
        russian: "Удаление у пустого списка",
    },
    Translation {
        english: "Value cannot be converted to iterator",
        russian: "Значение нельзя преобразовать в итератор",
    },
    Translation {
        english: "Iterator expects a collection",
        russian: "Итератор ожидает коллекцию",
    },
    Translation {
        english: "Type not found",
        russian: "Тип не найден",
    },
    Translation {
        english: "Module not found",
        russian: "Модуль не найден",
    },
    Translation {
        english: "Date/time value is out of range",
        russian: "Значение даты/времени вне диапазона",
    },
    Translation {
        english: "Date/time arithmetic overflow",
        russian: "Переполнение при вычислении даты/времени",
    },
    Translation {
        english: "Argument must be a number",
        russian: "Аргумент должен быть числом",
    },
    Translation {
        english: "Path is missing",
        russian: "Путь не найден",
    },
    Translation {
        english: "Subtraction can only be applied to numbers",
        russian: "Вычитание применимо только к числам",
    },
    Translation {
        english: "Multiplication can only be applied to numbers",
        russian: "Умножение применимо только к числам",
    },
    Translation {
        english: "Division can only be applied to numbers",
        russian: "Деление применимо только к числам",
    },
    Translation {
        english: "Comparison can only be applied to numbers",
        russian: "Сравнение применимо только к числам",
    },
    Translation {
        english: "Unary minus can only be applied to numbers",
        russian: "Унарный минус применим только к числам",
    },
    Translation {
        english: "Unknown native type id",
        russian: "Неизвестный id native-типа",
    },
    Translation {
        english: "Native function argument must be 'number'",
        russian: "Аргумент native-функции должен быть типа 'число'",
    },
    Translation {
        english: "Native function argument must be 'float'",
        russian: "Аргумент native-функции должен быть типа 'дробь'",
    },
    Translation {
        english: "Type 'void' cannot be used for a native function argument",
        russian: "Тип 'пустота' нельзя использовать для аргумента native-функции",
    },
    Translation {
        english: "Native string pointer is null",
        russian: "Указатель на native-строку равен null",
    },
    Translation {
        english: "Function 'from_json' expects a string",
        russian: "Функция 'из_json' ожидает строку",
    },
    Translation {
        english: "Function regex expects a string",
        russian: "Функция регулярное_выражение ожидает строку",
    },
    Translation {
        english: "Expected RegularExpression object",
        russian: "Ожидался объект РегулярноеВыражение",
    },
    Translation {
        english: "Expected declaration of macro",
        russian: "Ожидалось объявление макроса",
    },
    Translation {
        english: "Macro repeat variable used outside repetition",
        russian: "Повторяемая переменная использована вне повторения",
    },
    Translation {
        english: "Macro repeat index out of range",
        russian: "Индекс повторения макроса вне диапазона",
    },
    Translation {
        english: "Expression node not found",
        russian: "Не найдена нода для выражения",
    },
    Translation {
        english: "Invalid function call arguments",
        russian: "Неверные аргументы вызова функции",
    },
    Translation {
        english: "Function argument was passed more than once",
        russian: "Аргумент функции передан несколько раз",
    },
    Translation {
        english: "Required function argument was not passed",
        russian: "Не передан обязательный аргумент функции",
    },
    Translation {
        english: "Invalid token",
        russian: "Некорректный токен",
    },
    Translation {
        english: "Unexpected end of file",
        russian: "Неожиданный конец файла",
    },
    Translation {
        english: "Named arguments must follow positional ones",
        russian: "Именованные аргументы должны идти после позиционных",
    },
    Translation {
        english: "Native library path is missing",
        russian: "Путь к native-библиотеке не указан",
    },
    Translation {
        english: "Native parameter cannot be void",
        russian: "Native-параметр не может иметь тип пустота",
    },
    Translation {
        english: "Pointer argument must be an address, void, or string/list/array/dictionary value",
        russian: "Аргумент типа 'указатель' должен быть адресом, пустотой или значением строка/список/массив/словарь",
    },
    Translation {
        english: "Named arguments must follow positional arguments",
        russian: "Именованные аргументы должны идти после позиционных",
    },
];

const PREFIX_TRANSLATIONS: &[(Translation, Translation)] = &[
    (
        Translation {
            english: "Failed to parse JSON: ",
            russian: "Ошибка разбора JSON: ",
        },
        Translation {
            english: "JSON serialization error: ",
            russian: "Ошибка сериализации JSON: ",
        },
    ),
    (
        Translation {
            english: "Native string is not valid UTF-8: ",
            russian: "Native-строка не является корректным UTF-8: ",
        },
        Translation {
            english: "Failed to resolve current directory: ",
            russian: "Не удалось определить текущий каталог: ",
        },
    ),
];

const PREFIX_TRANSLATIONS_SIMPLE: &[Translation] = &[
    Translation {
        english: "Failed to read ",
        russian: "Не удалось прочитать ",
    },
    Translation {
        english: "Failed to remove stale dependency ",
        russian: "Не удалось удалить устаревшую зависимость ",
    },
    Translation {
        english: "Dependency name conflict for ",
        russian: "Конфликт имени зависимости для ",
    },
    Translation {
        english: "Cyclic package dependency detected at ",
        russian: "Обнаружена циклическая зависимость пакетов в ",
    },
    Translation {
        english: "Failed to resolve installed dependency path for ",
        russian: "Не удалось определить путь установленной зависимости для ",
    },
    Translation {
        english: "Dependency source is missing",
        russian: "Источник зависимости не указан",
    },
    Translation {
        english: "Built package ",
        russian: "Собран пакет ",
    },
    Translation {
        english: "Unsupported goida.lock version ",
        russian: "Неподдерживаемая версия goida.lock ",
    },
    Translation {
        english: "prebuilt package ",
        russian: "предсобранный пакет ",
    },
    Translation {
        english: "build command ",
        russian: "команда сборки ",
    },
    Translation {
        english: "Failed to install native artifact ",
        russian: "Не удалось установить native-артефакт ",
    },
    Translation {
        english: "Invalid artifact destination ",
        russian: "Некорректное назначение артефакта ",
    },
    Translation {
        english: "Failed to create artifact directory ",
        russian: "Не удалось создать каталог артефакта ",
    },
    Translation {
        english: "Failed to resolve package root ",
        russian: "Не удалось определить корень пакета ",
    },
    Translation {
        english: "Failed to resolve artifact directory ",
        russian: "Не удалось определить каталог артефакта ",
    },
    Translation {
        english: "Artifact destination ",
        russian: "Назначение артефакта ",
    },
    Translation {
        english: "build.command program must not be empty",
        russian: "program в build.command не должен быть пустым",
    },
    Translation {
        english: "build artifact source and destination must not be empty",
        russian: "source и destination артефакта сборки не должны быть пустыми",
    },
    Translation {
        english: "Build workdir ",
        russian: "Рабочий каталог сборки ",
    },
    Translation {
        english: "build.command must contain a program",
        russian: "build.command должен содержать программу",
    },
    Translation {
        english: "Failed to start build command ",
        russian: "Не удалось запустить команду сборки ",
    },
    Translation {
        english: "Build command for ",
        russian: "Команда сборки для ",
    },
    Translation {
        english: "Package path ",
        russian: "Путь пакета ",
    },
    Translation {
        english: "Failed to resolve package path ",
        russian: "Не удалось определить путь пакета ",
    },
    Translation {
        english: "Failed to get cwd",
        russian: "Не удалось получить текущий каталог",
    },
    Translation {
        english: "Current directory does not contain goida.toml",
        russian: "Текущий каталог не содержит goida.toml",
    },
    Translation {
        english: "Unexpected token ",
        russian: "Неожиданный токен ",
    },
    Translation {
        english: "Extra token ",
        russian: "Лишний токен ",
    },
    Translation {
        english: "Native library not found: ",
        russian: "Native-библиотека не найдена: ",
    },
    Translation {
        english: "Failed to normalize native library path ",
        russian: "Не удалось нормализовать путь к native-библиотеке ",
    },
    Translation {
        english: "Failed to load native library ",
        russian: "Не удалось загрузить native-библиотеку ",
    },
    Translation {
        english: "Native library ",
        russian: "Native-библиотека ",
    },
    Translation {
        english: "Failed to find symbol ",
        russian: "Не удалось найти символ ",
    },
    Translation {
        english: "Global ",
        russian: "Глобальная переменная ",
    },
    Translation {
        english: "Native parameter ",
        russian: "Native-параметр ",
    },
    Translation {
        english: "Unknown native type id ",
        russian: "Неизвестный id native-типа ",
    },
];

const USAGE_TERMS: &[Translation] = &[
    Translation {
        english: "list",
        russian: "список",
    },
    Translation {
        english: "iterator",
        russian: "итератор",
    },
    Translation {
        english: "regex",
        russian: "регулярное_выражение",
    },
    Translation {
        english: "function",
        russian: "функция",
    },
    Translation {
        english: "string",
        russian: "строка",
    },
    Translation {
        english: "number",
        russian: "число",
    },
    Translation {
        english: "value",
        russian: "значение",
    },
    Translation {
        english: "text",
        russian: "текст",
    },
    Translation {
        english: "replacement",
        russian: "замена",
    },
    Translation {
        english: "initial",
        russian: "начальное_значение",
    },
    Translation {
        english: "prefix",
        russian: "префикс",
    },
    Translation {
        english: "suffix",
        russian: "суффикс",
    },
    Translation {
        english: "pattern",
        russian: "шаблон",
    },
];

const FRAGMENT_TRANSLATIONS: &[Translation] = &[
    Translation {
        english: "additionally failed to restore",
        russian: "дополнительно не удалось восстановить",
    },
    Translation {
        english: "did not provide required artifact",
        russian: "не предоставил обязательный артефакт",
    },
    Translation {
        english: "failed with status",
        russian: "завершилась со статусом",
    },
    Translation {
        english: "is not a directory",
        russian: "не является каталогом",
    },
    Translation {
        english: "escapes the package",
        russian: "выходит за пределы пакета",
    },
    Translation {
        english: "must stay inside the package",
        russian: "должен оставаться внутри пакета",
    },
    Translation {
        english: "must not be a symbolic link",
        russian: "не должен быть символической ссылкой",
    },
    Translation {
        english: "; expected: ",
        russian: "; ожидалось: ",
    },
    Translation {
        english: " is not loaded",
        russian: " не загружена",
    },
    Translation {
        english: " cannot have void type",
        russian: " не может иметь тип пустота",
    },
    Translation {
        english: " is not initialized",
        russian: " не инициализирована",
    },
    Translation {
        english: " expects i64 value",
        russian: " ожидает значение i64",
    },
    Translation {
        english: " expects f64 value",
        russian: " ожидает значение f64",
    },
    Translation {
        english: " expects pointer value",
        russian: " ожидает значение-указатель",
    },
    Translation {
        english: "Unknown named argument",
        russian: "Неизвестный именованный аргумент",
    },
    Translation {
        english: "was passed more than once",
        russian: "передан несколько раз",
    },
    Translation {
        english: "was not passed",
        russian: "не передан",
    },
    Translation {
        english: "expects",
        russian: "ожидает",
    },
    Translation {
        english: "arguments, got",
        russian: "аргументов, получено",
    },
    Translation {
        english: "Invalid return value type: expected",
        russian: "Неверный тип return value: ожидался",
    },
    Translation {
        english: "Invalid global type: expected",
        russian: "Неверный тип global: ожидался",
    },
    Translation {
        english: "Unsupported type for native ABI:",
        russian: "Неподдерживаемый тип для native ABI:",
    },
    Translation {
        english: "Use number/float/pointer/void",
        russian: "Используйте число/дробь/указатель/пустота",
    },
    Translation {
        english: "number",
        russian: "число",
    },
    Translation {
        english: "float",
        russian: "дробь",
    },
    Translation {
        english: "pointer",
        russian: "указатель",
    },
    Translation {
        english: "void",
        russian: "пустота",
    },
    Translation {
        english: "list",
        russian: "список",
    },
    Translation {
        english: "array",
        russian: "массив",
    },
    Translation {
        english: "dictionary",
        russian: "словарь",
    },
    Translation {
        english: "function",
        russian: "функция",
    },
    Translation {
        english: "object",
        russian: "объект",
    },
    Translation {
        english: "class",
        russian: "класс",
    },
    Translation {
        english: "module",
        russian: "модуль",
    },
    Translation {
        english: "resource",
        russian: "ресурс",
    },
    Translation {
        english: "unknown",
        russian: "неизвестно",
    },
];

#[cfg(test)]
mod tests {
    use super::{localize_message, DiagnosticLanguage};

    #[test]
    fn localizes_exact_messages_both_directions() {
        assert_eq!(
            localize_message("Division by zero", DiagnosticLanguage::Russian),
            "Деление на ноль"
        );
        assert_eq!(
            localize_message("Деление на ноль", DiagnosticLanguage::English),
            "Division by zero"
        );
    }

    #[test]
    fn localizes_usage_messages_both_directions() {
        assert_eq!(
            localize_message(
                "Использование: list.get(number)",
                DiagnosticLanguage::English
            ),
            "Usage: list.get(number)"
        );
        assert_eq!(
            localize_message("Usage: list.get(number)", DiagnosticLanguage::Russian),
            "Использование: список.get(число)"
        );
    }

    #[test]
    fn localizes_dynamic_runtime_and_parser_messages() {
        assert_eq!(
            localize_message(
                "Native library not found: ./libdemo.dll",
                DiagnosticLanguage::Russian
            ),
            "Native-библиотека не найдена: ./libdemo.dll"
        );
        assert_eq!(
            localize_message(
                "Неожиданный токен ')'; ожидалось: ident",
                DiagnosticLanguage::English
            ),
            "Unexpected token ')'; expected: ident"
        );
    }
}
