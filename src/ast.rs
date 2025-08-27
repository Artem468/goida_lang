use string_interner::{DefaultSymbol as Symbol, StringInterner};


#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SourceLocation {
    pub line: u32,
    pub column: u32,
    pub offset: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Span {
    pub start: SourceLocation,
    pub end: SourceLocation,
}

impl Default for Span {
    fn default() -> Self {
        Self {
            start: SourceLocation {
                line: 0,
                column: 0,
                offset: 0,
            },
            end: SourceLocation {
                line: 0,
                column: 0,
                offset: 0,
            },
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum DataType {
    Primitive(PrimitiveType),
    List(Box<DataType>),
    Dict {
        key: Box<DataType>,
        value: Box<DataType>,
    },
    Function {
        params: Vec<DataType>,
        return_type: Box<DataType>,
    },
    Generic(Symbol),
    Unit,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PrimitiveType {
    Number,
    Float,
    Text,
    Boolean,
}

pub type ExprId = u32;
pub type StmtId = u32;
pub type TypeId = u32;

#[derive(Debug, Clone)]
pub struct AstArena {
    expressions: Vec<ExpressionNode>,
    statements: Vec<StatementNode>,
    types: Vec<DataType>,
    spans: Vec<Span>,
    pub interner: StringInterner,
}

#[derive(Debug, Clone)]
pub struct ExpressionNode {
    pub kind: ExpressionKind,
    pub span: Span,
    pub type_hint: Option<TypeId>,
}

#[derive(Debug, Clone)]
pub struct StatementNode {
    pub kind: StatementKind,
    pub span: Span,
}

impl AstArena {
    pub fn new() -> Self {
        Self {
            expressions: Vec::new(),
            statements: Vec::new(),
            types: Vec::new(),
            spans: Vec::new(),
            interner: StringInterner::default(),
        }
    }

    pub fn add_expression(&mut self, kind: ExpressionKind, span: Span) -> ExprId {
        let id = self.expressions.len() as ExprId;
        self.expressions.push(ExpressionNode {
            kind,
            span,
            type_hint: None,
        });
        id
    }

    pub fn add_statement(&mut self, kind: StatementKind, span: Span) -> StmtId {
        let id = self.statements.len() as StmtId;
        self.statements.push(StatementNode { kind, span });
        id
    }

    pub fn add_type(&mut self, data_type: DataType) -> TypeId {
        let id = self.types.len() as TypeId;
        self.types.push(data_type);
        id
    }

    pub fn get_expression(&self, id: ExprId) -> Option<&ExpressionNode> {
        self.expressions.get(id as usize)
    }

    pub fn get_statement(&self, id: StmtId) -> Option<&StatementNode> {
        self.statements.get(id as usize)
    }

    pub fn get_type(&self, id: TypeId) -> Option<&DataType> {
        self.types.get(id as usize)
    }

    pub fn intern_string(&mut self, s: &str) -> Symbol {
        self.interner.get_or_intern(s)
    }

    pub fn resolve_symbol(&self, symbol: Symbol) -> Option<&str> {
        self.interner.resolve(symbol)
    }
}

#[derive(Debug, Clone)]
pub enum ExpressionKind {
    Literal(LiteralValue),
    Identifier(Symbol),
    Binary {
        op: BinaryOperator,
        left: ExprId,
        right: ExprId,
    },
    Unary {
        op: UnaryOperator,
        operand: ExprId,
    },
    Call {
        function: ExprId,
        args: Vec<ExprId>,
    },
    Index {
        object: ExprId,
        index: ExprId,
    },
    Input(ExprId),
}

#[derive(Debug, Clone)]
pub enum LiteralValue {
    Number(i64),
    Float(f64),
    Text(Symbol),
    Boolean(bool),
    Unit,
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
            Self::Assign => 1,
            Self::Or => 2,
            Self::And => 3,
            Self::Eq | Self::Ne => 4,
            Self::Lt | Self::Le | Self::Gt | Self::Ge => 5,
            Self::Add | Self::Sub => 6,
            Self::Mul | Self::Div | Self::Mod => 7,
        }
    }

    pub fn is_left_associative(self) -> bool {
        !matches!(self, Self::Assign)
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum UnaryOperator {
    Negative,
    Not,
}

#[derive(Debug, Clone)]
pub enum StatementKind {
    Expression(ExprId),
    Let {
        name: Symbol,
        type_hint: Option<TypeId>,
        value: Option<ExprId>,
    },
    Assign {
        name: Symbol,
        value: ExprId,
    },
    IndexAssign {
        object: ExprId,
        index: ExprId,
        value: ExprId,
    },
    If {
        condition: ExprId,
        then_body: StmtId,
        else_body: Option<StmtId>,
    },
    While {
        condition: ExprId,
        body: StmtId,
    },
    For {
        variable: Symbol,
        start: ExprId,
        end: ExprId,
        body: StmtId,
    },
    Block(Vec<StmtId>),
    Return(Option<ExprId>),
    Print(ExprId),
    Input(ExprId),
}

#[derive(Debug, Clone)]
pub struct Function {
    pub name: Symbol,
    pub params: Vec<Parameter>,
    pub return_type: Option<TypeId>,
    pub body: StmtId,
    pub span: Span,
    pub module: Option<Symbol>,
}

#[derive(Debug, Clone)]
pub struct Parameter {
    pub name: Symbol,
    pub param_type: TypeId,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct Import {
    pub files: Vec<Symbol>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct Program {
    pub name: Symbol,
    pub functions: Vec<Function>,
    pub statements: Vec<StmtId>,
    pub imports: Vec<Import>,
    pub arena: AstArena,
}

impl Program {
    pub fn new(name: String) -> Self {
        let mut arena = AstArena::new();
        let name_symbol = arena.intern_string(&name);

        Self {
            name: name_symbol,
            functions: Vec::new(),
            statements: Vec::new(),
            imports: Vec::new(),
            arena,
        }
    }
}

pub trait AstVisitor<T> {
    fn visit_program(&mut self, program: &Program) -> T;
    fn visit_function(&mut self, function: &Function, arena: &AstArena) -> T;
    fn visit_statement(&mut self, stmt_id: StmtId, arena: &AstArena) -> T;
    fn visit_expression(&mut self, expr_id: ExprId, arena: &AstArena) -> T;
}

impl AstArena {
    pub fn optimize_constants(&mut self) {
        for i in 0..self.expressions.len() {
            if let ExpressionKind::Binary { op, left, right } = &self.expressions[i].kind {
                if let (Some(left_expr), Some(right_expr)) = (
                    self.expressions.get(*left as usize),
                    self.expressions.get(*right as usize),
                ) {
                    if let (ExpressionKind::Literal(l), ExpressionKind::Literal(r)) =
                        (&left_expr.kind, &right_expr.kind)
                    {
                        if let Some(result) = self.fold_binary_constants(*op, l, r) {
                            self.expressions[i].kind = ExpressionKind::Literal(result);
                        }
                    }
                }
            }
        }
    }

    fn fold_binary_constants(
        &self,
        op: BinaryOperator,
        left: &LiteralValue,
        right: &LiteralValue,
    ) -> Option<LiteralValue> {
        match (left, right) {
            (LiteralValue::Number(l), LiteralValue::Number(r)) => match op {
                BinaryOperator::Add => Some(LiteralValue::Number(l + r)),
                BinaryOperator::Sub => Some(LiteralValue::Number(l - r)),
                BinaryOperator::Mul => Some(LiteralValue::Number(l * r)),
                BinaryOperator::Div if *r != 0 => Some(LiteralValue::Number(l / r)),
                BinaryOperator::Mod if *r != 0 => Some(LiteralValue::Number(l % r)),
                BinaryOperator::Eq => Some(LiteralValue::Boolean(l == r)),
                BinaryOperator::Ne => Some(LiteralValue::Boolean(l != r)),
                BinaryOperator::Lt => Some(LiteralValue::Boolean(l < r)),
                BinaryOperator::Le => Some(LiteralValue::Boolean(l <= r)),
                BinaryOperator::Gt => Some(LiteralValue::Boolean(l > r)),
                BinaryOperator::Ge => Some(LiteralValue::Boolean(l >= r)),
                _ => None,
            },
            (LiteralValue::Boolean(l), LiteralValue::Boolean(r)) => match op {
                BinaryOperator::And => Some(LiteralValue::Boolean(*l && *r)),
                BinaryOperator::Or => Some(LiteralValue::Boolean(*l || *r)),
                BinaryOperator::Eq => Some(LiteralValue::Boolean(l == r)),
                BinaryOperator::Ne => Some(LiteralValue::Boolean(l != r)),
                _ => None,
            },
            _ => None,
        }
    }
}
