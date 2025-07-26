#[derive(Debug, Clone, PartialEq)]
pub enum DataType {
    Number,
    Text,
    Boolean,
}

#[derive(Debug, Clone)]
pub enum Expression {
    Number(i64),
    Text(String),
    Boolean(bool),
    Identifier(String),
    BinaryOperation {
        left: Box<Expression>,
        operator: BinaryOperator,
        right: Box<Expression>,
    },
    UnaryOperation {
        operator: UnaryOperator,
        operand: Box<Expression>,
    },
    CallingFunction {
        name: String,
        arguments: Vec<Expression>,
    },
    AccessIndex {
        object: Box<Expression>,
        index: Box<Expression>,
    }
}

#[derive(Debug, Clone)]
pub enum BinaryOperator {
    Plus,
    Minus,
    Multiply,
    Divide,
    Remainder,
    Equal,
    Unequal,
    More,
    Less,
    MoreEqual,
    LessEqual,
    And,
    Or,
}

#[derive(Debug, Clone)]
pub enum UnaryOperator {
    Negative,
    Not,
}

#[derive(Debug, Clone)]
pub enum Statement {
    Declaration {
        name: String,
        type_of: Option<DataType>,
        value: Option<Expression>,
    },
    Assignment {
        name: String,
        value: Expression,
    },
    If {
        condition: Expression,
        body: Vec<Statement>,
        another: Option<Vec<Statement>>,
    },
    While {
        condition: Expression,
        body: Vec<Statement>,
    },
    For {
        variable: String,
        start: Expression,
        end: Expression,
        body: Vec<Statement>,
    },
    Return(Option<Expression>),
    Expression(Expression),
    Print(Expression),
    Block(Vec<Statement>),

}

#[derive(Debug, Clone)]
pub struct Function {
    pub name: String,
    pub parameters: Vec<Parameter>,
    pub return_type: Option<DataType>,
    pub body: Vec<Statement>,
    pub module: Option<String>
}

#[derive(Debug, Clone)]
pub struct Import {
    pub files: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct Parameter {
    pub name: String,
    pub type_of: DataType,
}

#[derive(Debug, Clone)]
pub struct Program {
    pub functions: Vec<Function>,
    pub operators: Vec<Statement>,
    pub imports: Vec<Import>
}
