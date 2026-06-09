use std::collections::HashMap;
use string_interner::DefaultSymbol as Symbol;

use crate::ast::prelude::{
    BinaryOperator, DataType, ExprId, ExpressionKind, ExpressionNode, LiteralValue, PrimitiveType,
    RuntimeType, Span, StatementKind, StatementNode, StmtId, TypeId,
};
use goida_model::SharedInterner;

#[derive(Clone, Copy, Debug)]
pub enum BuiltinTypeSpec {
    Number,
    Text,
    Boolean,
    Float,
    Pointer,
    List,
    Array,
    Dict,
    Unit,
    Any,
    Function,
    Object,
    Module,
    Resource,
    Class,
}

#[derive(Debug, Clone)]
/// Compact storage for AST nodes created during parsing.
///
/// Expressions, statements, and type descriptors are addressed by small integer
/// ids. This keeps nodes cheap to clone and lets parser/interpreter code pass
/// references to syntax without owning large recursive structures.
pub struct AstArena {
    /// Expression nodes indexed by [`ExprId`].
    pub expressions: Vec<ExpressionNode>,
    /// Statement nodes indexed by [`StmtId`].
    pub statements: Vec<StatementNode>,
    /// Type descriptors indexed by [`TypeId`].
    pub types: Vec<DataType>,
    type_cache: HashMap<Symbol, TypeId>,
}

impl AstArena {
    /// Creates an empty arena with no registered built-in types.
    pub fn new() -> Self {
        Self {
            expressions: Vec::new(),
            statements: Vec::new(),
            types: Vec::new(),
            type_cache: HashMap::new(),
        }
    }

    /// Stores an expression and returns its stable id.
    pub fn add_expression(&mut self, kind: ExpressionKind, span: Span) -> ExprId {
        let id = self.expressions.len() as ExprId;
        self.expressions.push(ExpressionNode {
            kind,
            span,
            type_hint: None,
        });
        id
    }

    /// Stores a statement and returns its stable id.
    pub fn add_statement(&mut self, kind: StatementKind, span: Span) -> StmtId {
        let id = self.statements.len() as StmtId;
        self.statements.push(StatementNode { kind, span });
        id
    }

    /// Stores a type descriptor and returns its stable id.
    pub fn add_type(&mut self, data_type: DataType) -> TypeId {
        let id = self.types.len() as TypeId;
        self.types.push(data_type);
        id
    }

    /// Registers or reuses a named object type.
    pub fn register_custom_type(&mut self, interner: &SharedInterner, name: &str) -> TypeId {
        let symbol = self.intern_string(interner, name);
        if let Some(&id) = self.type_cache.get(&symbol) {
            return id;
        }

        let id = self.add_type(DataType::Object(symbol));
        self.type_cache.insert(symbol, id);
        id
    }

    /// Returns an expression by id.
    pub fn get_expression(&self, id: ExprId) -> Option<&ExpressionNode> {
        self.expressions.get(id as usize)
    }

    /// Returns a statement by id.
    pub fn get_statement(&self, id: StmtId) -> Option<&StatementNode> {
        self.statements.get(id as usize)
    }

    /// Finds a registered type by its source-level name.
    pub fn find_type_by_name(&self, interner: &SharedInterner, name: &str) -> Option<TypeId> {
        let symbol = interner.read(|i| i.get(name))?;

        self.type_cache.get(&symbol).copied()
    }

    /// Interns a string in the shared module interner.
    pub fn intern_string(&self, interner: &SharedInterner, s: &str) -> Symbol {
        interner.write(|i| i.get_or_intern(s))
    }

    /// Resolves an interned symbol into an owned string.
    pub fn resolve_symbol(&self, interner: &SharedInterner, symbol: Symbol) -> Option<String> {
        interner.read(|i| i.resolve(symbol).map(|s| s.to_string()))
    }

    /// Applies cheap AST-level optimizations, currently constant folding.
    pub fn optimize_all(&mut self, interner: &SharedInterner) {
        for i in 0..self.expressions.len() {
            self.optimize_expression(i as ExprId, interner);
        }
    }

    fn optimize_expression(&mut self, id: ExprId, interner: &SharedInterner) {
        let node = &self.expressions[id as usize];

        if let ExpressionKind::Binary { op, left, right } = node.kind {
            let left_lit = self
                .get_expression(left)
                .and_then(|e| e.kind.as_literal())
                .cloned();
            let right_lit = self
                .get_expression(right)
                .and_then(|e| e.kind.as_literal())
                .cloned();

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
            }
            (LiteralValue::Boolean(l), LiteralValue::Boolean(r)) => match op {
                BinaryOperator::And => Some(LiteralValue::Boolean(*l && *r)),
                BinaryOperator::Or => Some(LiteralValue::Boolean(*l || *r)),
                _ => None,
            },
            _ => None,
        }
    }

    pub fn register_builtin_type(
        &mut self,
        interner: &SharedInterner,
        names: &[&str],
        spec: BuiltinTypeSpec,
        object_name: Option<&str>,
    ) {
        let dt = match spec {
            BuiltinTypeSpec::Number => DataType::Primitive(PrimitiveType::Number),
            BuiltinTypeSpec::Text => DataType::Primitive(PrimitiveType::Text),
            BuiltinTypeSpec::Boolean => DataType::Primitive(PrimitiveType::Boolean),
            BuiltinTypeSpec::Float => DataType::Primitive(PrimitiveType::Float),
            BuiltinTypeSpec::Pointer => DataType::Primitive(PrimitiveType::Pointer),
            BuiltinTypeSpec::List => DataType::List(Box::new(DataType::Any)),
            BuiltinTypeSpec::Array => DataType::Array(Box::new(DataType::Any)),
            BuiltinTypeSpec::Dict => DataType::Dict {
                key: Box::new(DataType::Any),
                value: Box::new(DataType::Any),
            },
            BuiltinTypeSpec::Unit => DataType::Unit,
            BuiltinTypeSpec::Any => DataType::Any,
            BuiltinTypeSpec::Function => DataType::Function {
                params: vec![],
                return_type: Box::new(DataType::Any),
            },
            BuiltinTypeSpec::Object => {
                DataType::Object(self.intern_string(interner, object_name.unwrap_or_default()))
            }
            BuiltinTypeSpec::Module => DataType::Runtime(RuntimeType::Module),
            BuiltinTypeSpec::Resource => DataType::Runtime(RuntimeType::Resource),
            BuiltinTypeSpec::Class => DataType::Runtime(RuntimeType::Class),
        };

        let id = self.add_type(dt);
        for name in names {
            let symbol = self.intern_string(interner, name);
            self.type_cache.insert(symbol, id);
        }
    }
}

impl Default for AstArena {
    fn default() -> Self {
        Self::new()
    }
}
