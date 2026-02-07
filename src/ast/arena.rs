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
    type_cache: HashMap<Symbol, TypeId>,
}

impl AstArena {
    pub fn new() -> Self {
        Self {
            expressions: Vec::new(),
            statements: Vec::new(),
            types: Vec::new(),
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

    pub fn register_custom_type(&mut self, interner: &SharedInterner, name: &str) -> TypeId {
        let symbol = self.intern_string(interner, name);
        if let Some(&id) = self.type_cache.get(&symbol) {
            return id;
        }

        let id = self.add_type(DataType::Object(symbol));
        self.type_cache.insert(symbol, id);
        id
    }

    pub fn get_expression(&self, id: ExprId) -> Option<&ExpressionNode> {
        self.expressions.get(id as usize)
    }

    pub fn get_statement(&self, id: StmtId) -> Option<&StatementNode> {
        self.statements.get(id as usize)
    }

    pub fn find_type_by_name(&self, interner: &SharedInterner, name: &str) -> Option<TypeId> {
        let symbol = interner.read(|i| i.get(name))?;

        self.type_cache.get(&symbol).copied()
    }

    pub fn intern_string(&self, interner: &SharedInterner, s: &str) -> Symbol {
        interner.write(|i| i.get_or_intern(s))
    }

    pub fn resolve_symbol(&self, interner: &SharedInterner, symbol: Symbol) -> Option<String> {
        interner.read(|i| i.resolve(symbol).map(|s| s.to_string()))
    }

    pub fn optimize_all(&mut self, interner: &SharedInterner) {
        for i in 0..self.expressions.len() {
            self.optimize_expression(i as ExprId, interner);
        }
    }

    fn optimize_expression(&mut self, id: ExprId, interner: &SharedInterner) {
        let node = &self.expressions[id as usize];

        if let ExpressionKind::Binary { op, left, right } = node.kind {
            let left_lit = self.get_expression(left).and_then(|e| e.kind.as_literal()).cloned();
            let right_lit = self.get_expression(right).and_then(|e| e.kind.as_literal()).cloned();

            if let (Some(l), Some(r)) = (left_lit, right_lit) {
                if let Some(folded) = self.fold_binary_constants(interner, op, &l, &r) {
                    self.expressions[id as usize].kind = ExpressionKind::Literal(folded);
                }
            }
        }
    }

    fn fold_binary_constants(
        &self,
        interner: &SharedInterner,
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
                _ => None,
            },
            (LiteralValue::Text(l_sym), LiteralValue::Text(r_sym)) if op == BinaryOperator::Add => {
                let l_str = self.resolve_symbol(interner, *l_sym)?;
                let r_str = self.resolve_symbol(interner, *r_sym)?;
                let combined = format!("{}{}", l_str, r_str);
                let new_sym = self.intern_string(interner, &combined);

                Some(LiteralValue::Text(new_sym))
            },
            (LiteralValue::Boolean(l), LiteralValue::Boolean(r)) => match op {
                BinaryOperator::And => Some(LiteralValue::Boolean(*l && *r)),
                BinaryOperator::Or => Some(LiteralValue::Boolean(*l || *r)),
                _ => None,
            },
            _ => None,
        }
    }

    pub fn init_builtin_types(&mut self, interner: &SharedInterner) {
        let builtins = [
            ("число", DataType::Primitive(PrimitiveType::Number)),
            ("строка", DataType::Primitive(PrimitiveType::Text)),
            ("логический", DataType::Primitive(PrimitiveType::Boolean)),
            ("дробь", DataType::Primitive(PrimitiveType::Float)),
            ("список", DataType::List(Box::new(DataType::Any))),
            ("массив", DataType::Array(Box::new(DataType::Any))),
            ("словарь", DataType::Dict { key: Box::new(DataType::Any), value: Box::new(DataType::Any) }),
            ("пустота", DataType::Unit),
            ("неизвестно", DataType::Any),
            ("функция", DataType::Function { params: vec![], return_type: Box::new(DataType::Any) }),
            
            ("Строка", DataType::Primitive(PrimitiveType::Text)),
            ("Список", DataType::List(Box::new(DataType::Any))),
            ("Массив", DataType::Array(Box::new(DataType::Any))),
            ("Словарь", DataType::Dict { key: Box::new(DataType::Any), value: Box::new(DataType::Any) }),
            ("Файл", DataType::Object(self.intern_string(interner, "Файл"))),
        ];

        for (name, dt) in builtins {
            let symbol = self.intern_string(interner, name);
            let id = self.add_type(dt);
            self.type_cache.insert(symbol, id);
        }
    }
}
