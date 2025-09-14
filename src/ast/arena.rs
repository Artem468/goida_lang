use string_interner::{DefaultSymbol as Symbol, StringInterner};
use crate::ast::prelude::{BinaryOperator, DataType, ExprId, ExpressionKind, ExpressionNode, LiteralValue, Span, StatementKind, StatementNode, StmtId, TypeId};

#[derive(Debug, Clone)]
pub struct AstArena {
    pub expressions: Vec<ExpressionNode>,
    pub statements: Vec<StatementNode>,
    pub types: Vec<DataType>,
    pub spans: Vec<Span>,
    pub interner: StringInterner,
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
