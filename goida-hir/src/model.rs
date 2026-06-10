use crate::ast::prelude::{
    BinaryOperator, ClassDefinition, DataType, ExprId, FunctionDefinition, ImportItem,
    LiteralValue, NativeLibraryDefinition, Span, StmtId, TryHandler, UnaryOperator,
};
use crate::{Binding, HirModule, MethodResolution};
use string_interner::DefaultSymbol as Symbol;

#[derive(Clone, Debug, Default)]
pub struct HirArena {
    expressions: Vec<Option<HirExpression>>,
    statements: Vec<Option<HirStatement>>,
}

impl HirArena {
    pub fn expression(&self, id: ExprId) -> Option<&HirExpression> {
        self.expressions.get(id as usize)?.as_ref()
    }

    pub fn statement(&self, id: StmtId) -> Option<&HirStatement> {
        self.statements.get(id as usize)?.as_ref()
    }

    pub fn expressions(&self) -> impl Iterator<Item = (ExprId, &HirExpression)> {
        self.expressions
            .iter()
            .enumerate()
            .filter_map(|(id, node)| Some((id as ExprId, node.as_ref()?)))
    }

    pub fn statements(&self) -> impl Iterator<Item = (StmtId, &HirStatement)> {
        self.statements
            .iter()
            .enumerate()
            .filter_map(|(id, node)| Some((id as StmtId, node.as_ref()?)))
    }

    pub(crate) fn reserve(&mut self, expression_count: usize, statement_count: usize) {
        self.expressions.resize(expression_count, None);
        self.statements.resize(statement_count, None);
    }

    pub(crate) fn insert_expression(&mut self, id: ExprId, node: HirExpression) {
        self.expressions[id as usize] = Some(node);
    }

    pub(crate) fn expression_mut(&mut self, id: ExprId) -> Option<&mut HirExpression> {
        self.expressions.get_mut(id as usize)?.as_mut()
    }

    pub(crate) fn insert_statement(&mut self, id: StmtId, node: HirStatement) {
        self.statements[id as usize] = Some(node);
    }
}

#[derive(Clone, Debug)]
pub struct HirExpression {
    pub kind: HirExpressionKind,
    pub span: Span,
    pub declared_type: Option<DataType>,
    pub inferred_type: DataType,
}

#[derive(Clone, Debug)]
pub struct HirCallArg {
    pub name: Option<Symbol>,
    pub value: ExprId,
}

#[derive(Clone, Debug)]
pub enum HirExpressionKind {
    Literal(LiteralValue),
    Identifier {
        name: Symbol,
        binding: Binding,
        is_module: bool,
    },
    Binary {
        op: BinaryOperator,
        left: ExprId,
        right: ExprId,
    },
    Unary {
        op: UnaryOperator,
        operand: ExprId,
    },
    FunctionCall {
        function: ExprId,
        args: Vec<HirCallArg>,
    },
    Index {
        object: ExprId,
        index: ExprId,
    },
    PropertyAccess {
        object: ExprId,
        property: Symbol,
    },
    MethodCall {
        object: ExprId,
        resolution: MethodResolution,
        args: Vec<HirCallArg>,
    },
    ObjectCreation {
        class_name: Symbol,
        args: Vec<HirCallArg>,
    },
    Lambda {
        params: Vec<crate::ast::prelude::Parameter>,
        body: StmtId,
    },
    This,
}

#[derive(Clone, Debug)]
pub struct HirStatement {
    pub kind: HirStatementKind,
    pub span: Span,
}

#[derive(Clone, Debug)]
pub enum HirStatementKind {
    Expression(ExprId),
    Import(ImportItem),
    Assign {
        name: Symbol,
        binding: Binding,
        is_const: bool,
        declared_type: Option<DataType>,
        value: ExprId,
    },
    CompoundAssign {
        target: ExprId,
        op: BinaryOperator,
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
        binding: Binding,
        init: ExprId,
        condition: ExprId,
        update: StmtId,
        body: StmtId,
    },
    ForEach {
        variable: Symbol,
        binding: Binding,
        iterable: ExprId,
        body: StmtId,
    },
    Thread {
        body: StmtId,
    },
    Try {
        body: StmtId,
        handlers: Vec<TryHandler>,
    },
    Raise {
        error_type: Symbol,
        message: Option<ExprId>,
    },
    Block(Vec<StmtId>),
    Return(Option<ExprId>),
    FunctionDefinition(FunctionDefinition),
    NativeLibraryDefinition(NativeLibraryDefinition),
    ClassDefinition(ClassDefinition),
    PropertyAssign {
        object: ExprId,
        property: Symbol,
        value: ExprId,
    },
    Empty,
}

pub trait HirVisitor {
    fn visit_statement(&mut self, module: &HirModule, id: StmtId) {
        walk_hir_statement(self, module, id);
    }

    fn visit_expression(&mut self, module: &HirModule, id: ExprId) {
        walk_hir_expression(self, module, id);
    }
}

pub fn walk_hir_statement<V: HirVisitor + ?Sized>(visitor: &mut V, module: &HirModule, id: StmtId) {
    let Some(node) = module.arena.statement(id) else {
        return;
    };
    match &node.kind {
        HirStatementKind::Expression(value) | HirStatementKind::Assign { value, .. } => {
            visitor.visit_expression(module, *value);
        }
        HirStatementKind::CompoundAssign { target, value, .. } => {
            visitor.visit_expression(module, *target);
            visitor.visit_expression(module, *value);
        }
        HirStatementKind::IndexAssign {
            object,
            index,
            value,
        } => {
            visitor.visit_expression(module, *object);
            visitor.visit_expression(module, *index);
            visitor.visit_expression(module, *value);
        }
        HirStatementKind::If {
            condition,
            then_body,
            else_body,
        } => {
            visitor.visit_expression(module, *condition);
            visitor.visit_statement(module, *then_body);
            if let Some(body) = else_body {
                visitor.visit_statement(module, *body);
            }
        }
        HirStatementKind::While { condition, body } => {
            visitor.visit_expression(module, *condition);
            visitor.visit_statement(module, *body);
        }
        HirStatementKind::For {
            init,
            condition,
            update,
            body,
            ..
        } => {
            visitor.visit_expression(module, *init);
            visitor.visit_expression(module, *condition);
            visitor.visit_statement(module, *update);
            visitor.visit_statement(module, *body);
        }
        HirStatementKind::ForEach { iterable, body, .. } => {
            visitor.visit_expression(module, *iterable);
            visitor.visit_statement(module, *body);
        }
        HirStatementKind::Thread { body } => visitor.visit_statement(module, *body),
        HirStatementKind::Try { body, handlers } => {
            visitor.visit_statement(module, *body);
            for handler in handlers {
                visitor.visit_statement(module, handler.body);
            }
        }
        HirStatementKind::Raise { message, .. } => {
            if let Some(message) = message {
                visitor.visit_expression(module, *message);
            }
        }
        HirStatementKind::Block(statements) => {
            for statement in statements {
                visitor.visit_statement(module, *statement);
            }
        }
        HirStatementKind::Return(value) => {
            if let Some(value) = value {
                visitor.visit_expression(module, *value);
            }
        }
        HirStatementKind::FunctionDefinition(function) => {
            for param in &function.params {
                if let Some(default) = param.default_value {
                    visitor.visit_expression(module, default);
                }
            }
            visitor.visit_statement(module, function.body);
        }
        HirStatementKind::ClassDefinition(class) => {
            for (_, _, field) in class.fields.values() {
                if let crate::ast::program::FieldData::Expression(Some(value)) = field {
                    visitor.visit_expression(module, *value);
                }
            }
        }
        HirStatementKind::PropertyAssign { object, value, .. } => {
            visitor.visit_expression(module, *object);
            visitor.visit_expression(module, *value);
        }
        HirStatementKind::Import(_)
        | HirStatementKind::NativeLibraryDefinition(_)
        | HirStatementKind::Empty => {}
    }
}

pub fn walk_hir_expression<V: HirVisitor + ?Sized>(
    visitor: &mut V,
    module: &HirModule,
    id: ExprId,
) {
    let Some(node) = module.arena.expression(id) else {
        return;
    };
    match &node.kind {
        HirExpressionKind::Binary { left, right, .. } => {
            visitor.visit_expression(module, *left);
            visitor.visit_expression(module, *right);
        }
        HirExpressionKind::Unary { operand, .. } => visitor.visit_expression(module, *operand),
        HirExpressionKind::FunctionCall { function, args } => {
            visitor.visit_expression(module, *function);
            for arg in args {
                visitor.visit_expression(module, arg.value);
            }
        }
        HirExpressionKind::Index { object, index } => {
            visitor.visit_expression(module, *object);
            visitor.visit_expression(module, *index);
        }
        HirExpressionKind::PropertyAccess { object, .. } => {
            visitor.visit_expression(module, *object)
        }
        HirExpressionKind::MethodCall { object, args, .. } => {
            visitor.visit_expression(module, *object);
            for arg in args {
                visitor.visit_expression(module, arg.value);
            }
        }
        HirExpressionKind::ObjectCreation { args, .. } => {
            for arg in args {
                visitor.visit_expression(module, arg.value);
            }
        }
        HirExpressionKind::Lambda { params, body } => {
            for param in params {
                if let Some(default) = param.default_value {
                    visitor.visit_expression(module, default);
                }
            }
            visitor.visit_statement(module, *body);
        }
        HirExpressionKind::Literal(_)
        | HirExpressionKind::Identifier { .. }
        | HirExpressionKind::This => {}
    }
}
