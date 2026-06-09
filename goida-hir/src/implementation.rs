use crate::ast::prelude::{
    AstArena, ExprId, ExpressionKind, FunctionDefinition, StatementKind, StmtId, TypeId,
};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use string_interner::DefaultSymbol as Symbol;

pub trait HirSource {
    fn arena(&self) -> &AstArena;
    fn body(&self) -> &[StmtId];
    fn global_names(&self) -> Vec<Symbol>;
    fn functions(&self) -> Vec<Arc<FunctionDefinition>>;
    fn class_names(&self) -> Vec<Symbol>;
    fn module_names(&self) -> Vec<Symbol>;
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
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
    pub names: HashMap<ExprId, Binding>,
    pub stores: HashMap<StmtId, Binding>,
    pub types: HashMap<ExprId, TypeId>,
    pub modules: HashMap<ExprId, Symbol>,
    pub methods: HashMap<ExprId, MethodResolution>,
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
        StatementKind::NativeLibraryDefinition(_) | StatementKind::Empty => {}
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

pub struct Resolver {
    hir: HirModule,
    globals: HashMap<Symbol, u32>,
    scopes: Vec<HashMap<Symbol, u32>>,
    function_depth: usize,
    function_scope_starts: Vec<usize>,
    next_local_slots: Vec<u32>,
}

impl Resolver {
    pub fn resolve(module: &dyn HirSource) -> HirModule {
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
            hir: HirModule::default(),
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
        resolver.hir
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

impl Visitor for Resolver {
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
                self.hir.stores.insert(id, binding);
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
                self.hir.stores.insert(id, binding);
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
                self.hir.stores.insert(id, binding);
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
        if let Some(type_id) = node.type_hint {
            self.hir.types.insert(id, type_id);
        }
        match node.kind {
            ExpressionKind::Identifier(name) => {
                self.hir.names.insert(id, self.binding(name));
                if module.module_names().contains(&name) {
                    self.hir.modules.insert(id, name);
                }
            }
            ExpressionKind::MethodCall { method, .. } => {
                self.hir
                    .methods
                    .insert(id, MethodResolution::Dynamic(method));
                walk_expression(self, module, id);
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
