use crate::ast::prelude::*;
use crate::parser::structs::{ParseError, Parser as ParserTrait};
use pest::Parser;
use pest_derive::Parser;

#[derive(Parser)]
#[grammar = "grammar.pest"]
struct ProgramParser;

impl ParserTrait {
    pub fn new(name: String) -> Self {
        Self {
            program: Program::new(name),
        }
    }

    pub fn parse(mut self, code: &str) -> Result<Program, ParseError> {
        let pairs = ProgramParser::parse(Rule::program, code)
            .map_err(|e| ParseError::UnexpectedToken(e.to_string()))?;

        for pair in pairs {
            if pair.as_rule() == Rule::program {
                for inner in pair.into_inner() {
                    match inner.as_rule() {
                        Rule::function => {
                            if let Some(stmt_id) = self.parse_function(inner) {
                                self.program.statements.push(stmt_id);
                            }
                        }
                        Rule::class => {
                            if let Some(stmt_id) = self.parse_class(inner) {
                                self.program.statements.push(stmt_id);
                            }
                        }
                        Rule::assignment => {
                            if let Some(stmt_id) = self.parse_assignment(inner) {
                                self.program.statements.push(stmt_id);
                            }
                        }
                        Rule::property_assign => {
                            if let Some(stmt_id) = self.parse_property_assign(inner) {
                                self.program.statements.push(stmt_id);
                            }
                        }
                        Rule::if_stmt => {
                            if let Some(stmt_id) = self.parse_if_stmt(inner) {
                                self.program.statements.push(stmt_id);
                            }
                        }
                        Rule::while_stmt => {
                            if let Some(stmt_id) = self.parse_while_stmt(inner) {
                                self.program.statements.push(stmt_id);
                            }
                        }
                        Rule::for_stmt => {
                            if let Some(stmt_id) = self.parse_for_stmt(inner) {
                                self.program.statements.push(stmt_id);
                            }
                        }
                        Rule::return_stmt => {
                            if let Some(stmt_id) = self.parse_return_stmt(inner) {
                                self.program.statements.push(stmt_id);
                            }
                        }
                        Rule::expr_stmt => {
                            if let Some(stmt_id) = self.parse_expr_stmt(inner) {
                                self.program.statements.push(stmt_id);
                            }
                        }
                        _ => {}
                    }
                }
            }
        }

        Ok(self.program)
    }

    fn parse_function(&mut self, pair: pest::iterators::Pair<Rule>) -> Option<StmtId> {
        let mut inner = pair.into_inner();
        let name = inner.next()?.as_str().to_string();

        let mut params = Vec::new();
        let mut return_type = None;

        while let Some(token) = inner.next() {
            match token.as_rule() {
                Rule::param_list => {
                    params = self.parse_param_list(token);
                }
                Rule::return_type => {
                    let type_str = token.into_inner().next()?.as_str().to_string();
                    return_type = Some(self.program.arena.resolve_or_intern_type(&type_str));
                }
                Rule::block => {
                    let body = self.parse_block(token)?;
                    let body_id = self
                        .program
                        .arena
                        .add_statement(StatementKind::Block(body), Span::default());

                    let func_def = FunctionDefinition {
                        name: self.program.arena.intern_string(&name),
                        params,
                        return_type,
                        body: body_id,
                        span: Span::default(),
                        module: None,
                    };

                    self.program.functions.push(func_def.clone());
                    let stmt_id = self.program.arena.add_statement(
                        StatementKind::FunctionDefinition(func_def),
                        Span::default(),
                    );
                    return Some(stmt_id);
                }
                _ => {}
            }
        }
        None
    }

    fn parse_class(&mut self, pair: pest::iterators::Pair<Rule>) -> Option<StmtId> {
        let mut inner = pair.into_inner();
        let name = inner.next()?.as_str().to_string();

        let mut fields = Vec::new();
        let mut methods = Vec::new();

        while let Some(token) = inner.next() {
            match token.as_rule() {
                Rule::class_field => {
                    if let Some(field) = self.parse_class_field(token) {
                        fields.push(field);
                    }
                }
                Rule::constructor => {
                    if let Some(mut method) = self.parse_constructor(token) {
                        method.is_constructor = true;
                        methods.push(method);
                    }
                }
                Rule::class_method => {
                    if let Some(method) = self.parse_class_method(token) {
                        methods.push(method);
                    }
                }
                _ => {}
            }
        }

        let class_def = ClassDefinition {
            name: self.program.arena.intern_string(&name),
            fields,
            methods,
            span: Span::default(),
        };

        self.program.classes.push(class_def.clone());
        let stmt_id = self
            .program
            .arena
            .add_statement(StatementKind::ClassDefinition(class_def), Span::default());
        Some(stmt_id)
    }

    fn parse_class_field(&mut self, pair: pest::iterators::Pair<Rule>) -> Option<ClassField> {
        let mut inner = pair.into_inner();
        let mut visibility = Visibility::Private;
        let mut field_name = String::new();
        let mut field_type = None;
        let mut default_value = None;

        while let Some(token) = inner.next() {
            match token.as_rule() {
                Rule::visibility => {
                    visibility = if token.as_str() == "публичный" {
                        Visibility::Public
                    } else {
                        Visibility::Private
                    };
                }
                Rule::identifier => {
                    field_name = token.as_str().to_string();
                }
                Rule::type_name => {
                    field_type = Some(self.program.arena.resolve_or_intern_type(token.as_str()));
                }
                Rule::expression => {
                    default_value = Some(self.parse_expression(token)?);
                }
                _ => {}
            }
        }

        Some(ClassField {
            name: self.program.arena.intern_string(&field_name),
            field_type,
            visibility,
            default_value,
            span: Span::default(),
        })
    }

    fn parse_constructor(&mut self, pair: pest::iterators::Pair<Rule>) -> Option<ClassMethod> {
        let mut inner = pair.into_inner();
        let mut visibility = Visibility::Private;
        let mut method_name = String::new();
        let mut params = Vec::new();
        let mut return_type = None;
        let mut body = None;

        while let Some(token) = inner.next() {
            match token.as_rule() {
                Rule::visibility => {
                    visibility = if token.as_str() == "публичный" {
                        Visibility::Public
                    } else {
                        Visibility::Private
                    };
                }
                Rule::identifier => {
                    method_name = token.as_str().to_string();
                }
                Rule::param_list => {
                    params = self.parse_param_list(token);
                }
                Rule::return_type => {
                    let type_str = token.into_inner().next()?.as_str().to_string();
                    return_type = Some(self.program.arena.resolve_or_intern_type(&type_str));
                }
                Rule::block => {
                    let block_stmts = self.parse_block(token)?;
                    body = Some(
                        self.program
                            .arena
                            .add_statement(StatementKind::Block(block_stmts), Span::default()),
                    );
                }
                _ => {}
            }
        }

        Some(ClassMethod {
            name: self.program.arena.intern_string(&method_name),
            params,
            return_type,
            body: body?,
            visibility,
            is_constructor: false, // будет установлено в parse_class
            span: Span::default(),
        })
    }

    fn parse_class_method(&mut self, pair: pest::iterators::Pair<Rule>) -> Option<ClassMethod> {
        let mut inner = pair.into_inner();
        let mut visibility = Visibility::Private;
        let mut method_name = String::new();
        let mut params = Vec::new();
        let mut return_type = None;
        let mut body = None;

        while let Some(token) = inner.next() {
            match token.as_rule() {
                Rule::visibility => {
                    visibility = if token.as_str() == "публичный" {
                        Visibility::Public
                    } else {
                        Visibility::Private
                    };
                }
                Rule::identifier => {
                    method_name = token.as_str().to_string();
                }
                Rule::param_list => {
                    params = self.parse_param_list(token);
                }
                Rule::return_type => {
                    let type_str = token.into_inner().next()?.as_str().to_string();
                    return_type = Some(self.program.arena.resolve_or_intern_type(&type_str));
                }
                Rule::block => {
                    let block_stmts = self.parse_block(token)?;
                    body = Some(
                        self.program
                            .arena
                            .add_statement(StatementKind::Block(block_stmts), Span::default()),
                    );
                }
                _ => {}
            }
        }

        Some(ClassMethod {
            name: self.program.arena.intern_string(&method_name),
            params,
            return_type,
            body: body?,
            visibility,
            is_constructor: false,
            span: Span::default(),
        })
    }

    fn parse_param_list(&mut self, pair: pest::iterators::Pair<Rule>) -> Vec<Parameter> {
        let mut params = Vec::new();

        for param_pair in pair.into_inner() {
            if param_pair.as_rule() == Rule::param {
                let mut param_inner = param_pair.into_inner();
                let name = param_inner
                    .next()
                    .map(|p| p.as_str().to_string())
                    .unwrap_or_default();
                let param_type = if let Some(type_pair) = param_inner.next() {
                    let type_str = type_pair.as_str().to_string();
                    self.program.arena.resolve_or_intern_type(&type_str)
                } else {
                    self.program.arena.resolve_or_intern_type("неизвестно")
                };

                params.push(Parameter {
                    name: self.program.arena.intern_string(&name),
                    param_type,
                    span: Span::default(),
                });
            }
        }

        params
    }

    fn parse_block(&mut self, pair: pest::iterators::Pair<Rule>) -> Option<Vec<StmtId>> {
        let mut statements = Vec::new();

        for inner in pair.into_inner() {
            match inner.as_rule() {
                Rule::assignment => {
                    if let Some(stmt_id) = self.parse_assignment(inner) {
                        statements.push(stmt_id);
                    }
                }
                Rule::property_assign => {
                    if let Some(stmt_id) = self.parse_property_assign(inner) {
                        statements.push(stmt_id);
                    }
                }
                Rule::if_stmt => {
                    if let Some(stmt_id) = self.parse_if_stmt(inner) {
                        statements.push(stmt_id);
                    }
                }
                Rule::while_stmt => {
                    if let Some(stmt_id) = self.parse_while_stmt(inner) {
                        statements.push(stmt_id);
                    }
                }
                Rule::for_stmt => {
                    if let Some(stmt_id) = self.parse_for_stmt(inner) {
                        statements.push(stmt_id);
                    }
                }
                Rule::return_stmt => {
                    if let Some(stmt_id) = self.parse_return_stmt(inner) {
                        statements.push(stmt_id);
                    }
                }
                Rule::expr_stmt => {
                    if let Some(stmt_id) = self.parse_expr_stmt(inner) {
                        statements.push(stmt_id);
                    }
                }
                _ => {}
            }
        }

        Some(statements)
    }

    fn parse_assignment(&mut self, pair: pest::iterators::Pair<Rule>) -> Option<StmtId> {
        let mut inner = pair.into_inner();
        let name_str = inner.next()?.as_str().to_string();
        let name = self.program.arena.intern_string(&name_str);

        let mut type_hint = None;
        let mut value = None;

        while let Some(token) = inner.next() {
            match token.as_rule() {
                Rule::type_hint => {
                    let type_str = token.into_inner().next()?.as_str().to_string();
                    type_hint = Some(self.program.arena.resolve_or_intern_type(&type_str));
                }
                Rule::expression => {
                    value = self.parse_expression(token);
                }
                _ => {}
            }
        }

        let stmt_id = self.program.arena.add_statement(
            StatementKind::Assign {
                name,
                type_hint,
                value: value?,
            },
            Span::default(),
        );
        Some(stmt_id)
    }

    fn parse_property_assign(&mut self, pair: pest::iterators::Pair<Rule>) -> Option<StmtId> {
        let mut inner = pair.into_inner();
        
        // Parse the postfix expression (e.g., "это.поле" or "объект.свойство")
        let postfix_pair = inner.next()?;
        let postfix_expr = self.parse_postfix(postfix_pair)?;
        
        // The postfix expression should be a PropertyAccess
        // Extract the object and property from it
        if let ExpressionKind::PropertyAccess { object, property } = &self.program.arena.expressions[postfix_expr as usize].kind {
            let object = *object;
            let property = *property;
            
            let value_expr = self.parse_expression(inner.next()?)?;
            
            let stmt_id = self.program.arena.add_statement(
                StatementKind::PropertyAssign {
                    object,
                    property,
                    value: value_expr,
                },
                Span::default(),
            );
            Some(stmt_id)
        } else {
            None
        }
    }

    fn parse_if_stmt(&mut self, pair: pest::iterators::Pair<Rule>) -> Option<StmtId> {
        let mut inner = pair.into_inner();

        let condition = self.parse_expression(inner.next()?)?;

        let then_block = self.parse_block(inner.next()?)?;
        let then_body = self
            .program
            .arena
            .add_statement(StatementKind::Block(then_block), Span::default());

        let mut else_body = None;

        if let Some(else_clause) = inner.next() {
            if else_clause.as_rule() == Rule::else_clause {
                let mut clause_inner = else_clause.into_inner();
                
                if let Some(else_content) = clause_inner.next() {
                    match else_content.as_rule() {
                        Rule::else_if_clause => {
                            if let Some(if_stmt) = else_content.into_inner().next() {
                                else_body = self.parse_if_stmt(if_stmt);
                            }
                        }
                        Rule::block => {
                            let else_block = self.parse_block(else_content)?;
                            else_body = Some(
                                self.program
                                    .arena
                                    .add_statement(StatementKind::Block(else_block), Span::default()),
                            );
                        }
                        _ => {}
                    }
                }
            }
        }

        let stmt_id = self.program.arena.add_statement(
            StatementKind::If {
                condition,
                then_body,
                else_body,
            },
            Span::default(),
        );
        Some(stmt_id)
    }

    fn parse_while_stmt(&mut self, pair: pest::iterators::Pair<Rule>) -> Option<StmtId> {
        let mut inner = pair.into_inner();

        let condition = self.parse_expression(inner.next()?)?;

        let block_stmts = self.parse_block(inner.next()?)?;
        let body = self
            .program
            .arena
            .add_statement(StatementKind::Block(block_stmts), Span::default());

        let stmt_id = self
            .program
            .arena
            .add_statement(StatementKind::While { condition, body }, Span::default());
        Some(stmt_id)
    }

    fn parse_for_stmt(&mut self, pair: pest::iterators::Pair<Rule>) -> Option<StmtId> {
        let mut inner = pair.into_inner();

        let for_init = inner.next()?;
        let mut init_inner = for_init.into_inner();
        let variable_str = init_inner.next()?.as_str().to_string();
        let variable = self.program.arena.intern_string(&variable_str);
        let init_expr = self.parse_expression(init_inner.next()?)?;

        let for_cond_token = inner.next()?;
        let mut cond_inner = for_cond_token.into_inner();
        let cond_expr_token = cond_inner.next()?;
        let condition_expr = self.parse_expression(cond_expr_token)?;

        let for_upd_token = inner.next()?;

        let mut upd_inner = for_upd_token.into_inner();
        let first_upd_token = upd_inner.next()?;

        let update_expr = match first_upd_token.as_rule() {
            Rule::compound_assign => {
                let mut ca_inner = first_upd_token.into_inner();
                let var_str = ca_inner.next()?.as_str().to_string();
                let op_str = ca_inner.next()?.as_str().to_string();
                let val_expr = self.parse_expression(ca_inner.next()?)?;

                let var_sym = self.program.arena.intern_string(&var_str);
                let var_expr = self
                    .program
                    .arena
                    .add_expression(ExpressionKind::Identifier(var_sym), Span::default());

                let bin_op = match op_str.as_str() {
                    "+=" => BinaryOperator::Add,
                    "-=" => BinaryOperator::Sub,
                    "*=" => BinaryOperator::Mul,
                    "/=" => BinaryOperator::Div,
                    _ => BinaryOperator::Add,
                };

                self.program.arena.add_expression(
                    ExpressionKind::Binary {
                        left: var_expr,
                        op: bin_op,
                        right: val_expr,
                    },
                    Span::default(),
                )
            }
            Rule::assignment_expr => {
                let mut ae_inner = first_upd_token.into_inner();
                let _var_str = ae_inner.next()?.as_str().to_string();
                let val_expr = self.parse_expression(ae_inner.next()?)?;
                val_expr
            }
            _ => self.parse_expression(first_upd_token)?,
        };

        let block_stmts = self.parse_block(inner.next()?)?;
        let body = self
            .program
            .arena
            .add_statement(StatementKind::Block(block_stmts), Span::default());

        let stmt_id = self.program.arena.add_statement(
            StatementKind::For {
                variable,
                init: init_expr,
                condition: condition_expr,
                update: update_expr,
                body,
            },
            Span::default(),
        );
        Some(stmt_id)
    }
    
    fn parse_return_stmt(&mut self, pair: pest::iterators::Pair<Rule>) -> Option<StmtId> {
        let inner = pair.into_inner();

        let mut expr = None;
        for token in inner {
            if token.as_rule() == Rule::expression {
                expr = self.parse_expression(token);
                break;
            }
        }

        let stmt_id = self
            .program
            .arena
            .add_statement(StatementKind::Return(expr), Span::default());
        Some(stmt_id)
    }

    fn parse_expr_stmt(&mut self, pair: pest::iterators::Pair<Rule>) -> Option<StmtId> {
        let inner = pair.into_inner();

        for token in inner {
            if token.as_rule() == Rule::expression {
                let expr = self.parse_expression(token)?;
                let stmt_id = self
                    .program
                    .arena
                    .add_statement(StatementKind::Expression(expr), Span::default());
                return Some(stmt_id);
            }
        }

        None
    }

    fn parse_expression(&mut self, pair: pest::iterators::Pair<Rule>) -> Option<ExprId> {
        let mut inner = pair.into_inner();

        if let Some(first_token) = inner.next() {
            return self.parse_logical_or(first_token);
        }
        None
    }

    fn parse_logical_or(&mut self, pair: pest::iterators::Pair<Rule>) -> Option<ExprId> {
        let mut inner = pair.into_inner();
        let mut left = self.parse_logical_and(inner.next()?)?;

        while let Some(token) = inner.next() {
            if token.as_str() == "или" {
                let right = self.parse_logical_and(inner.next()?)?;
                left = self.program.arena.add_expression(
                    ExpressionKind::Binary {
                        op: BinaryOperator::Or,
                        left,
                        right,
                    },
                    Span::default(),
                );
            }
        }

        Some(left)
    }

    fn parse_logical_and(&mut self, pair: pest::iterators::Pair<Rule>) -> Option<ExprId> {
        let mut inner = pair.into_inner();
        let mut left = self.parse_comparison(inner.next()?)?;

        while let Some(token) = inner.next() {
            if token.as_str() == "и" {
                let right = self.parse_comparison(inner.next()?)?;
                left = self.program.arena.add_expression(
                    ExpressionKind::Binary {
                        op: BinaryOperator::And,
                        left,
                        right,
                    },
                    Span::default(),
                );
            }
        }

        Some(left)
    }

    fn parse_comparison(&mut self, pair: pest::iterators::Pair<Rule>) -> Option<ExprId> {
        let mut inner = pair.into_inner();
        let mut left = self.parse_addition(inner.next()?)?;

        while let Some(token) = inner.next() {
            match token.as_rule() {
                Rule::comp_op => {
                    let op = match token.as_str() {
                        "<=" => BinaryOperator::Le,
                        ">=" => BinaryOperator::Ge,
                        "==" => BinaryOperator::Eq,
                        "!=" => BinaryOperator::Ne,
                        "<" => BinaryOperator::Lt,
                        ">" => BinaryOperator::Gt,
                        _ => return None,
                    };
                    let right = self.parse_addition(inner.next()?)?;
                    left = self.program.arena.add_expression(
                        ExpressionKind::Binary { op, left, right },
                        Span::default(),
                    );
                }
                _ => {}
            }
        }

        Some(left)
    }

    fn parse_addition(&mut self, pair: pest::iterators::Pair<Rule>) -> Option<ExprId> {
        let mut inner = pair.into_inner();
        let mut left = self.parse_multiplication(inner.next()?)?;

        while let Some(token) = inner.next() {
            match token.as_rule() {
                Rule::add_op => {
                    let op = match token.as_str() {
                        "+" => BinaryOperator::Add,
                        "-" => BinaryOperator::Sub,
                        _ => return None,
                    };
                    let right = self.parse_multiplication(inner.next()?)?;
                    left = self.program.arena.add_expression(
                        ExpressionKind::Binary { op, left, right },
                        Span::default(),
                    );
                }
                _ => {}
            }
        }

        Some(left)
    }

    fn parse_multiplication(&mut self, pair: pest::iterators::Pair<Rule>) -> Option<ExprId> {
        let mut inner = pair.into_inner();
        let mut left = self.parse_unary(inner.next()?)?;

        while let Some(token) = inner.next() {
            match token.as_rule() {
                Rule::mul_op => {
                    let op = match token.as_str() {
                        "*" => BinaryOperator::Mul,
                        "/" => BinaryOperator::Div,
                        "%" => BinaryOperator::Mod,
                        _ => return None,
                    };
                    let right = self.parse_unary(inner.next()?)?;
                    left = self.program.arena.add_expression(
                        ExpressionKind::Binary { op, left, right },
                        Span::default(),
                    );
                }
                _ => {}
            }
        }

        Some(left)
    }

    fn parse_unary(&mut self, pair: pest::iterators::Pair<Rule>) -> Option<ExprId> {
        let mut inner = pair.into_inner();

        let mut unary_op = None;
        while let Some(token) = inner.next() {
            match token.as_rule() {
                Rule::unary_op => {
                    unary_op = match token.as_str() {
                        "-" => Some(UnaryOperator::Negative),
                        "!" => Some(UnaryOperator::Not),
                        _ => None,
                    };
                }
                Rule::postfix => {
                    let mut expr = self.parse_postfix(token)?;
                    if let Some(op) = unary_op {
                        expr = self.program.arena.add_expression(
                            ExpressionKind::Unary { op, operand: expr },
                            Span::default(),
                        );
                    }
                    return Some(expr);
                }
                _ => {}
            }
        }

        None
    }

    fn parse_postfix(&mut self, pair: pest::iterators::Pair<Rule>) -> Option<ExprId> {
        let mut inner = pair.into_inner();
        let mut expr = self.parse_primary(inner.next()?)?;

        while let Some(token) = inner.next() {
            match token.as_rule() {
                Rule::function_call => {
                    let mut args = Vec::new();
                    for arg_pair in token.into_inner() {
                        if arg_pair.as_rule() == Rule::arg_list {
                            for arg in arg_pair.into_inner() {
                                if let Some(arg_expr) = self.parse_expression(arg) {
                                    args.push(arg_expr);
                                }
                            }
                        }
                    }

                    expr = self.program.arena.add_expression(
                        ExpressionKind::FunctionCall {
                            function: expr,
                            args,
                        },
                        Span::default(),
                    );
                }
                Rule::method_call => {
                    let mut method_inner = token.into_inner();
                    let method_name_str = method_inner.next()?.as_str().to_string();
                    let method_name = self.program.arena.intern_string(&method_name_str);

                    let mut args = Vec::new();
                    if let Some(arg_list) = method_inner.next() {
                        if arg_list.as_rule() == Rule::arg_list {
                            for arg_pair in arg_list.into_inner() {
                                if let Some(arg_expr) = self.parse_expression(arg_pair) {
                                    args.push(arg_expr);
                                }
                            }
                        }
                    }

                    expr = self.program.arena.add_expression(
                        ExpressionKind::MethodCall {
                            object: expr,
                            method: method_name,
                            args,
                        },
                        Span::default(),
                    );
                }
                Rule::property_access => {
                    let prop_name_str = token.into_inner().next()?.as_str().to_string();
                    let prop_name = self.program.arena.intern_string(&prop_name_str);
                    expr = self.program.arena.add_expression(
                        ExpressionKind::PropertyAccess {
                            object: expr,
                            property: prop_name,
                        },
                        Span::default(),
                    );
                }
                Rule::index_access => {
                    let index_expr = self.parse_expression(token.into_inner().next()?)?;
                    expr = self.program.arena.add_expression(
                        ExpressionKind::Index {
                            object: expr,
                            index: index_expr,
                        },
                        Span::default(),
                    );
                }
                _ => {}
            }
        }

        Some(expr)
    }

    fn parse_primary(&mut self, pair: pest::iterators::Pair<Rule>) -> Option<ExprId> {
        match pair.as_rule() {
            Rule::paren_expr => {
                let mut inner = pair.into_inner();
                let expr = self.parse_expression(inner.next()?)?;
                Some(expr)
            }
            Rule::new_expr => {
                let mut inner = pair.into_inner();
                let class_name_str = inner.next()?.as_str().to_string();
                let class_name = self.program.arena.intern_string(&class_name_str);

                let mut args = Vec::new();
                if let Some(arg_list) = inner.next() {
                    if arg_list.as_rule() == Rule::arg_list {
                        for arg_pair in arg_list.into_inner() {
                            if let Some(arg_expr) = self.parse_expression(arg_pair) {
                                args.push(arg_expr);
                            }
                        }
                    }
                }

                Some(self.program.arena.add_expression(
                    ExpressionKind::ObjectCreation { class_name, args },
                    Span::default(),
                ))
            }
            Rule::string_literal => {
                let s = pair.as_str();
                let trimmed = &s[1..s.len() - 1];
                let text_symbol = self.program.arena.intern_string(trimmed);
                Some(self.program.arena.add_expression(
                    ExpressionKind::Literal(LiteralValue::Text(text_symbol)),
                    Span::default(),
                ))
            }
            Rule::number_literal => {
                let s = pair.as_str();
                if s.contains('.') {
                    if let Ok(num) = s.parse::<f64>() {
                        Some(self.program.arena.add_expression(
                            ExpressionKind::Literal(LiteralValue::Float(num)),
                            Span::default(),
                        ))
                    } else {
                        None
                    }
                } else {
                    if let Ok(num) = s.parse::<i64>() {
                        Some(self.program.arena.add_expression(
                            ExpressionKind::Literal(LiteralValue::Number(num)),
                            Span::default(),
                        ))
                    } else {
                        None
                    }
                }
            }
            Rule::identifier => {
                let name_str = pair.as_str().to_string();
                let name = self.program.arena.intern_string(&name_str);
                Some(
                    self.program
                        .arena
                        .add_expression(ExpressionKind::Identifier(name), Span::default()),
                )
            }
            Rule::bool_literal => {
                let s = pair.as_str();
                let boolean_val = s == "истина";
                Some(self.program.arena.add_expression(
                    ExpressionKind::Literal(LiteralValue::Boolean(boolean_val)),
                    Span::default(),
                ))
            }
            Rule::empty_literal => {
                Some(self.program.arena.add_expression(
                    ExpressionKind::Literal(LiteralValue::Unit),
                    Span::default(),
                ))
            }
            Rule::this_expr => {
                Some(self.program.arena.add_expression(
                    ExpressionKind::This,
                    Span::default(),
                ))
            }
            _ => None,
        }
    }
}
