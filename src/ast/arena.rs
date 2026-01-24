use std::collections::HashMap;
use string_interner::DefaultSymbol as Symbol;

use crate::ast::prelude::{
    BinaryOperator, DataType, ExprId, ExpressionKind, ExpressionNode, LiteralValue, PrimitiveType,
    Span, StatementKind, StatementNode, StmtId, TypeId,
};
use crate::interpreter::prelude::SharedInterner;

#[derive(Debug, Clone)]
pub struct AstArena {
    pub expressions: Vec<ExpressionNode>,
    pub statements: Vec<StatementNode>,
    pub types: Vec<DataType>,
    pub spans: Vec<Span>,
    type_cache: HashMap<Symbol, TypeId>,
}

impl AstArena {
    pub fn new() -> Self {
        Self {
            expressions: Vec::new(),
            statements: Vec::new(),
            types: Vec::new(),
            spans: Vec::new(),
            type_cache: HashMap::new(),
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

    pub fn resolve_or_intern_type(&mut self, interner: &SharedInterner, name: &str) -> TypeId {
        let mut lock = interner.write().expect("interner lock poisoned");
        let symbol = lock.get_or_intern(name);

        if let Some(&id) = self.type_cache.get(&symbol) {
            return id;
        }

        let new_type = match name {
            "число" => DataType::Primitive(PrimitiveType::Number),
            "логическое" => DataType::Primitive(PrimitiveType::Boolean),
            "текст" => DataType::Primitive(PrimitiveType::Text),
            "дробь" => DataType::Primitive(PrimitiveType::Float),
            _ => DataType::Object(symbol),
        };

        let id = self.add_type(new_type);
        self.type_cache.insert(symbol, id);
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

    pub fn intern_string(&self, interner: &SharedInterner, s: &str) -> Symbol {
        interner.write().expect("interner lock poisoned").get_or_intern(s)
    }

    pub fn resolve_symbol(&self, interner: &SharedInterner, symbol: Symbol) -> Option<String> {
        interner.read().expect("interner lock poisoned").resolve(symbol).map(|s| s.to_string())
    }

    pub fn optimize_constants(&mut self) {
        for i in 0..self.expressions.len() {
            if let ExpressionKind::Binary { op, left, right } = self.expressions[i].kind {
                let left_val = self.get_expression(left).and_then(|e| e.kind.as_literal());
                let right_val = self.get_expression(right).and_then(|e| e.kind.as_literal());

                if let (Some(l), Some(r)) = (left_val, right_val) {
                    if let Some(result) = self.fold_binary_constants(op, l, r) {
                        self.expressions[i].kind = ExpressionKind::Literal(result);
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
                BinaryOperator::Eq => Some(LiteralValue::Boolean(l == r)),
                BinaryOperator::Lt => Some(LiteralValue::Boolean(l < r)),
                _ => None,
            },
            (LiteralValue::Boolean(l), LiteralValue::Boolean(r)) => match op {
                BinaryOperator::And => Some(LiteralValue::Boolean(*l && *r)),
                BinaryOperator::Or => Some(LiteralValue::Boolean(*l || *r)),
                BinaryOperator::Eq => Some(LiteralValue::Boolean(l == r)),
                _ => None,
            },
            _ => None,
        }
    }
}
