use crate::parser::lexer::Token;
use std::ops::Range;

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Spanned<T> {
    pub node: T,
    pub span: Range<usize>,
}

impl<T> Spanned<T> {
    pub(crate) fn new(node: T, start: usize, end: usize) -> Self {
        Self {
            node,
            span: start..end,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Program {
    pub items: Vec<Item>,
}

pub(crate) type Item = Spanned<ItemKind>;
pub(crate) type Stmt = Spanned<StmtKind>;
pub(crate) type Expr = Spanned<ExprKind>;

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum ItemKind {
    Import(Import),
    Function(Function),
    Class(Class),
    Library(Library),
    MacroDefinition(MacroDefinition),
    Statement(Box<Stmt>),
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct MacroDefinition {
    pub name: String,
    pub rules: Vec<MacroRule>,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct MacroRule {
    pub matcher: Vec<MacroMatcher>,
    pub template: Vec<MacroTemplate>,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum MacroMatcher {
    Token(MacroToken),
    Fragment {
        name: String,
        kind: MacroFragmentKind,
    },
    Repeat {
        matcher: Vec<MacroMatcher>,
        separator: Vec<MacroToken>,
        op: MacroRepeatOp,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum MacroTemplate {
    Token(MacroToken),
    Variable(String),
    Delimited {
        delimiter: MacroDelimiter,
        template: Vec<MacroTemplate>,
        span: Range<usize>,
    },
    Repeat {
        template: Vec<MacroTemplate>,
        separator: Vec<MacroToken>,
        op: MacroRepeatOp,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum MacroFragmentKind {
    Expr,
    Ident,
    Block,
    Stmt,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum MacroRepeatOp {
    ZeroOrMore,
    OneOrMore,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct MacroToken {
    pub token: Token,
    pub span: Range<usize>,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct MacroCall {
    pub name: String,
    pub args: Vec<MacroToken>,
    pub delimiter: MacroDelimiter,
    pub span: Range<usize>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum MacroDelimiter {
    Paren,
    Bracket,
    Brace,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Import {
    pub path: String,
    pub alias: String,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Function {
    pub name: String,
    pub params: Vec<Param>,
    pub return_type: Option<String>,
    pub body: Vec<Item>,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Param {
    pub name: String,
    pub type_name: Option<String>,
    pub default_value: Option<Expr>,
    pub span: Range<usize>,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Class {
    pub name: String,
    pub base: Option<String>,
    pub items: Vec<ClassItem>,
}

pub(crate) type ClassItem = Spanned<ClassItemKind>;

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum ClassItemKind {
    Field(ClassField),
    Constructor(ClassMethod),
    Method(ClassMethod),
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ClassField {
    pub visibility: Option<Visibility>,
    pub is_static: bool,
    pub name: String,
    pub type_name: String,
    pub default_value: Option<Expr>,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ClassMethod {
    pub visibility: Option<Visibility>,
    pub is_static: bool,
    pub name: String,
    pub params: Vec<Param>,
    pub return_type: Option<String>,
    pub body: Vec<Item>,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Visibility {
    Public,
    Private,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Library {
    pub path: String,
    pub items: Vec<LibraryItem>,
}

pub(crate) type LibraryItem = Spanned<LibraryItemKind>;

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum LibraryItemKind {
    Function(LibraryFunction),
    Global(LibraryGlobal),
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct LibraryFunction {
    pub name: String,
    pub params: Vec<LibraryParam>,
    pub return_type: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct LibraryParam {
    pub name: String,
    pub type_name: String,
    pub span: Range<usize>,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct LibraryGlobal {
    pub name: String,
    pub type_name: String,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum StmtKind {
    Assign {
        name: String,
        is_const: bool,
        type_hint: Option<String>,
        value: Expr,
    },
    AssignTarget {
        target: Expr,
        value: Expr,
    },
    CompoundAssign {
        target: Expr,
        op: CompoundOp,
        value: Expr,
    },
    If {
        condition: Expr,
        then_body: Vec<Item>,
        else_body: Option<ElseBody>,
    },
    While {
        condition: Expr,
        body: Vec<Item>,
    },
    For {
        variable: String,
        init: Expr,
        condition: Expr,
        update: Box<ForUpdate>,
        body: Vec<Item>,
    },
    ForEach {
        variable: String,
        iterable: Expr,
        body: Vec<Item>,
    },
    Thread {
        body: Vec<Item>,
    },
    Try {
        body: Vec<Item>,
        handlers: Vec<Catch>,
    },
    Raise {
        error_type: String,
        message: Option<Expr>,
    },
    Return(Option<Expr>),
    Expr(Expr),
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum ElseBody {
    Block(Vec<Item>, Range<usize>),
    If(Box<Stmt>),
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Catch {
    pub pattern: Option<CatchPattern>,
    pub body: Vec<Item>,
    pub span: Range<usize>,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum CatchPattern {
    Text(String, Range<usize>),
    Type(String, Range<usize>),
    TypeAndText {
        type_name: String,
        type_span: Range<usize>,
        text_name: String,
        text_span: Range<usize>,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum ForUpdate {
    Assign {
        name: String,
        value: Expr,
        span: Range<usize>,
    },
    AssignTarget {
        target: Expr,
        value: Expr,
        span: Range<usize>,
    },
    Compound {
        target: Expr,
        op: CompoundOp,
        value: Expr,
        span: Range<usize>,
    },
    Expr(Expr),
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum CompoundOp {
    Add,
    Sub,
    Mul,
    Div,
    Mod,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum StmtExprTail {
    None,
    Assign(Expr),
    Compound(CompoundOp, Expr),
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum ExprKind {
    Number(i64),
    Float(f64),
    Text(String),
    Boolean(bool),
    Empty,
    Identifier(String),
    Binary {
        op: BinaryOp,
        left: Box<Expr>,
        right: Box<Expr>,
    },
    Unary {
        op: UnaryOp,
        operand: Box<Expr>,
    },
    FunctionCall {
        function: Box<Expr>,
        args: Vec<CallArg>,
    },
    MethodCall {
        object: Box<Expr>,
        method: String,
        args: Vec<CallArg>,
    },
    PropertyAccess {
        object: Box<Expr>,
        property: String,
    },
    Index {
        object: Box<Expr>,
        index: Box<Expr>,
    },
    ObjectCreation {
        class_name: String,
        args: Vec<CallArg>,
    },
    Lambda {
        params: Vec<Param>,
        body: LambdaBody,
    },
    MacroCall(MacroCall),
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum LambdaBody {
    Expr(Box<Expr>),
    Block(Vec<Item>, Range<usize>),
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum PostfixOp {
    FunctionCall(Vec<CallArg>, Range<usize>),
    MethodCall(String, Vec<CallArg>, Range<usize>),
    PropertyAccess(String, Range<usize>),
    Index(Expr, Range<usize>),
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct CallArg {
    pub name: Option<String>,
    pub value: Expr,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum BinaryOp {
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
pub(crate) enum UnaryOp {
    Negative,
    Not,
}

pub(crate) fn apply_postfix(mut expr: Expr, ops: Vec<PostfixOp>) -> Expr {
    for op in ops {
        let start = expr.span.start;
        expr = match op {
            PostfixOp::FunctionCall(args, span) => Spanned::new(
                ExprKind::FunctionCall {
                    function: Box::new(expr),
                    args,
                },
                start,
                span.end,
            ),
            PostfixOp::MethodCall(method, args, span) => Spanned::new(
                ExprKind::MethodCall {
                    object: Box::new(expr),
                    method,
                    args,
                },
                start,
                span.end,
            ),
            PostfixOp::PropertyAccess(property, span) => Spanned::new(
                ExprKind::PropertyAccess {
                    object: Box::new(expr),
                    property,
                },
                start,
                span.end,
            ),
            PostfixOp::Index(index, span) => Spanned::new(
                ExprKind::Index {
                    object: Box::new(expr),
                    index: Box::new(index),
                },
                start,
                span.end,
            ),
        };
    }
    expr
}
