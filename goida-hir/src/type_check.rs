use crate::ast::prelude::{
    BinaryOperator, DataType, ErrorData, ExprId, FunctionDefinition, LiteralValue, PrimitiveType,
    Span, StmtId, UnaryOperator,
};
use crate::{
    walk_hir_expression, CallableSignature, HirCallArg, HirExpressionKind, HirModule,
    HirStatementKind, HirVisitor,
};
use std::collections::HashMap;
use string_interner::DefaultSymbol as Symbol;

#[derive(Clone, Debug)]
pub struct TypeCheckError {
    pub data: ErrorData,
}

pub struct TypeChecker {
    inferred_types: HashMap<ExprId, DataType>,
    signatures: HashMap<Symbol, CallableSignature>,
    scopes: Vec<HashMap<Symbol, DataType>>,
    expected_return: Option<DataType>,
    error: Option<TypeCheckError>,
}

impl TypeChecker {
    pub fn check(hir: &mut HirModule) -> Result<(), TypeCheckError> {
        let signatures = hir
            .callable_signatures
            .iter()
            .cloned()
            .map(|signature| (signature.name, signature))
            .collect();
        let (result, inferred_types) = {
            let lowered = &*hir;
            let mut checker = Self {
                inferred_types: HashMap::new(),
                signatures,
                scopes: vec![HashMap::new()],
                expected_return: None,
                error: None,
            };

            for statement in &lowered.body {
                checker.visit_statement(lowered, *statement);
                if checker.error.is_some() {
                    break;
                }
            }
            if checker.error.is_none() {
                for function in &lowered.functions {
                    checker.check_function(lowered, function);
                    if checker.error.is_some() {
                        break;
                    }
                }
            }

            (checker.error.map_or(Ok(()), Err), checker.inferred_types)
        };
        for (id, data_type) in inferred_types {
            if let Some(expression) = hir.arena.expression_mut(id) {
                expression.inferred_type = data_type.clone();
            }
            hir.inferred_types.insert(id, data_type);
        }
        result
    }

    fn fail(&mut self, span: Span, context: &str, expected: &DataType, actual: &DataType) {
        if self.error.is_none() {
            self.error = Some(TypeCheckError {
                data: ErrorData::new(
                    span,
                    format!(
                        "Несовместимый тип {}: ожидался {}, получен {}",
                        context,
                        describe_type(expected),
                        describe_type(actual)
                    ),
                ),
            });
        }
    }

    fn declared_type(module: &HirModule, type_id: u32) -> DataType {
        module
            .type_definitions
            .get(type_id as usize)
            .cloned()
            .unwrap_or(DataType::Any)
    }

    fn lookup(&self, name: Symbol) -> Option<DataType> {
        self.scopes
            .iter()
            .rev()
            .find_map(|scope| scope.get(&name).cloned())
    }

    fn declare(&mut self, name: Symbol, data_type: DataType) {
        if let Some(scope) = self.scopes.last_mut() {
            scope.insert(name, data_type);
        }
    }

    fn assign(&mut self, name: Symbol, data_type: DataType) {
        if let Some(scope) = self
            .scopes
            .iter_mut()
            .rev()
            .find(|scope| scope.contains_key(&name))
        {
            scope.insert(name, data_type);
        } else {
            self.declare(name, data_type);
        }
    }

    fn check_compatible(
        &mut self,
        span: Span,
        context: &str,
        expected: &DataType,
        actual: &DataType,
    ) {
        if !types_compatible(expected, actual) {
            self.fail(span, context, expected, actual);
        }
    }

    fn infer_expression(&mut self, module: &HirModule, id: ExprId) -> DataType {
        if let Some(data_type) = self.inferred_types.get(&id) {
            return data_type.clone();
        }
        let Some(node) = module.arena.expression(id) else {
            return DataType::Any;
        };

        let inferred = match &node.kind {
            HirExpressionKind::Literal(literal) => literal_type(literal),
            HirExpressionKind::Identifier { name, .. } => self.lookup(*name).unwrap_or_else(|| {
                self.signatures
                    .get(name)
                    .map(|signature| signature_type(module, signature))
                    .unwrap_or(DataType::Any)
            }),
            HirExpressionKind::Binary { op, left, right } => {
                let left = self.infer_expression(module, *left);
                let right = self.infer_expression(module, *right);
                infer_binary(*op, &left, &right)
            }
            HirExpressionKind::Unary { op, operand } => {
                let operand = self.infer_expression(module, *operand);
                match op {
                    UnaryOperator::Negative => operand,
                    UnaryOperator::Not => DataType::Primitive(PrimitiveType::Boolean),
                }
            }
            HirExpressionKind::FunctionCall { function, args } => {
                let return_type = module
                    .arena
                    .expression(*function)
                    .and_then(|function| match function.kind {
                        HirExpressionKind::Identifier { name, .. } => {
                            self.signatures.get(&name).cloned()
                        }
                        _ => None,
                    })
                    .map(|signature| {
                        self.check_call(module, &signature, args, node.span);
                        signature
                            .return_type
                            .map(|id| Self::declared_type(module, id))
                            .unwrap_or(DataType::Unit)
                    });
                if return_type.is_none() {
                    walk_hir_expression(self, module, id);
                }
                return_type.unwrap_or(DataType::Any)
            }
            HirExpressionKind::Index { object, index } => {
                let object = self.infer_expression(module, *object);
                self.infer_expression(module, *index);
                match object {
                    DataType::List(item) | DataType::Array(item) => *item,
                    DataType::Dict { value, .. } => *value,
                    _ => DataType::Any,
                }
            }
            HirExpressionKind::ObjectCreation { args, .. } => {
                for arg in args {
                    self.infer_expression(module, arg.value);
                }
                DataType::Any
            }
            HirExpressionKind::Lambda { .. } => DataType::Any,
            HirExpressionKind::PropertyAccess { object, .. }
            | HirExpressionKind::MethodCall { object, .. } => {
                self.infer_expression(module, *object);
                walk_hir_expression(self, module, id);
                DataType::Any
            }
            HirExpressionKind::This => DataType::Any,
        };

        self.inferred_types.insert(id, inferred.clone());
        inferred
    }

    fn check_call(
        &mut self,
        module: &HirModule,
        signature: &CallableSignature,
        args: &[HirCallArg],
        span: Span,
    ) {
        let mut bound = vec![None; signature.params.len()];
        let mut positional = 0;
        for arg in args {
            let index = if let Some(name) = arg.name {
                signature.params.iter().position(|param| param.name == name)
            } else {
                let index = positional;
                positional += 1;
                Some(index)
            };
            let Some(index) = index.filter(|index| *index < signature.params.len()) else {
                self.error = Some(TypeCheckError {
                    data: ErrorData::new(span, "Неверные аргументы вызова функции".into()),
                });
                return;
            };
            if bound[index].is_some() {
                self.error = Some(TypeCheckError {
                    data: ErrorData::new(span, "Аргумент функции передан несколько раз".into()),
                });
                return;
            }
            bound[index] = Some(arg.value);
        }

        for (index, param) in signature.params.iter().enumerate() {
            if let Some(argument) = bound[index] {
                let actual = self.infer_expression(module, argument);
                let expected = Self::declared_type(module, param.param_type);
                self.check_compatible(
                    module
                        .arena
                        .expression(argument)
                        .map(|node| node.span)
                        .unwrap_or(span),
                    "аргумента функции",
                    &expected,
                    &actual,
                );
            } else if param.default_value.is_none() {
                self.error = Some(TypeCheckError {
                    data: ErrorData::new(span, "Не передан обязательный аргумент функции".into()),
                });
                return;
            }
        }
    }

    fn check_function(&mut self, module: &HirModule, function: &FunctionDefinition) {
        for param in &function.params {
            if let Some(default) = param.default_value {
                let actual = self.infer_expression(module, default);
                let expected = Self::declared_type(module, param.param_type);
                self.check_compatible(
                    param.span,
                    "значения параметра по умолчанию",
                    &expected,
                    &actual,
                );
            }
        }
        if self.error.is_some() {
            return;
        }

        let previous_return = self.expected_return.take();
        self.expected_return = function
            .return_type
            .map(|id| Self::declared_type(module, id));
        self.scopes.push(HashMap::new());
        for param in &function.params {
            self.declare(param.name, Self::declared_type(module, param.param_type));
        }
        self.visit_statement(module, function.body);
        self.scopes.pop();
        self.expected_return = previous_return;
    }
}

impl HirVisitor for TypeChecker {
    fn visit_statement(&mut self, module: &HirModule, id: StmtId) {
        if self.error.is_some() {
            return;
        }
        let Some(node) = module.arena.statement(id) else {
            return;
        };
        match &node.kind {
            HirStatementKind::Assign {
                name,
                declared_type,
                value,
                ..
            } => {
                let actual = self.infer_expression(module, *value);
                if let Some(expected) = declared_type {
                    let expected = expected.clone();
                    self.check_compatible(node.span, "присваивания", &expected, &actual);
                    self.assign(*name, expected);
                } else if let Some(expected) = self.lookup(*name) {
                    self.check_compatible(node.span, "присваивания", &expected, &actual);
                } else {
                    self.declare(*name, DataType::Any);
                }
            }
            HirStatementKind::Return(value) => {
                let actual = value
                    .map(|value| self.infer_expression(module, value))
                    .unwrap_or(DataType::Unit);
                if let Some(expected) = self.expected_return.clone() {
                    self.check_compatible(node.span, "возвращаемого значения", &expected, &actual);
                }
            }
            HirStatementKind::Block(statements) => {
                self.scopes.push(HashMap::new());
                for statement in statements {
                    self.visit_statement(module, *statement);
                }
                self.scopes.pop();
            }
            HirStatementKind::FunctionDefinition(function) => self.check_function(module, function),
            _ => crate::walk_hir_statement(self, module, id),
        }
    }

    fn visit_expression(&mut self, module: &HirModule, id: ExprId) {
        self.infer_expression(module, id);
    }
}

fn signature_type(module: &HirModule, signature: &CallableSignature) -> DataType {
    DataType::Function {
        params: signature
            .params
            .iter()
            .map(|param| TypeChecker::declared_type(module, param.param_type))
            .collect(),
        return_type: Box::new(
            signature
                .return_type
                .map(|id| TypeChecker::declared_type(module, id))
                .unwrap_or(DataType::Unit),
        ),
    }
}

fn literal_type(literal: &LiteralValue) -> DataType {
    DataType::Primitive(match literal {
        LiteralValue::Number(_) => PrimitiveType::Number,
        LiteralValue::Float(_) => PrimitiveType::Float,
        LiteralValue::Text(_) => PrimitiveType::Text,
        LiteralValue::Boolean(_) => PrimitiveType::Boolean,
        LiteralValue::Unit => return DataType::Unit,
    })
}

fn infer_binary(op: BinaryOperator, left: &DataType, right: &DataType) -> DataType {
    match op {
        BinaryOperator::Eq
        | BinaryOperator::Ne
        | BinaryOperator::Lt
        | BinaryOperator::Le
        | BinaryOperator::Gt
        | BinaryOperator::Ge
        | BinaryOperator::And
        | BinaryOperator::Or => DataType::Primitive(PrimitiveType::Boolean),
        _ if left == right => left.clone(),
        _ => DataType::Any,
    }
}

fn types_compatible(expected: &DataType, actual: &DataType) -> bool {
    matches!(expected, DataType::Any)
        || matches!(actual, DataType::Any)
        || expected == actual
        || matches!(
            (expected, actual),
            (DataType::List(_), DataType::List(_))
                | (DataType::Array(_), DataType::Array(_))
                | (DataType::Dict { .. }, DataType::Dict { .. })
        )
}

fn describe_type(data_type: &DataType) -> &'static str {
    match data_type {
        DataType::Primitive(PrimitiveType::Number) => "число",
        DataType::Primitive(PrimitiveType::Float) => "дробь",
        DataType::Primitive(PrimitiveType::Text) => "строка",
        DataType::Primitive(PrimitiveType::Boolean) => "логический",
        DataType::Primitive(PrimitiveType::Pointer) => "указатель",
        DataType::List(_) => "список",
        DataType::Array(_) => "массив",
        DataType::Dict { .. } => "словарь",
        DataType::Function { .. } => "функция",
        DataType::Object(_) => "объект",
        DataType::Runtime(_) => "runtime-значение",
        DataType::Any => "неизвестно",
        DataType::Unit => "пустота",
    }
}
