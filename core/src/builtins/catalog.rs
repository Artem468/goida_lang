#[derive(Clone, Copy)]
pub struct BuiltinNames {
    pub canonical: &'static str,
    pub names: &'static [&'static str],
}

#[derive(Clone, Copy)]
pub enum BuiltinTypeSpec {
    Number,
    Text,
    Boolean,
    Float,
    Pointer,
    List,
    Array,
    Dict,
    Unit,
    Any,
    Function,
    Object,
    Module,
    Resource,
    Class,
}

#[derive(Clone, Copy)]
pub struct BuiltinTypeNames {
    pub names: &'static [&'static str],
    pub spec: BuiltinTypeSpec,
}

#[derive(Clone, Copy)]
pub struct DateTimeUnit {
    pub millis: i64,
    pub add: BuiltinNames,
    pub subtract: BuiltinNames,
}

pub(crate) mod function {
    use super::BuiltinNames;

    pub(crate) const PRINT: BuiltinNames = BuiltinNames {
        canonical: "print",
        names: &["печать", "print"],
    };
    pub(crate) const INPUT: BuiltinNames = BuiltinNames {
        canonical: "input",
        names: &["ввод", "input"],
    };
    pub(crate) const TYPE: BuiltinNames = BuiltinNames {
        canonical: "type",
        names: &["тип", "type"],
    };
    pub(crate) const IS: BuiltinNames = BuiltinNames {
        canonical: "is",
        names: &["является", "is"],
    };
    pub(crate) const NUMBER: BuiltinNames = BuiltinNames {
        canonical: "number",
        names: &["число", "number"],
    };
    pub(crate) const STRING: BuiltinNames = BuiltinNames {
        canonical: "string",
        names: &["строка", "string"],
    };
    pub(crate) const BOOLEAN: BuiltinNames = BuiltinNames {
        canonical: "bool",
        names: &["логический", "bool"],
    };
    pub(crate) const FLOAT: BuiltinNames = BuiltinNames {
        canonical: "float",
        names: &["дробь", "float"],
    };
    pub(crate) const LIST: BuiltinNames = BuiltinNames {
        canonical: "list",
        names: &["список", "list"],
    };
    pub(crate) const ARRAY: BuiltinNames = BuiltinNames {
        canonical: "array",
        names: &["массив", "array"],
    };
    pub(crate) const DICT: BuiltinNames = BuiltinNames {
        canonical: "dict",
        names: &["словарь", "dict"],
    };
    pub(crate) const ITERATOR: BuiltinNames = BuiltinNames {
        canonical: "iterator",
        names: &["итератор", "iterator"],
    };
    pub(crate) const FROM_JSON: BuiltinNames = BuiltinNames {
        canonical: "from_json",
        names: &["из_json", "from_json"],
    };
    pub(crate) const TO_JSON: BuiltinNames = BuiltinNames {
        canonical: "to_json",
        names: &["в_json", "to_json"],
    };
    pub(crate) const STRING_FROM_POINTER: BuiltinNames = BuiltinNames {
        canonical: "string_from_pointer",
        names: &["строка_из_указателя", "string_from_pointer"],
    };
    pub(crate) const REGEX: BuiltinNames = BuiltinNames {
        canonical: "regex",
        names: &["регулярное_выражение", "regex"],
    };
}

pub(crate) mod class {
    use super::{BuiltinNames, BuiltinTypeSpec};

    #[derive(Clone, Copy)]
    pub struct BuiltinClass {
        pub names: BuiltinNames,
        pub type_spec: BuiltinTypeSpec,
    }

    pub(crate) const STRING: BuiltinClass = BuiltinClass {
        names: BuiltinNames {
            canonical: "String",
            names: &["Строка", "String"],
        },
        type_spec: BuiltinTypeSpec::Text,
    };
    pub(crate) const LIST: BuiltinClass = BuiltinClass {
        names: BuiltinNames {
            canonical: "List",
            names: &["Список", "List"],
        },
        type_spec: BuiltinTypeSpec::List,
    };
    pub(crate) const ARRAY: BuiltinClass = BuiltinClass {
        names: BuiltinNames {
            canonical: "Array",
            names: &["Массив", "Array"],
        },
        type_spec: BuiltinTypeSpec::Array,
    };
    pub(crate) const DICT: BuiltinClass = BuiltinClass {
        names: BuiltinNames {
            canonical: "Dict",
            names: &["Словарь", "Dict"],
        },
        type_spec: BuiltinTypeSpec::Dict,
    };
    pub(crate) const ITERATOR: BuiltinClass = BuiltinClass {
        names: BuiltinNames {
            canonical: "Iterator",
            names: &["Итератор", "Iterator"],
        },
        type_spec: BuiltinTypeSpec::Object,
    };
    pub(crate) const FILE: BuiltinClass = BuiltinClass {
        names: BuiltinNames {
            canonical: "File",
            names: &["Файл", "File"],
        },
        type_spec: BuiltinTypeSpec::Object,
    };
    pub(crate) const SYSTEM: BuiltinClass = BuiltinClass {
        names: BuiltinNames {
            canonical: "System",
            names: &["Система", "System"],
        },
        type_spec: BuiltinTypeSpec::Object,
    };
    pub(crate) const TERMINAL: BuiltinClass = BuiltinClass {
        names: BuiltinNames {
            canonical: "Terminal",
            names: &["Терминал", "Terminal"],
        },
        type_spec: BuiltinTypeSpec::Object,
    };
    pub(crate) const DATETIME: BuiltinClass = BuiltinClass {
        names: BuiltinNames {
            canonical: "DateTime",
            names: &["ДатаВремя", "DateTime"],
        },
        type_spec: BuiltinTypeSpec::Object,
    };
    pub(crate) const REGEX: BuiltinClass = BuiltinClass {
        names: BuiltinNames {
            canonical: "Regex",
            names: &["РегулярноеВыражение", "Regex"],
        },
        type_spec: BuiltinTypeSpec::Object,
    };
    pub(crate) const THREAD: BuiltinClass = BuiltinClass {
        names: BuiltinNames {
            canonical: "Thread",
            names: &["Поток", "Thread"],
        },
        type_spec: BuiltinTypeSpec::Object,
    };
    pub(crate) const MUTEX: BuiltinClass = BuiltinClass {
        names: BuiltinNames {
            canonical: "Mutex",
            names: &["Мьютекс", "Mutex"],
        },
        type_spec: BuiltinTypeSpec::Object,
    };
    pub(crate) const RWLOCK: BuiltinClass = BuiltinClass {
        names: BuiltinNames {
            canonical: "RwLock",
            names: &["БлокировкаЧтенияЗаписи", "RwLock"],
        },
        type_spec: BuiltinTypeSpec::Object,
    };
}

pub(crate) mod method {
    use super::BuiltinNames;

    pub(crate) const LEN: BuiltinNames = BuiltinNames {
        canonical: "длина",
        names: &["длина", "length"],
    };
    pub(crate) const JOIN: BuiltinNames = BuiltinNames {
        canonical: "join",
        names: &["объединить", "join"],
    };
    pub(crate) const GET: BuiltinNames = BuiltinNames {
        canonical: "get",
        names: &["получить", "get"],
    };
    pub(crate) const ITERATOR: BuiltinNames = BuiltinNames {
        canonical: "iterator",
        names: &["итератор", "iterator"],
    };
    pub(crate) const ADD: BuiltinNames = BuiltinNames {
        canonical: "push",
        names: &["добавить", "push"],
    };
    pub(crate) const SET: BuiltinNames = BuiltinNames {
        canonical: "set",
        names: &["задать", "set"],
    };
    pub(crate) const REMOVE: BuiltinNames = BuiltinNames {
        canonical: "delete",
        names: &["удалить", "delete"],
    };
    pub(crate) const CLEAR_TYPO: BuiltinNames = BuiltinNames {
        canonical: "clear",
        names: &["очистить", "clear"],
    };
    pub(crate) const HAS: BuiltinNames = BuiltinNames {
        canonical: "has",
        names: &["имеет", "has"],
    };
    pub(crate) const KEYS: BuiltinNames = BuiltinNames {
        canonical: "keys",
        names: &["ключи", "keys"],
    };
    pub(crate) const VALUES: BuiltinNames = BuiltinNames {
        canonical: "values",
        names: &["значения", "values"],
    };
    pub(crate) const MAP: BuiltinNames = BuiltinNames {
        canonical: "map",
        names: &["преобразовать", "map"],
    };
    pub(crate) const FILTER: BuiltinNames = BuiltinNames {
        canonical: "filter",
        names: &["отфильтровать", "filter"],
    };
    pub(crate) const REDUCE: BuiltinNames = BuiltinNames {
        canonical: "reduce",
        names: &["свернуть", "reduce"],
    };
    pub(crate) const TO_LIST: BuiltinNames = BuiltinNames {
        canonical: "список",
        names: &["список", "list"],
    };
    pub(crate) const SPLIT: BuiltinNames = BuiltinNames {
        canonical: "split",
        names: &["разделить", "split"],
    };
    pub(crate) const UPPER: BuiltinNames = BuiltinNames {
        canonical: "upper",
        names: &["верхний", "upper"],
    };
    pub(crate) const LOWER: BuiltinNames = BuiltinNames {
        canonical: "lower",
        names: &["нижний", "lower"],
    };
    pub(crate) const CONTAINS: BuiltinNames = BuiltinNames {
        canonical: "contains",
        names: &["содержит", "contains"],
    };
    pub(crate) const REPLACE: BuiltinNames = BuiltinNames {
        canonical: "replace",
        names: &["заменить", "replace"],
    };
    pub(crate) const REPLACE_ALL: BuiltinNames = BuiltinNames {
        canonical: "replace_all",
        names: &["заменить_все", "replace_all"],
    };
    pub(crate) const TRIM: BuiltinNames = BuiltinNames {
        canonical: "trim",
        names: &["обрезать", "trim"],
    };
    pub(crate) const STARTS_WITH: BuiltinNames = BuiltinNames {
        canonical: "starts_with",
        names: &["начинается_с", "starts_with"],
    };
    pub(crate) const ENDS_WITH: BuiltinNames = BuiltinNames {
        canonical: "ends_with",
        names: &["заканчивается_на", "ends_with"],
    };
    pub(crate) const PATTERN: BuiltinNames = BuiltinNames {
        canonical: "pattern",
        names: &["шаблон", "pattern"],
    };
    pub(crate) const MATCHES: BuiltinNames = BuiltinNames {
        canonical: "matches",
        names: &["совпадает", "matches"],
    };
    pub(crate) const FIND: BuiltinNames = BuiltinNames {
        canonical: "find",
        names: &["найти", "find"],
    };
    pub(crate) const FIND_ALL: BuiltinNames = BuiltinNames {
        canonical: "find_all",
        names: &["найти_все", "find_all"],
    };
    pub(crate) const GROUPS: BuiltinNames = BuiltinNames {
        canonical: "groups",
        names: &["группы", "groups"],
    };
    pub(crate) const GROUPS_ALL: BuiltinNames = BuiltinNames {
        canonical: "groups_all",
        names: &["группы_все", "groups_all"],
    };
    pub(crate) const EXISTS: BuiltinNames = BuiltinNames {
        canonical: "exists",
        names: &["существует", "exists"],
    };
    pub(crate) const READ: BuiltinNames = BuiltinNames {
        canonical: "read",
        names: &["прочитать", "read"],
    };
    pub(crate) const WRITE: BuiltinNames = BuiltinNames {
        canonical: "write",
        names: &["записать", "write"],
    };
    pub(crate) const APPEND: BuiltinNames = BuiltinNames {
        canonical: "append",
        names: &["дописать", "append"],
    };
    pub(crate) const EXIT: BuiltinNames = BuiltinNames {
        canonical: "exit",
        names: &["выход", "exit"],
    };
    pub(crate) const PANIC: BuiltinNames = BuiltinNames {
        canonical: "panic",
        names: &["паника", "panic"],
    };
    pub(crate) const PLATFORM: BuiltinNames = BuiltinNames {
        canonical: "platform",
        names: &["платформа", "platform"],
    };
    pub(crate) const ARGS: BuiltinNames = BuiltinNames {
        canonical: "args",
        names: &["аргументы", "args"],
    };
    pub(crate) const TIME: BuiltinNames = BuiltinNames {
        canonical: "time",
        names: &["время", "time"],
    };
    pub(crate) const SLEEP: BuiltinNames = BuiltinNames {
        canonical: "sleep",
        names: &["сон", "sleep"],
    };
    pub(crate) const BEEP: BuiltinNames = BuiltinNames {
        canonical: "beep",
        names: &["сигнал", "beep"],
    };
    pub(crate) const ENV: BuiltinNames = BuiltinNames {
        canonical: "environment",
        names: &["окружение", "environment"],
    };
    pub(crate) const CLEAR: BuiltinNames = BuiltinNames {
        canonical: "clear",
        names: &["очистить", "clear"],
    };
    pub(crate) const TITLE: BuiltinNames = BuiltinNames {
        canonical: "title",
        names: &["заголовок", "title"],
    };
    pub(crate) const HIDE_CURSOR: BuiltinNames = BuiltinNames {
        canonical: "hide_cursor",
        names: &["скрыть_курсор", "hide_cursor"],
    };
    pub(crate) const SHOW_CURSOR: BuiltinNames = BuiltinNames {
        canonical: "show_cursor",
        names: &["показать_курсор", "show_cursor"],
    };
    pub(crate) const POSITION: BuiltinNames = BuiltinNames {
        canonical: "position",
        names: &["позиция", "position"],
    };
    pub(crate) const PAUSE: BuiltinNames = BuiltinNames {
        canonical: "pause",
        names: &["пауза", "pause"],
    };
    pub(crate) const NOW: BuiltinNames = BuiltinNames {
        canonical: "now",
        names: &["сейчас", "now"],
    };
    pub(crate) const FORMAT: BuiltinNames = BuiltinNames {
        canonical: "format",
        names: &["формат", "format"],
    };
    pub(crate) const YEAR: BuiltinNames = BuiltinNames {
        canonical: "year",
        names: &["год", "year"],
    };
    pub(crate) const MONTH: BuiltinNames = BuiltinNames {
        canonical: "month",
        names: &["месяц", "month"],
    };
    pub(crate) const DAY: BuiltinNames = BuiltinNames {
        canonical: "day",
        names: &["день", "day"],
    };
    pub(crate) const HOUR: BuiltinNames = BuiltinNames {
        canonical: "hour",
        names: &["час", "hour"],
    };
    pub(crate) const MINUTE: BuiltinNames = BuiltinNames {
        canonical: "minute",
        names: &["минута", "minute"],
    };
    pub(crate) const SECOND: BuiltinNames = BuiltinNames {
        canonical: "second",
        names: &["секунда", "second"],
    };
    pub(crate) const ADD_SECONDS: BuiltinNames = BuiltinNames {
        canonical: "add_seconds",
        names: &["добавить_секунд", "add_seconds"],
    };
    pub(crate) const SUB_SECONDS: BuiltinNames = BuiltinNames {
        canonical: "sub_seconds",
        names: &["вычесть_секунд", "sub_seconds"],
    };
    pub(crate) const ADD_MINUTES: BuiltinNames = BuiltinNames {
        canonical: "add_minutes",
        names: &["добавить_минут", "add_minutes"],
    };
    pub(crate) const SUB_MINUTES: BuiltinNames = BuiltinNames {
        canonical: "sub_minutes",
        names: &["вычесть_минут", "sub_minutes"],
    };
    pub(crate) const ADD_HOURS: BuiltinNames = BuiltinNames {
        canonical: "add_hours",
        names: &["добавить_часов", "add_hours"],
    };
    pub(crate) const SUB_HOURS: BuiltinNames = BuiltinNames {
        canonical: "sub_hours",
        names: &["вычесть_часов", "sub_hours"],
    };
    pub(crate) const ADD_DAYS: BuiltinNames = BuiltinNames {
        canonical: "add_days",
        names: &["добавить_дней", "add_days"],
    };
    pub(crate) const SUB_DAYS: BuiltinNames = BuiltinNames {
        canonical: "sub_days",
        names: &["вычесть_дней", "sub_days"],
    };
    pub(crate) const ADD_MONTHS: BuiltinNames = BuiltinNames {
        canonical: "add_months",
        names: &["добавить_месяцев", "add_months"],
    };
    pub(crate) const SUB_MONTHS: BuiltinNames = BuiltinNames {
        canonical: "sub_months",
        names: &["вычесть_месяцев", "sub_months"],
    };
    pub(crate) const ADD_YEARS: BuiltinNames = BuiltinNames {
        canonical: "add_years",
        names: &["добавить_лет", "add_years"],
    };
    pub(crate) const SUB_YEARS: BuiltinNames = BuiltinNames {
        canonical: "sub_years",
        names: &["вычесть_лет", "sub_years"],
    };
    pub(crate) const CREATE: BuiltinNames = BuiltinNames {
        canonical: "create",
        names: &["создать", "create"],
    };
    pub(crate) const JOIN_THREAD: BuiltinNames = BuiltinNames {
        canonical: "wait",
        names: &["ждать", "wait"],
    };
    pub(crate) const LOCK: BuiltinNames = BuiltinNames {
        canonical: "lock",
        names: &["блокировать", "lock"],
    };
    pub(crate) const UNLOCK: BuiltinNames = BuiltinNames {
        canonical: "unlock",
        names: &["разблокировать", "unlock"],
    };
    pub(crate) const WRITE_LOCK: BuiltinNames = BuiltinNames {
        canonical: "write_lock",
        names: &["писать_блокировать", "write_lock"],
    };
    pub(crate) const WRITE_UNLOCK: BuiltinNames = BuiltinNames {
        canonical: "write_unlock",
        names: &["писать_разблокировать", "write_unlock"],
    };
    pub(crate) const READ_LOCK: BuiltinNames = BuiltinNames {
        canonical: "read_lock",
        names: &["читать_блокировать", "read_lock"],
    };
    pub(crate) const READ_UNLOCK: BuiltinNames = BuiltinNames {
        canonical: "read_unlock",
        names: &["читать_разблокировать", "read_unlock"],
    };
}

pub(crate) mod macros {
    use super::BuiltinNames;

    pub(crate) const FORMAT: BuiltinNames = BuiltinNames {
        canonical: "format",
        names: &["format", "формат"],
    };
}

pub const FUNCTIONS: &[BuiltinNames] = &[
    function::PRINT,
    function::INPUT,
    function::TYPE,
    function::IS,
    function::NUMBER,
    function::STRING,
    function::BOOLEAN,
    function::FLOAT,
    function::LIST,
    function::ARRAY,
    function::DICT,
    function::ITERATOR,
    function::FROM_JSON,
    function::TO_JSON,
    function::STRING_FROM_POINTER,
    function::REGEX,
];

pub const CLASSES: &[class::BuiltinClass] = &[
    class::STRING,
    class::LIST,
    class::ARRAY,
    class::DICT,
    class::ITERATOR,
    class::FILE,
    class::SYSTEM,
    class::TERMINAL,
    class::DATETIME,
    class::REGEX,
    class::THREAD,
    class::MUTEX,
    class::RWLOCK,
];

pub const METHODS: &[BuiltinNames] = &[
    method::LEN,
    method::JOIN,
    method::GET,
    method::ITERATOR,
    method::ADD,
    method::SET,
    method::REMOVE,
    method::CLEAR_TYPO,
    method::HAS,
    method::KEYS,
    method::VALUES,
    method::MAP,
    method::FILTER,
    method::REDUCE,
    method::TO_LIST,
    method::SPLIT,
    method::UPPER,
    method::LOWER,
    method::CONTAINS,
    method::REPLACE,
    method::REPLACE_ALL,
    method::TRIM,
    method::STARTS_WITH,
    method::ENDS_WITH,
    method::PATTERN,
    method::MATCHES,
    method::FIND,
    method::FIND_ALL,
    method::GROUPS,
    method::GROUPS_ALL,
    method::EXISTS,
    method::READ,
    method::WRITE,
    method::APPEND,
    method::EXIT,
    method::PANIC,
    method::PLATFORM,
    method::ARGS,
    method::TIME,
    method::SLEEP,
    method::BEEP,
    method::ENV,
    method::CLEAR,
    method::TITLE,
    method::HIDE_CURSOR,
    method::SHOW_CURSOR,
    method::POSITION,
    method::PAUSE,
    method::NOW,
    method::FORMAT,
    method::YEAR,
    method::MONTH,
    method::DAY,
    method::HOUR,
    method::MINUTE,
    method::SECOND,
    method::ADD_SECONDS,
    method::SUB_SECONDS,
    method::ADD_MINUTES,
    method::SUB_MINUTES,
    method::ADD_HOURS,
    method::SUB_HOURS,
    method::ADD_DAYS,
    method::SUB_DAYS,
    method::ADD_MONTHS,
    method::SUB_MONTHS,
    method::ADD_YEARS,
    method::SUB_YEARS,
    method::CREATE,
    method::JOIN_THREAD,
    method::LOCK,
    method::UNLOCK,
    method::WRITE_LOCK,
    method::WRITE_UNLOCK,
    method::READ_LOCK,
    method::READ_UNLOCK,
];

pub const DATETIME_UNITS: &[DateTimeUnit] = &[
    DateTimeUnit {
        millis: 1_000,
        add: method::ADD_SECONDS,
        subtract: method::SUB_SECONDS,
    },
    DateTimeUnit {
        millis: 60_000,
        add: method::ADD_MINUTES,
        subtract: method::SUB_MINUTES,
    },
    DateTimeUnit {
        millis: 3_600_000,
        add: method::ADD_HOURS,
        subtract: method::SUB_HOURS,
    },
    DateTimeUnit {
        millis: 86_400_000,
        add: method::ADD_DAYS,
        subtract: method::SUB_DAYS,
    },
    DateTimeUnit {
        millis: 2_592_000_000,
        add: method::ADD_MONTHS,
        subtract: method::SUB_MONTHS,
    },
    DateTimeUnit {
        millis: 31_536_000_000,
        add: method::ADD_YEARS,
        subtract: method::SUB_YEARS,
    },
];

pub const MACROS: &[BuiltinNames] = &[macros::FORMAT];

pub const TYPES: &[BuiltinTypeNames] = &[
    BuiltinTypeNames {
        names: &["число", "number"],
        spec: BuiltinTypeSpec::Number,
    },
    BuiltinTypeNames {
        names: &["строка", "string"],
        spec: BuiltinTypeSpec::Text,
    },
    BuiltinTypeNames {
        names: &["логический", "bool"],
        spec: BuiltinTypeSpec::Boolean,
    },
    BuiltinTypeNames {
        names: &["дробь", "float"],
        spec: BuiltinTypeSpec::Float,
    },
    BuiltinTypeNames {
        names: &["указатель", "pointer"],
        spec: BuiltinTypeSpec::Pointer,
    },
    BuiltinTypeNames {
        names: &["список", "list"],
        spec: BuiltinTypeSpec::List,
    },
    BuiltinTypeNames {
        names: &["массив", "array"],
        spec: BuiltinTypeSpec::Array,
    },
    BuiltinTypeNames {
        names: &["словарь", "dict"],
        spec: BuiltinTypeSpec::Dict,
    },
    BuiltinTypeNames {
        names: &["пустота", "void"],
        spec: BuiltinTypeSpec::Unit,
    },
    BuiltinTypeNames {
        names: &["неизвестно", "any"],
        spec: BuiltinTypeSpec::Any,
    },
    BuiltinTypeNames {
        names: &["функция", "function"],
        spec: BuiltinTypeSpec::Function,
    },
    BuiltinTypeNames {
        names: &["модуль", "module"],
        spec: BuiltinTypeSpec::Module,
    },
    BuiltinTypeNames {
        names: &["ресурс", "resource"],
        spec: BuiltinTypeSpec::Resource,
    },
    BuiltinTypeNames {
        names: &["класс", "class"],
        spec: BuiltinTypeSpec::Class,
    },
];

pub fn names_for(entries: &[BuiltinNames], canonical: &str) -> &'static [&'static str] {
    entries
        .iter()
        .find(|entry| entry.canonical == canonical)
        .map(|entry| entry.names)
        .unwrap_or(&[])
}

pub fn function_names(canonical: &str) -> &'static [&'static str] {
    names_for(FUNCTIONS, canonical)
}

pub fn method_names(canonical: &str) -> &'static [&'static str] {
    names_for(METHODS, canonical)
}

pub fn macro_names(canonical: &str) -> &'static [&'static str] {
    names_for(MACROS, canonical)
}

pub fn class_names(canonical: &str) -> &'static [&'static str] {
    CLASSES
        .iter()
        .find(|entry| entry.names.canonical == canonical)
        .map(|entry| entry.names.names)
        .unwrap_or(&[])
}

pub fn known_global_names() -> impl Iterator<Item = &'static str> {
    FUNCTIONS
        .iter()
        .flat_map(|entry| entry.names.iter().copied())
        .chain(
            CLASSES
                .iter()
                .flat_map(|entry| entry.names.names.iter().copied()),
        )
}
