use std::fmt;
use string_interner::DefaultSymbol as Symbol;

pub type TypeId = u32;

#[derive(Debug, Clone, PartialEq)]
pub enum DataType {
    Primitive(PrimitiveType),
    List(Box<DataType>),
    Array(Box<DataType>),
    Dict {
        key: Box<DataType>,
        value: Box<DataType>,
    },
    Function {
        params: Vec<DataType>,
        return_type: Box<DataType>,
    },
    Object(Symbol),
    Any,
    Unit,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PrimitiveType {
    Number,
    Float,
    Text,
    Boolean,
}

impl fmt::Display for PrimitiveType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PrimitiveType::Number => write!(f, "число"),
            PrimitiveType::Boolean => write!(f, "логическое"),
            PrimitiveType::Text => write!(f, "строка"),
            PrimitiveType::Float => write!(f, "дробь"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BinaryOperator {
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
    And,
    Or,
}


#[derive(Debug, Clone, Copy, PartialEq)]
pub enum UnaryOperator {
    Negative,
    Not,
}
