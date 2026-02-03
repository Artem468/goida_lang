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
    Assign,
}

impl BinaryOperator {
    pub fn precedence(self) -> u8 {
        match self {
            Self::Or => 2,
            Self::And => 3,
            Self::Eq | Self::Ne => 4,
            Self::Lt | Self::Le | Self::Gt | Self::Ge => 5,
            Self::Add | Self::Sub => 6,
            Self::Mul | Self::Div | Self::Mod => 7,
            Self::Assign => 1,
        }
    }

    pub fn is_left_associative(self) -> bool {
        matches!(
            self,
            Self::Add
                | Self::Sub
                | Self::Mul
                | Self::Div
                | Self::Mod
                | Self::Eq
                | Self::Ne
                | Self::Lt
                | Self::Le
                | Self::Gt
                | Self::Ge
                | Self::And
                | Self::Or
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum UnaryOperator {
    Negative,
    Not,
}
