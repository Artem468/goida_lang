#[derive(Debug, Clone, PartialEq)]
pub enum DataType {
    Число,
    Текст,
    Логический,
}

#[derive(Debug, Clone)]
pub enum Expression {
    Число(i64),
    Текст(String),
    Логический(bool),
    Идентификатор(String),
    БинарнаяОперация {
        левый: Box<Expression>,
        оператор: BinaryOperator,
        правый: Box<Expression>,
    },
    УнарнаяОперация {
        оператор: UnaryOperator,
        операнд: Box<Expression>,
    },
    ВызовФункции {
        имя: String,
        аргументы: Vec<Expression>,
    },
    ИндексДоступ {
        объект: Box<Expression>,
        индекс: Box<Expression>,
    },
}

#[derive(Debug, Clone)]
pub enum BinaryOperator {
    Плюс,
    Минус,
    Умножить,
    Разделить,
    Остаток,
    Равно,
    НеРавно,
    Больше,
    Меньше,
    БольшеРавно,
    МеньшеРавно,
    И,
    Или,
}

#[derive(Debug, Clone)]
pub enum UnaryOperator {
    Минус,
    Не,
}

#[derive(Debug, Clone)]
pub enum Statement {
    Объявление {
        имя: String,
        тип: Option<DataType>,
        значение: Option<Expression>,
    },
    Присваивание {
        имя: String,
        значение: Expression,
    },
    Если {
        условие: Expression,
        тело: Vec<Statement>,
        иначе: Option<Vec<Statement>>,
    },
    Пока {
        условие: Expression,
        тело: Vec<Statement>,
    },
    Для {
        переменная: String,
        начало: Expression,
        конец: Expression,
        тело: Vec<Statement>,
    },
    Возврат(Option<Expression>),
    Выражение(Expression),
    Печать(Expression),
    Блок(Vec<Statement>),
}

#[derive(Debug, Clone)]
pub struct Function {
    pub имя: String,
    pub параметры: Vec<Parameter>,
    pub возвращаемый_тип: Option<DataType>,
    pub тело: Vec<Statement>,
}

#[derive(Debug, Clone)]
pub struct Parameter {
    pub имя: String,
    pub тип: DataType,
}

#[derive(Debug, Clone)]
pub struct Program {
    pub функции: Vec<Function>,
    pub операторы: Vec<Statement>,
}
