declare_builtin_registry! {
    functions {
        PRINT => ("print", ["печать", "print"], super::io::setup_io_func);
        INPUT => ("input", ["ввод", "input"], super::io::setup_io_func);
        TYPE => ("type", ["тип", "type"], super::common::setup_type_func);
        IS => ("is", ["является", "is"], super::common::setup_is_instance_func);
        NUMBER => ("number", ["число", "number"], super::number::setup_number_func);
        STRING => ("string", ["строка", "string"], super::text::setup_text_func);
        BOOLEAN => ("bool", ["логический", "bool"], super::bool::setup_bool_func);
        FLOAT => ("float", ["дробь", "float"], super::float::setup_float_func);
        LIST => ("list", ["список", "list"], super::list::setup_list_func);
        ARRAY => ("array", ["массив", "array"], super::array::setup_array_func);
        DICT => ("dict", ["словарь", "dict"], super::dict::setup_dict_func);
        ITERATOR => ("iterator", ["итератор", "iterator"], super::iterator::setup_iterator_func);
        FROM_JSON => ("from_json", ["из_json", "from_json"], super::json::setup_json_funcs);
        TO_JSON => ("to_json", ["в_json", "to_json"], super::json::setup_json_funcs);
        STRING_FROM_POINTER => ("string_from_pointer", ["строка_из_указателя", "string_from_pointer"], super::text::setup_text_func);
        REGEX => ("regex", ["регулярное_выражение", "regex"], super::regex::setup_regex_func);
    }
    classes {
        STRING => ("String", ["Строка", "String"], Text, super::text::setup_text_class);
        LIST => ("List", ["Список", "List"], List, super::list::setup_list_class);
        ARRAY => ("Array", ["Массив", "Array"], Array, super::array::setup_array_class);
        DICT => ("Dict", ["Словарь", "Dict"], Dict, super::dict::setup_dict_class);
        ITERATOR => ("Iterator", ["Итератор", "Iterator"], Object, super::iterator::setup_iterator_class);
        FILE => ("File", ["Файл", "File"], Object, super::file::setup_file_class);
        SYSTEM => ("System", ["Система", "System"], Object, super::system::setup_system_class);
        TERMINAL => ("Terminal", ["Терминал", "Terminal"], Object, super::terminal::setup_terminal_class);
        DATETIME => ("DateTime", ["ДатаВремя", "DateTime"], Object, super::datetime::setup_datetime_class);
        REGEX => ("Regex", ["РегулярноеВыражение", "Regex"], Object, super::regex::setup_regex_class);
        THREAD => ("Thread", ["Поток", "Thread"], Object, super::thread::setup_thread_class);
        MUTEX => ("Mutex", ["Мьютекс", "Mutex"], Object, super::thread::setup_mutex_class);
        RWLOCK => ("RwLock", ["БлокировкаЧтенияЗаписи", "RwLock"], Object, super::thread::setup_rwlock_class);
    }
    methods {
        LEN => ("length", ["длина", "length"]);
        JOIN => ("join", ["объединить", "join"]);
        GET => ("get", ["получить", "get"]);
        ITERATOR => ("iterator", ["итератор", "iterator"]);
        ADD => ("push", ["добавить", "push"]);
        SET => ("set", ["задать", "set"]);
        REMOVE => ("delete", ["удалить", "delete"]);
        CLEAR_TYPO => ("clear", ["очистить", "clear"]);
        HAS => ("has", ["имеет", "has"]);
        KEYS => ("keys", ["ключи", "keys"]);
        VALUES => ("values", ["значения", "values"]);
        MAP => ("map", ["преобразовать", "map"]);
        FILTER => ("filter", ["отфильтровать", "filter"]);
        REDUCE => ("reduce", ["свернуть", "reduce"]);
        TO_LIST => ("список", ["список", "list"]);
        SPLIT => ("split", ["разделить", "split"]);
        UPPER => ("upper", ["верхний", "upper"]);
        LOWER => ("lower", ["нижний", "lower"]);
        CONTAINS => ("contains", ["содержит", "contains"]);
        REPLACE => ("replace", ["заменить", "replace"]);
        REPLACE_ALL => ("replace_all", ["заменить_все", "replace_all"]);
        TRIM => ("trim", ["обрезать", "trim"]);
        STARTS_WITH => ("starts_with", ["начинается_с", "starts_with"]);
        ENDS_WITH => ("ends_with", ["заканчивается_на", "ends_with"]);
        PATTERN => ("pattern", ["шаблон", "pattern"]);
        MATCHES => ("matches", ["совпадает", "matches"]);
        FIND => ("find", ["найти", "find"]);
        FIND_ALL => ("find_all", ["найти_все", "find_all"]);
        GROUPS => ("groups", ["группы", "groups"]);
        GROUPS_ALL => ("groups_all", ["группы_все", "groups_all"]);
        EXISTS => ("exists", ["существует", "exists"]);
        READ => ("read", ["прочитать", "read"]);
        WRITE => ("write", ["записать", "write"]);
        APPEND => ("append", ["дописать", "append"]);
        EXIT => ("exit", ["выход", "exit"]);
        PANIC => ("panic", ["паника", "panic"]);
        PLATFORM => ("platform", ["платформа", "platform"]);
        ARGS => ("args", ["аргументы", "args"]);
        TIME => ("time", ["время", "time"]);
        SLEEP => ("sleep", ["сон", "sleep"]);
        BEEP => ("beep", ["сигнал", "beep"]);
        ENV => ("environment", ["окружение", "environment"]);
        CLEAR => ("clear", ["очистить", "clear"]);
        TITLE => ("title", ["заголовок", "title"]);
        HIDE_CURSOR => ("hide_cursor", ["скрыть_курсор", "hide_cursor"]);
        SHOW_CURSOR => ("show_cursor", ["показать_курсор", "show_cursor"]);
        POSITION => ("position", ["позиция", "position"]);
        PAUSE => ("pause", ["пауза", "pause"]);
        NOW => ("now", ["сейчас", "now"]);
        FORMAT => ("format", ["формат", "format"]);
        YEAR => ("year", ["год", "year"]);
        MONTH => ("month", ["месяц", "month"]);
        DAY => ("day", ["день", "day"]);
        HOUR => ("hour", ["час", "hour"]);
        MINUTE => ("minute", ["минута", "minute"]);
        SECOND => ("second", ["секунда", "second"]);
        ADD_SECONDS => ("add_seconds", ["добавить_секунд", "add_seconds"]);
        SUB_SECONDS => ("sub_seconds", ["вычесть_секунд", "sub_seconds"]);
        ADD_MINUTES => ("add_minutes", ["добавить_минут", "add_minutes"]);
        SUB_MINUTES => ("sub_minutes", ["вычесть_минут", "sub_minutes"]);
        ADD_HOURS => ("add_hours", ["добавить_часов", "add_hours"]);
        SUB_HOURS => ("sub_hours", ["вычесть_часов", "sub_hours"]);
        ADD_DAYS => ("add_days", ["добавить_дней", "add_days"]);
        SUB_DAYS => ("sub_days", ["вычесть_дней", "sub_days"]);
        ADD_MONTHS => ("add_months", ["добавить_месяцев", "add_months"]);
        SUB_MONTHS => ("sub_months", ["вычесть_месяцев", "sub_months"]);
        ADD_YEARS => ("add_years", ["добавить_лет", "add_years"]);
        SUB_YEARS => ("sub_years", ["вычесть_лет", "sub_years"]);
        CREATE => ("create", ["создать", "create"]);
        JOIN_THREAD => ("wait", ["ждать", "wait"]);
        LOCK => ("lock", ["блокировать", "lock"]);
        UNLOCK => ("unlock", ["разблокировать", "unlock"]);
        WRITE_LOCK => ("write_lock", ["писать_блокировать", "write_lock"]);
        WRITE_UNLOCK => ("write_unlock", ["писать_разблокировать", "write_unlock"]);
        READ_LOCK => ("read_lock", ["читать_блокировать", "read_lock"]);
        READ_UNLOCK => ("read_unlock", ["читать_разблокировать", "read_unlock"]);
    }
    macros {
        FORMAT => ("format", ["format", "формат"], super::macros::setup_macro_builtins);
    }
    types {
        (["число", "number"], Number);
        (["строка", "string"], Text);
        (["логический", "bool"], Boolean);
        (["дробь", "float"], Float);
        (["указатель", "pointer"], Pointer);
        (["список", "list"], List);
        (["массив", "array"], Array);
        (["словарь", "dict"], Dict);
        (["пустота", "void"], Unit);
        (["неизвестно", "any"], Any);
        (["функция", "function"], Function);
        (["модуль", "module"], Module);
        (["ресурс", "resource"], Resource);
        (["класс", "class"], Class);
    }
    errors {
        ERROR => ("Ошибка", None);
        VARIABLE_ERROR => ("ОшибкаПеременной", Some("Ошибка"));
        FUNCTION_ERROR => ("ОшибкаФункции", Some("Ошибка"));
        METHOD_ERROR => ("ОшибкаМетода", Some("Ошибка"));
        TYPE_ERROR => ("ОшибкаТипа", Some("Ошибка"));
        DIVISION_BY_ZERO_ERROR => ("ОшибкаДеленияНаНоль", Some("Ошибка"));
        OPERATION_ERROR => ("ОшибкаОперации", Some("Ошибка"));
        IO_ERROR => ("ОшибкаВводаВывода", Some("Ошибка"));
        IMPORT_ERROR => ("ОшибкаИмпорта", Some("Ошибка"));
        PANIC => ("Паника", Some("Ошибка"));
    }
}

pub(crate) const DATETIME_UNITS: &[DateTimeUnit] = &[
    DateTimeUnit { millis: 1_000, add: method::ADD_SECONDS, subtract: method::SUB_SECONDS },
    DateTimeUnit { millis: 60_000, add: method::ADD_MINUTES, subtract: method::SUB_MINUTES },
    DateTimeUnit { millis: 3_600_000, add: method::ADD_HOURS, subtract: method::SUB_HOURS },
    DateTimeUnit { millis: 86_400_000, add: method::ADD_DAYS, subtract: method::SUB_DAYS },
    DateTimeUnit { millis: 2_592_000_000, add: method::ADD_MONTHS, subtract: method::SUB_MONTHS },
    DateTimeUnit { millis: 31_536_000_000, add: method::ADD_YEARS, subtract: method::SUB_YEARS },
];
