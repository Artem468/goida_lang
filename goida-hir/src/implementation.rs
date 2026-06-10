use crate::ast::prelude::{
    AstArena, CallArg, DataType, ExprId, ExpressionKind, FunctionDefinition, Parameter, Span,
    StatementKind, StmtId, TypeId,
};
use crate::{
    HirArena, HirCallArg, HirExpression, HirExpressionKind, HirStatement, HirStatementKind,
};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use string_interner::DefaultSymbol as Symbol;

pub trait HirSource {
    fn arena(&self) -> &AstArena;
    fn body(&self) -> &[StmtId];
    fn global_names(&self) -> Vec<Symbol>;
    fn functions(&self) -> Vec<Arc<FunctionDefinition>>;
    fn functions_to_type_check(&self) -> Vec<Arc<FunctionDefinition>>;
    fn class_names(&self) -> Vec<Symbol>;
    fn is_module_name(&self, name: Symbol) -> bool;
    fn callable_signatures(&self) -> Vec<CallableSignature>;
}

#[derive(Clone, Debug)]
pub struct CallableSignature {
    pub name: Symbol,
    pub params: Vec<Parameter>,
    pub return_type: Option<TypeId>,
    pub span: Span,
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum Binding {
    LocalSlot(u32),
    GlobalSlot(u32),
    UpvalueSlot(u32),
    Dynamic(Symbol),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MethodResolution {
    Static(Symbol),
    Dynamic(Symbol),
}

#[derive(Clone, Debug, Default)]
pub struct HirModule {
    pub arena: HirArena,
    pub body: Vec<StmtId>,
    pub functions: Vec<Arc<FunctionDefinition>>,
    pub callable_signatures: Vec<CallableSignature>,
    pub type_definitions: Vec<DataType>,
    pub global_names: Vec<Symbol>,
    pub inferred_types: HashMap<ExprId, DataType>,
}

pub trait Visitor {
    fn visit_statement(&mut self, module: &dyn HirSource, id: StmtId) {
        walk_statement(self, module, id);
    }

    fn visit_expression(&mut self, module: &dyn HirSource, id: ExprId) {
        walk_expression(self, module, id);
    }

    fn visit_function(&mut self, module: &dyn HirSource, function: &FunctionDefinition) {
        for param in &function.params {
            if let Some(default) = param.default_value {
                self.visit_expression(module, default);
            }
        }
        self.visit_statement(module, function.body);
    }
}

pub fn walk_statement<V: Visitor + ?Sized>(visitor: &mut V, module: &dyn HirSource, id: StmtId) {
    let Some(node) = module.arena().get_statement(id) else {
        return;
    };
    match &node.kind {
        StatementKind::Expression(expr) => visitor.visit_expression(module, *expr),
        StatementKind::Assign { value, .. } => visitor.visit_expression(module, *value),
        StatementKind::CompoundAssign { target, value, .. } => {
            visitor.visit_expression(module, *target);
            visitor.visit_expression(module, *value);
        }
        StatementKind::IndexAssign {
            object,
            index,
            value,
        } => {
            visitor.visit_expression(module, *object);
            visitor.visit_expression(module, *index);
            visitor.visit_expression(module, *value);
        }
        StatementKind::If {
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
        StatementKind::While { condition, body } => {
            visitor.visit_expression(module, *condition);
            visitor.visit_statement(module, *body);
        }
        StatementKind::For {
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
        StatementKind::ForEach { iterable, body, .. } => {
            visitor.visit_expression(module, *iterable);
            visitor.visit_statement(module, *body);
        }
        StatementKind::Thread { body } => visitor.visit_statement(module, *body),
        StatementKind::Try { body, handlers } => {
            visitor.visit_statement(module, *body);
            for handler in handlers {
                visitor.visit_statement(module, handler.body);
            }
        }
        StatementKind::Raise { message, .. } => {
            if let Some(message) = message {
                visitor.visit_expression(module, *message);
            }
        }
        StatementKind::Block(statements) => {
            for statement in statements {
                visitor.visit_statement(module, *statement);
            }
        }
        StatementKind::Return(value) => {
            if let Some(value) = value {
                visitor.visit_expression(module, *value);
            }
        }
        StatementKind::FunctionDefinition(function) => visitor.visit_function(module, function),
        StatementKind::ClassDefinition(class) => {
            for (_, _, field) in class.fields.values() {
                if let crate::ast::program::FieldData::Expression(Some(expr)) = field {
                    visitor.visit_expression(module, *expr);
                }
            }
        }
        StatementKind::PropertyAssign { object, value, .. } => {
            visitor.visit_expression(module, *object);
            visitor.visit_expression(module, *value);
        }
        StatementKind::Import(_)
        | StatementKind::NativeLibraryDefinition(_)
        | StatementKind::Empty => {}
    }
}

pub fn walk_expression<V: Visitor + ?Sized>(visitor: &mut V, module: &dyn HirSource, id: ExprId) {
    let Some(node) = module.arena().get_expression(id) else {
        return;
    };
    match &node.kind {
        ExpressionKind::Binary { left, right, .. } => {
            visitor.visit_expression(module, *left);
            visitor.visit_expression(module, *right);
        }
        ExpressionKind::Unary { operand, .. } => visitor.visit_expression(module, *operand),
        ExpressionKind::FunctionCall { function, args } => {
            visitor.visit_expression(module, *function);
            for arg in args {
                visitor.visit_expression(module, arg.value);
            }
        }
        ExpressionKind::Index { object, index } => {
            visitor.visit_expression(module, *object);
            visitor.visit_expression(module, *index);
        }
        ExpressionKind::PropertyAccess { object, .. } => visitor.visit_expression(module, *object),
        ExpressionKind::MethodCall { object, args, .. } => {
            visitor.visit_expression(module, *object);
            for arg in args {
                visitor.visit_expression(module, arg.value);
            }
        }
        ExpressionKind::ObjectCreation { args, .. } => {
            for arg in args {
                visitor.visit_expression(module, arg.value);
            }
        }
        ExpressionKind::Lambda { params, body } => {
            for param in params {
                if let Some(default) = param.default_value {
                    visitor.visit_expression(module, default);
                }
            }
            visitor.visit_statement(module, *body);
        }
        ExpressionKind::Literal(_) | ExpressionKind::Identifier(_) | ExpressionKind::This => {}
    }
}

pub struct Lowerer {
    resolutions: ResolutionTables,
    globals: HashMap<Symbol, u32>,
    scopes: Vec<HashMap<Symbol, u32>>,
    function_depth: usize,
    function_scope_starts: Vec<usize>,
    next_local_slots: Vec<u32>,
}

#[derive(Default)]
struct ResolutionTables {
    names: HashMap<ExprId, Binding>,
    stores: HashMap<StmtId, Binding>,
    modules: HashSet<ExprId>,
    methods: HashMap<ExprId, MethodResolution>,
}

impl Lowerer {
    pub fn lower(module: &dyn HirSource) -> HirModule {
        let mut globals = HashMap::new();
        for name in module
            .global_names()
            .into_iter()
            .chain(module.functions().into_iter().map(|function| function.name))
            .chain(module.class_names())
        {
            let next = globals.len() as u32;
            globals.entry(name).or_insert(next);
        }
        for statement in module.body() {
            if let Some(node) = module.arena().get_statement(*statement) {
                if let StatementKind::Assign { name, .. } = node.kind {
                    let next = globals.len() as u32;
                    globals.entry(name).or_insert(next);
                }
            }
        }

        let mut resolver = Self {
            resolutions: ResolutionTables::default(),
            globals,
            scopes: vec![HashMap::new()],
            function_depth: 0,
            function_scope_starts: Vec::new(),
            next_local_slots: Vec::new(),
        };
        for statement in module.body() {
            resolver.visit_statement(module, *statement);
        }
        for function in module.functions() {
            resolver.visit_function(module, &function);
        }
        let mut global_names = resolver.globals.into_iter().collect::<Vec<_>>();
        global_names.sort_unstable_by_key(|(_, slot)| *slot);
        let mut hir = HirModule {
            global_names: global_names.into_iter().map(|(name, _)| name).collect(),
            body: module.body().to_vec(),
            functions: module.functions_to_type_check(),
            callable_signatures: module.callable_signatures(),
            type_definitions: module.arena().types.clone(),
            ..HirModule::default()
        };
        let mut materializer = Materializer::new(module, &resolver.resolutions, &mut hir);
        materializer.lower_all();
        hir
    }

    fn declare(&mut self, name: Symbol) -> u32 {
        let next = self.next_local_slots.last_mut().map(|next| {
            let slot = *next;
            *next += 1;
            slot
        });
        let scope = self.scopes.last_mut().expect("resolver always has a scope");
        let next = next.unwrap_or(scope.len() as u32);
        *scope.entry(name).or_insert(next)
    }

    fn binding(&self, name: Symbol) -> Binding {
        let start = self.function_scope_starts.last().copied().unwrap_or(0);
        for scope in self.scopes[start..].iter().rev() {
            if let Some(slot) = scope.get(&name) {
                return Binding::LocalSlot(*slot);
            }
        }
        self.globals
            .get(&name)
            .copied()
            .map(Binding::GlobalSlot)
            .unwrap_or(Binding::Dynamic(name))
    }
}

struct Materializer<'a> {
    source: &'a dyn HirSource,
    resolutions: &'a ResolutionTables,
    hir: &'a mut HirModule,
}

impl<'a> Materializer<'a> {
    fn new(
        source: &'a dyn HirSource,
        resolutions: &'a ResolutionTables,
        hir: &'a mut HirModule,
    ) -> Self {
        hir.arena.reserve(
            source.arena().expressions.len(),
            source.arena().statements.len(),
        );
        Self {
            source,
            resolutions,
            hir,
        }
    }

    fn lower_all(&mut self) {
        for id in 0..self.source.arena().statements.len() as StmtId {
            self.visit_statement(self.source, id);
        }
        for id in 0..self.source.arena().expressions.len() as ExprId {
            self.visit_expression(self.source, id);
        }
    }

    fn data_type(&self, type_id: TypeId) -> DataType {
        self.source
            .arena()
            .types
            .get(type_id as usize)
            .cloned()
            .unwrap_or(DataType::Any)
    }

    fn args(args: &[CallArg]) -> Vec<HirCallArg> {
        args.iter()
            .map(|arg| HirCallArg {
                name: arg.name,
                value: arg.value,
            })
            .collect()
    }
}

impl Visitor for Materializer<'_> {
    fn visit_statement(&mut self, module: &dyn HirSource, id: StmtId) {
        if self.hir.arena.statement(id).is_some() {
            return;
        }
        let Some(node) = module.arena().get_statement(id) else {
            return;
        };
        let kind = match &node.kind {
            StatementKind::Expression(value) => HirStatementKind::Expression(*value),
            StatementKind::Import(item) => HirStatementKind::Import(item.clone()),
            StatementKind::Assign {
                name,
                is_const,
                type_hint,
                value,
            } => HirStatementKind::Assign {
                name: *name,
                binding: self
                    .resolutions
                    .stores
                    .get(&id)
                    .copied()
                    .unwrap_or(Binding::Dynamic(*name)),
                is_const: *is_const,
                declared_type: type_hint.map(|id| self.data_type(id)),
                value: *value,
            },
            StatementKind::CompoundAssign { target, op, value } => {
                HirStatementKind::CompoundAssign {
                    target: *target,
                    op: *op,
                    value: *value,
                }
            }
            StatementKind::IndexAssign {
                object,
                index,
                value,
            } => HirStatementKind::IndexAssign {
                object: *object,
                index: *index,
                value: *value,
            },
            StatementKind::If {
                condition,
                then_body,
                else_body,
            } => HirStatementKind::If {
                condition: *condition,
                then_body: *then_body,
                else_body: *else_body,
            },
            StatementKind::While { condition, body } => HirStatementKind::While {
                condition: *condition,
                body: *body,
            },
            StatementKind::For {
                variable,
                init,
                condition,
                update,
                body,
            } => HirStatementKind::For {
                variable: *variable,
                binding: self
                    .resolutions
                    .stores
                    .get(&id)
                    .copied()
                    .unwrap_or(Binding::Dynamic(*variable)),
                init: *init,
                condition: *condition,
                update: *update,
                body: *body,
            },
            StatementKind::ForEach {
                variable,
                iterable,
                body,
            } => HirStatementKind::ForEach {
                variable: *variable,
                binding: self
                    .resolutions
                    .stores
                    .get(&id)
                    .copied()
                    .unwrap_or(Binding::Dynamic(*variable)),
                iterable: *iterable,
                body: *body,
            },
            StatementKind::Thread { body } => HirStatementKind::Thread { body: *body },
            StatementKind::Try { body, handlers } => HirStatementKind::Try {
                body: *body,
                handlers: handlers.clone(),
            },
            StatementKind::Raise {
                error_type,
                message,
            } => HirStatementKind::Raise {
                error_type: *error_type,
                message: *message,
            },
            StatementKind::Block(statements) => HirStatementKind::Block(statements.clone()),
            StatementKind::Return(value) => HirStatementKind::Return(*value),
            StatementKind::FunctionDefinition(function) => {
                HirStatementKind::FunctionDefinition(function.clone())
            }
            StatementKind::NativeLibraryDefinition(definition) => {
                HirStatementKind::NativeLibraryDefinition(definition.clone())
            }
            StatementKind::ClassDefinition(class) => {
                HirStatementKind::ClassDefinition(class.clone())
            }
            StatementKind::PropertyAssign {
                object,
                property,
                value,
            } => HirStatementKind::PropertyAssign {
                object: *object,
                property: *property,
                value: *value,
            },
            StatementKind::Empty => HirStatementKind::Empty,
        };
        self.hir.arena.insert_statement(
            id,
            HirStatement {
                kind,
                span: node.span,
            },
        );
        walk_statement(self, module, id);
    }

    fn visit_expression(&mut self, module: &dyn HirSource, id: ExprId) {
        if self.hir.arena.expression(id).is_some() {
            return;
        }
        let Some(node) = module.arena().get_expression(id) else {
            return;
        };
        let kind = match &node.kind {
            ExpressionKind::Literal(value) => HirExpressionKind::Literal(value.clone()),
            ExpressionKind::Identifier(name) => HirExpressionKind::Identifier {
                name: *name,
                binding: self
                    .resolutions
                    .names
                    .get(&id)
                    .copied()
                    .unwrap_or(Binding::Dynamic(*name)),
                is_module: self.resolutions.modules.contains(&id),
            },
            ExpressionKind::Binary { op, left, right } => HirExpressionKind::Binary {
                op: *op,
                left: *left,
                right: *right,
            },
            ExpressionKind::Unary { op, operand } => HirExpressionKind::Unary {
                op: *op,
                operand: *operand,
            },
            ExpressionKind::FunctionCall { function, args } => HirExpressionKind::FunctionCall {
                function: *function,
                args: Self::args(args),
            },
            ExpressionKind::Index { object, index } => HirExpressionKind::Index {
                object: *object,
                index: *index,
            },
            ExpressionKind::PropertyAccess { object, property } => {
                HirExpressionKind::PropertyAccess {
                    object: *object,
                    property: *property,
                }
            }
            ExpressionKind::MethodCall {
                object,
                method,
                args,
            } => HirExpressionKind::MethodCall {
                object: *object,
                resolution: self
                    .resolutions
                    .methods
                    .get(&id)
                    .copied()
                    .unwrap_or(MethodResolution::Dynamic(*method)),
                args: Self::args(args),
            },
            ExpressionKind::ObjectCreation { class_name, args } => {
                HirExpressionKind::ObjectCreation {
                    class_name: *class_name,
                    args: Self::args(args),
                }
            }
            ExpressionKind::Lambda { params, body } => HirExpressionKind::Lambda {
                params: params.clone(),
                body: *body,
            },
            ExpressionKind::This => HirExpressionKind::This,
        };
        self.hir.arena.insert_expression(
            id,
            HirExpression {
                kind,
                span: node.span,
                declared_type: node.type_hint.map(|id| self.data_type(id)),
                inferred_type: DataType::Any,
            },
        );
        walk_expression(self, module, id);
    }
}

impl Visitor for Lowerer {
    fn visit_statement(&mut self, module: &dyn HirSource, id: StmtId) {
        let Some(node) = module.arena().get_statement(id) else {
            return;
        };
        match &node.kind {
            StatementKind::Assign { name, value, .. } => {
                self.visit_expression(module, *value);
                let binding = if self.function_depth > 0 {
                    match self.binding(*name) {
                        Binding::LocalSlot(slot) => Binding::LocalSlot(slot),
                        _ => Binding::LocalSlot(self.declare(*name)),
                    }
                } else {
                    self.binding(*name)
                };
                self.resolutions.stores.insert(id, binding);
            }
            StatementKind::For {
                variable,
                init,
                condition,
                update,
                body,
            } => {
                self.visit_expression(module, *init);
                self.scopes.push(HashMap::new());
                let binding = if self.function_depth > 0 {
                    Binding::LocalSlot(self.declare(*variable))
                } else {
                    Binding::Dynamic(*variable)
                };
                self.resolutions.stores.insert(id, binding);
                self.visit_expression(module, *condition);
                self.visit_statement(module, *update);
                self.visit_statement(module, *body);
                self.scopes.pop();
            }
            StatementKind::ForEach {
                variable,
                iterable,
                body,
            } => {
                self.visit_expression(module, *iterable);
                self.scopes.push(HashMap::new());
                let binding = if self.function_depth > 0 {
                    Binding::LocalSlot(self.declare(*variable))
                } else {
                    Binding::Dynamic(*variable)
                };
                self.resolutions.stores.insert(id, binding);
                self.visit_statement(module, *body);
                self.scopes.pop();
            }
            StatementKind::Block(statements) => {
                self.scopes.push(HashMap::new());
                for statement in statements {
                    self.visit_statement(module, *statement);
                }
                self.scopes.pop();
            }
            _ => walk_statement(self, module, id),
        }
    }

    fn visit_expression(&mut self, module: &dyn HirSource, id: ExprId) {
        let Some(node) = module.arena().get_expression(id) else {
            return;
        };
        match node.kind {
            ExpressionKind::Identifier(name) => {
                self.resolutions.names.insert(id, self.binding(name));
                if module.is_module_name(name) {
                    self.resolutions.modules.insert(id);
                }
            }
            ExpressionKind::MethodCall { method, .. } => {
                self.resolutions
                    .methods
                    .insert(id, MethodResolution::Dynamic(method));
                walk_expression(self, module, id);
            }
            ExpressionKind::Lambda { ref params, body } => {
                for param in params {
                    if let Some(default) = param.default_value {
                        self.visit_expression(module, default);
                    }
                }
                self.function_depth += 1;
                self.scopes.push(HashMap::new());
                self.function_scope_starts.push(self.scopes.len() - 1);
                self.next_local_slots.push(0);
                for param in params {
                    self.declare(param.name);
                }
                self.visit_statement(module, body);
                self.next_local_slots.pop();
                self.function_scope_starts.pop();
                self.scopes.pop();
                self.function_depth -= 1;
            }
            _ => walk_expression(self, module, id),
        }
    }

    fn visit_function(&mut self, module: &dyn HirSource, function: &FunctionDefinition) {
        for param in &function.params {
            if let Some(default) = param.default_value {
                self.visit_expression(module, default);
            }
        }
        self.function_depth += 1;
        self.scopes.push(HashMap::new());
        self.function_scope_starts.push(self.scopes.len() - 1);
        self.next_local_slots.push(0);
        for param in &function.params {
            self.declare(param.name);
        }
        self.visit_statement(module, function.body);
        self.next_local_slots.pop();
        self.function_scope_starts.pop();
        self.scopes.pop();
        self.function_depth -= 1;
    }
}

pub fn declared_names(module: &dyn HirSource) -> HashSet<Symbol> {
    module
        .functions()
        .into_iter()
        .map(|function| function.name)
        .chain(module.class_names())
        .collect()
}
