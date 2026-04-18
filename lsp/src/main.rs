use goida_core::ast::prelude::{ExpressionKind, Span, StatementKind, StmtId};
use goida_core::ast::program::MethodType;
use goida_core::interpreter::prelude::{Interpreter, Module, SharedInterner};
use goida_core::parser::prelude::{ParseError, Parser};
use goida_core::traits::prelude::CoreOperations;
use std::cmp::min;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use string_interner::StringInterner;
use tokio::sync::RwLock;
use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LanguageServer, LspService, Server};

const TOKEN_TYPES: &[SemanticTokenType] = &[
    SemanticTokenType::FUNCTION,
    SemanticTokenType::VARIABLE,
    SemanticTokenType::PARAMETER,
    SemanticTokenType::CLASS,
    SemanticTokenType::PROPERTY,
];
const TOKEN_MODIFIERS: &[SemanticTokenModifier] = &[SemanticTokenModifier::DECLARATION];

#[derive(Default)]
struct ServerState {
    documents: HashMap<Url, String>,
    workspace_roots: Vec<PathBuf>,
}

#[derive(Clone)]
struct ResolvedSymbol {
    name: String,
    span: Span,
}

#[derive(Clone)]
struct LocatedIdentifier {
    name: String,
    start_char: usize,
    module_alias: Option<String>,
}

#[derive(Clone, Copy)]
struct SemanticTokenAbsolute {
    line: u32,
    start: u32,
    length: u32,
    token_type: u32,
    token_modifiers_bitset: u32,
}

struct Backend {
    client: Client,
    interpreter: Arc<RwLock<Interpreter>>,
    state: Arc<RwLock<ServerState>>,
}

#[tower_lsp::async_trait]
impl LanguageServer for Backend {
    async fn initialize(&self, params: InitializeParams) -> Result<InitializeResult> {
        let mut roots = Vec::new();
        if let Some(workspace_folders) = params.workspace_folders {
            for folder in workspace_folders {
                if let Ok(path) = folder.uri.to_file_path() {
                    roots.push(path);
                }
            }
        }
        if let Some(root_uri) = params.root_uri {
            if let Ok(path) = root_uri.to_file_path() {
                roots.push(path);
            }
        }

        self.state.write().await.workspace_roots = roots;

        Ok(InitializeResult {
            capabilities: ServerCapabilities {
                text_document_sync: Some(TextDocumentSyncCapability::Kind(
                    TextDocumentSyncKind::FULL,
                )),
                definition_provider: Some(OneOf::Left(true)),
                semantic_tokens_provider: Some(
                    SemanticTokensServerCapabilities::SemanticTokensOptions(
                        SemanticTokensOptions {
                            legend: SemanticTokensLegend {
                                token_types: TOKEN_TYPES.to_vec(),
                                token_modifiers: TOKEN_MODIFIERS.to_vec(),
                            },
                            full: Some(SemanticTokensFullOptions::Bool(true)),
                            ..Default::default()
                        },
                    ),
                ),
                ..Default::default()
            },
            ..Default::default()
        })
    }

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        self.state.write().await.documents.insert(
            params.text_document.uri.clone(),
            params.text_document.text.clone(),
        );
        self.validate(params.text_document.uri, params.text_document.text)
            .await;
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        if let Some(change) = params.content_changes.into_iter().next() {
            self.state
                .write()
                .await
                .documents
                .insert(params.text_document.uri.clone(), change.text.clone());
            self.validate(params.text_document.uri, change.text).await;
        }
    }

    async fn did_close(&self, params: DidCloseTextDocumentParams) {
        self.state
            .write()
            .await
            .documents
            .remove(&params.text_document.uri);
        self.client
            .publish_diagnostics(params.text_document.uri, Vec::new(), None)
            .await;
    }

    async fn semantic_tokens_full(
        &self,
        params: SemanticTokensParams,
    ) -> Result<Option<SemanticTokensResult>> {
        let uri = params.text_document.uri;
        let Some(text) = self.read_document_text(&uri).await else {
            return Ok(None);
        };
        let Some(path_buf) = uri.to_file_path().ok() else {
            return Ok(None);
        };
        let Some(module) = self.parse_module(&path_buf, &text).await else {
            return Ok(None);
        };

        let tokens = self.collect_semantic_tokens(&module, &text).await;
        let encoded = encode_semantic_tokens(tokens);

        Ok(Some(SemanticTokensResult::Tokens(SemanticTokens {
            result_id: None,
            data: encoded,
        })))
    }

    async fn goto_definition(
        &self,
        params: GotoDefinitionParams,
    ) -> Result<Option<GotoDefinitionResponse>> {
        let uri = params.text_document_position_params.text_document.uri;
        let position = params.text_document_position_params.position;

        let Some(text) = self.read_document_text(&uri).await else {
            return Ok(None);
        };
        let char_offset = position_to_char_offset(&text, position);
        let Some(ident) = find_identifier_at_char_offset(&text, char_offset) else {
            return Ok(None);
        };

        let Some(current_path) = uri.to_file_path().ok() else {
            return Ok(None);
        };
        let Some(module) = self.parse_module(&current_path, &text).await else {
            return Ok(None);
        };

        if let Some(alias) = ident.module_alias.as_ref() {
            if let Some(location) = self
                .resolve_import_member_definition(&module, &current_path, alias, &ident.name)
                .await
            {
                return Ok(Some(GotoDefinitionResponse::Scalar(location)));
            }
        }

        if let Some(location) = self
            .resolve_local_definition(&module, &uri, &text, &ident.name, ident.start_char)
            .await
        {
            return Ok(Some(GotoDefinitionResponse::Scalar(location)));
        }

        if let Some(location) = self
            .resolve_workspace_definition(&current_path, &ident.name)
            .await
        {
            return Ok(Some(GotoDefinitionResponse::Scalar(location)));
        }

        Ok(None)
    }

    async fn shutdown(&self) -> Result<()> {
        Ok(())
    }
}

impl Backend {
    async fn read_document_text(&self, uri: &Url) -> Option<String> {
        if let Some(text) = self.state.read().await.documents.get(uri).cloned() {
            return Some(text);
        }
        let path = uri.to_file_path().ok()?;
        fs::read_to_string(path).ok()
    }

    async fn parse_module(&self, path: &Path, text: &str) -> Option<Module> {
        let filename = path.to_str()?;
        let interner = self.interpreter.read().await.interner.clone();
        Parser::new(interner, filename, path.to_path_buf())
            .parse(text)
            .ok()
    }

    async fn resolve_workspace_definition(
        &self,
        current_file: &Path,
        symbol_name: &str,
    ) -> Option<Location> {
        let roots = self.state.read().await.workspace_roots.clone();
        let files = collect_goida_files(&roots);
        let interner = self.interpreter.read().await.interner.clone();

        for file in files {
            if file == current_file {
                continue;
            }

            let text = match fs::read_to_string(&file) {
                Ok(v) => v,
                Err(_) => continue,
            };
            let file_name = match file.to_str() {
                Some(v) => v,
                None => continue,
            };
            let module = match Parser::new(interner.clone(), file_name, file.clone()).parse(&text) {
                Ok(v) => v,
                Err(_) => continue,
            };

            if let Some(symbol) = find_top_level_symbol(&module, &interner, symbol_name) {
                if let Ok(uri) = Url::from_file_path(&file) {
                    if let Some(location) = span_to_location(&text, uri, symbol.span) {
                        return Some(location);
                    }
                }
            }
        }

        None
    }

    async fn resolve_import_member_definition(
        &self,
        module: &Module,
        current_file: &Path,
        module_alias: &str,
        member_name: &str,
    ) -> Option<Location> {
        let interner = self.interpreter.read().await.interner.clone();
        let imports = collect_imports(module, &interner);
        let import_path = imports.get(module_alias)?;

        let workspace_roots = self.state.read().await.workspace_roots.clone();
        let target_file = resolve_import_path(current_file, import_path, &workspace_roots)?;

        let text = fs::read_to_string(&target_file).ok()?;
        let target_name = target_file.to_str()?;
        let target_module = Parser::new(interner.clone(), target_name, target_file.clone())
            .parse(&text)
            .ok()?;
        let symbol = find_top_level_symbol(&target_module, &interner, member_name)?;

        let uri = Url::from_file_path(target_file).ok()?;
        span_to_location(&text, uri, symbol.span)
    }

    async fn resolve_local_definition(
        &self,
        module: &Module,
        uri: &Url,
        text: &str,
        symbol_name: &str,
        usage_start_char: usize,
    ) -> Option<Location> {
        let interner = self.interpreter.read().await.interner.clone();
        let mut declarations = Vec::new();
        collect_declarations(module, &interner, &module.body, &mut declarations);
        declarations.retain(|decl| decl.name == symbol_name);

        let mut best: Option<ResolvedSymbol> = None;
        for decl in declarations {
            let decl_start = decl.span.as_ariadne(text).start;
            if decl_start > usage_start_char {
                continue;
            }
            match &best {
                Some(current) => {
                    if current.span.as_ariadne(text).start < decl_start {
                        best = Some(decl);
                    }
                }
                None => best = Some(decl),
            }
        }

        if best.is_none() {
            if let Some(top_level) = find_top_level_symbol(module, &interner, symbol_name) {
                return span_to_location(text, uri.clone(), top_level.span);
            }
            return None;
        }

        span_to_location(text, uri.clone(), best?.span)
    }

    async fn collect_semantic_tokens(
        &self,
        module: &Module,
        text: &str,
    ) -> Vec<SemanticTokenAbsolute> {
        let interner = self.interpreter.read().await.interner.clone();
        let mut tokens = Vec::new();

        for function in module.functions.values() {
            if let Some(name) = module.arena.resolve_symbol(&interner, function.name) {
                push_name_token(&mut tokens, text, function.span, &name, 0, true);
            }
            for param in &function.params {
                if let Some(name) = module.arena.resolve_symbol(&interner, param.name) {
                    push_name_token(&mut tokens, text, param.span, &name, 2, true);
                }
            }
        }

        for class in module.classes.values() {
            class.read(|class_def| {
                if let Some(name) = module.arena.resolve_symbol(&interner, class_def.name) {
                    push_name_token(&mut tokens, text, class_def.span, &name, 3, true);
                }
            });
        }

        collect_statement_tokens(module, &interner, &module.body, text, &mut tokens);
        tokens.sort_by_key(|tok| (tok.line, tok.start, tok.length, tok.token_type));
        tokens
    }

    async fn validate(&self, uri: Url, text: String) {
        let path_buf = uri.to_file_path().expect("Invalid file URI");
        let filename = path_buf.to_str().expect("Path is not valid UTF-8");
        let intp = self.interpreter.read().await;
        let parser = Parser::new(intp.interner.clone(), filename, path_buf.clone());

        let mut diagnostics = Vec::new();
        if let Err(err) = parser.parse(&text) {
            let (msg, err_data) = match err {
                ParseError::TypeError(e) => ("Ошибка типов", e),
                ParseError::InvalidSyntax(e) => ("Некорректный синтаксис", e),
                ParseError::ImportError(e) => ("Ошибка импортов", e),
            };

            let span = err_data.location.as_ariadne(text.as_ref());
            let (sl, sc) = intp
                .source_manager
                .get_line_col_from_char_offset(text.as_ref(), span.start);
            let (el, ec) = intp
                .source_manager
                .get_line_col_from_char_offset(text.as_ref(), span.end);

            let range = Range::new(
                Position::new(sl as u32, sc as u32),
                Position::new(el as u32, ec as u32),
            );

            diagnostics.push(Diagnostic {
                range,
                message: format!("{}: {}", msg, err_data.message),
                severity: Some(DiagnosticSeverity::ERROR),
                source: Some("goida-lsp".into()),
                ..Default::default()
            });
        }

        self.client
            .publish_diagnostics(uri, diagnostics, None)
            .await;
    }
}

fn push_name_token(
    out: &mut Vec<SemanticTokenAbsolute>,
    text: &str,
    span: Span,
    name: &str,
    token_type: u32,
    declaration: bool,
) {
    if let Some(range) = find_name_char_range(text, span, name) {
        if let Some(token) =
            char_range_to_semantic_token(text, range.start, range.end, token_type, declaration)
        {
            out.push(token);
        }
    }
}

fn collect_statement_tokens(
    module: &Module,
    interner: &SharedInterner,
    statement_ids: &[StmtId],
    text: &str,
    out: &mut Vec<SemanticTokenAbsolute>,
) {
    for stmt_id in statement_ids {
        let Some(statement) = module.arena.get_statement(*stmt_id) else {
            continue;
        };

        match &statement.kind {
            StatementKind::Assign { name, value, .. } => {
                if let Some(name) = module.arena.resolve_symbol(interner, *name) {
                    push_name_token(out, text, statement.span, &name, 1, true);
                }
                collect_expression_tokens(module, interner, *value, text, out);
            }
            StatementKind::IndexAssign {
                object,
                index,
                value,
            } => {
                collect_expression_tokens(module, interner, *object, text, out);
                collect_expression_tokens(module, interner, *index, text, out);
                collect_expression_tokens(module, interner, *value, text, out);
            }
            StatementKind::PropertyAssign {
                object,
                property,
                value,
            } => {
                collect_expression_tokens(module, interner, *object, text, out);
                collect_expression_tokens(module, interner, *value, text, out);
                if let Some(name) = module.arena.resolve_symbol(interner, *property) {
                    push_name_token(out, text, statement.span, &name, 4, false);
                }
            }
            StatementKind::Expression(expr) => {
                collect_expression_tokens(module, interner, *expr, text, out)
            }
            StatementKind::If {
                condition,
                then_body,
                else_body,
            } => {
                collect_expression_tokens(module, interner, *condition, text, out);
                collect_statement_tokens(module, interner, &[*then_body], text, out);
                if let Some(else_stmt) = else_body {
                    collect_statement_tokens(module, interner, &[*else_stmt], text, out);
                }
            }
            StatementKind::While { condition, body } => {
                collect_expression_tokens(module, interner, *condition, text, out);
                collect_statement_tokens(module, interner, &[*body], text, out);
            }
            StatementKind::For {
                variable,
                init,
                condition,
                update,
                body,
            } => {
                if let Some(name) = module.arena.resolve_symbol(interner, *variable) {
                    push_name_token(out, text, statement.span, &name, 1, true);
                }
                collect_expression_tokens(module, interner, *init, text, out);
                collect_expression_tokens(module, interner, *condition, text, out);
                collect_expression_tokens(module, interner, *update, text, out);
                collect_statement_tokens(module, interner, &[*body], text, out);
            }
            StatementKind::Block(items) => {
                collect_statement_tokens(module, interner, items, text, out)
            }
            StatementKind::Return(expr) => {
                if let Some(expr_id) = expr {
                    collect_expression_tokens(module, interner, *expr_id, text, out);
                }
            }
            StatementKind::FunctionDefinition(function) => {
                if let Some(name) = module.arena.resolve_symbol(interner, function.name) {
                    push_name_token(out, text, function.span, &name, 0, true);
                }
                for param in &function.params {
                    if let Some(name) = module.arena.resolve_symbol(interner, param.name) {
                        push_name_token(out, text, param.span, &name, 2, true);
                    }
                }
                collect_statement_tokens(module, interner, &[function.body], text, out);
            }
            StatementKind::ClassDefinition(class_def) => {
                if let Some(name) = module.arena.resolve_symbol(interner, class_def.name) {
                    push_name_token(out, text, class_def.span, &name, 3, true);
                }
                if let Some(MethodType::User(constructor)) = &class_def.constructor {
                    for param in &constructor.params {
                        if let Some(name) = module.arena.resolve_symbol(interner, param.name) {
                            push_name_token(out, text, param.span, &name, 2, true);
                        }
                    }
                    collect_statement_tokens(module, interner, &[constructor.body], text, out);
                }
                for (method_name, (_, _, method_type)) in &class_def.methods {
                    if let (Some(name), MethodType::User(user_method)) = (
                        module.arena.resolve_symbol(interner, *method_name),
                        method_type,
                    ) {
                        push_name_token(out, text, user_method.span, &name, 0, true);
                        for param in &user_method.params {
                            if let Some(param_name) =
                                module.arena.resolve_symbol(interner, param.name)
                            {
                                push_name_token(out, text, param.span, &param_name, 2, true);
                            }
                        }
                        collect_statement_tokens(module, interner, &[user_method.body], text, out);
                    }
                }
            }
            StatementKind::NativeLibraryDefinition(_) | StatementKind::Empty => {}
        }
    }
}

fn collect_expression_tokens(
    module: &Module,
    interner: &SharedInterner,
    expr_id: u32,
    text: &str,
    out: &mut Vec<SemanticTokenAbsolute>,
) {
    let Some(expr) = module.arena.get_expression(expr_id) else {
        return;
    };
    match &expr.kind {
        ExpressionKind::Identifier(symbol) => {
            if let Some(name) = module.arena.resolve_symbol(interner, *symbol) {
                push_name_token(out, text, expr.span, &name, 1, false);
            }
        }
        ExpressionKind::FunctionCall { function, args } => {
            if let Some(function_expr) = module.arena.get_expression(*function) {
                if let ExpressionKind::Identifier(symbol) = function_expr.kind {
                    if let Some(name) = module.arena.resolve_symbol(interner, symbol) {
                        push_name_token(out, text, function_expr.span, &name, 0, false);
                    }
                } else {
                    collect_expression_tokens(module, interner, *function, text, out);
                }
            }
            for arg in args {
                collect_expression_tokens(module, interner, arg.value, text, out);
            }
        }
        ExpressionKind::MethodCall {
            object,
            method,
            args,
        } => {
            collect_expression_tokens(module, interner, *object, text, out);
            if let Some(name) = module.arena.resolve_symbol(interner, *method) {
                push_name_token(out, text, expr.span, &name, 4, false);
            }
            for arg in args {
                collect_expression_tokens(module, interner, arg.value, text, out);
            }
        }
        ExpressionKind::PropertyAccess { object, property } => {
            collect_expression_tokens(module, interner, *object, text, out);
            if let Some(name) = module.arena.resolve_symbol(interner, *property) {
                push_name_token(out, text, expr.span, &name, 4, false);
            }
        }
        ExpressionKind::ObjectCreation { class_name, args } => {
            if let Some(name) = module.arena.resolve_symbol(interner, *class_name) {
                push_name_token(out, text, expr.span, &name, 3, false);
            }
            for arg in args {
                collect_expression_tokens(module, interner, arg.value, text, out);
            }
        }
        ExpressionKind::Binary { left, right, .. } => {
            collect_expression_tokens(module, interner, *left, text, out);
            collect_expression_tokens(module, interner, *right, text, out);
        }
        ExpressionKind::Unary { operand, .. } => {
            collect_expression_tokens(module, interner, *operand, text, out);
        }
        ExpressionKind::Index { object, index } => {
            collect_expression_tokens(module, interner, *object, text, out);
            collect_expression_tokens(module, interner, *index, text, out);
        }
        ExpressionKind::Literal(_) | ExpressionKind::This => {}
    }
}

fn find_top_level_symbol(
    module: &Module,
    interner: &SharedInterner,
    name: &str,
) -> Option<ResolvedSymbol> {
    for function in module.functions.values() {
        if let Some(function_name) = module.arena.resolve_symbol(interner, function.name) {
            if function_name == name {
                return Some(ResolvedSymbol {
                    name: function_name,
                    span: function.span,
                });
            }
        }
    }

    for class in module.classes.values() {
        let class_match = class.read(|class_def| {
            module
                .arena
                .resolve_symbol(interner, class_def.name)
                .filter(|class_name| class_name == name)
                .map(|class_name| ResolvedSymbol {
                    name: class_name,
                    span: class_def.span,
                })
        });
        if class_match.is_some() {
            return class_match;
        }
    }

    for stmt_id in &module.body {
        let Some(statement) = module.arena.get_statement(*stmt_id) else {
            continue;
        };
        if let StatementKind::Assign { name: symbol, .. } = statement.kind {
            if let Some(assign_name) = module.arena.resolve_symbol(interner, symbol) {
                if assign_name == name {
                    return Some(ResolvedSymbol {
                        name: assign_name,
                        span: statement.span,
                    });
                }
            }
        }
    }

    None
}

fn collect_declarations(
    module: &Module,
    interner: &SharedInterner,
    statement_ids: &[StmtId],
    out: &mut Vec<ResolvedSymbol>,
) {
    for stmt_id in statement_ids {
        let Some(statement) = module.arena.get_statement(*stmt_id) else {
            continue;
        };
        match &statement.kind {
            StatementKind::Assign { name, .. } => {
                if let Some(name) = module.arena.resolve_symbol(interner, *name) {
                    out.push(ResolvedSymbol {
                        name,
                        span: statement.span,
                    });
                }
            }
            StatementKind::For { variable, body, .. } => {
                if let Some(name) = module.arena.resolve_symbol(interner, *variable) {
                    out.push(ResolvedSymbol {
                        name,
                        span: statement.span,
                    });
                }
                collect_declarations(module, interner, &[*body], out);
            }
            StatementKind::If {
                then_body,
                else_body,
                ..
            } => {
                collect_declarations(module, interner, &[*then_body], out);
                if let Some(else_body) = else_body {
                    collect_declarations(module, interner, &[*else_body], out);
                }
            }
            StatementKind::While { body, .. } => {
                collect_declarations(module, interner, &[*body], out)
            }
            StatementKind::Block(items) => collect_declarations(module, interner, items, out),
            StatementKind::FunctionDefinition(function) => {
                if let Some(name) = module.arena.resolve_symbol(interner, function.name) {
                    out.push(ResolvedSymbol {
                        name,
                        span: function.span,
                    });
                }
                for param in &function.params {
                    if let Some(name) = module.arena.resolve_symbol(interner, param.name) {
                        out.push(ResolvedSymbol {
                            name,
                            span: param.span,
                        });
                    }
                }
                collect_declarations(module, interner, &[function.body], out);
            }
            StatementKind::ClassDefinition(class_def) => {
                if let Some(name) = module.arena.resolve_symbol(interner, class_def.name) {
                    out.push(ResolvedSymbol {
                        name,
                        span: class_def.span,
                    });
                }
                if let Some(MethodType::User(constructor)) = &class_def.constructor {
                    for param in &constructor.params {
                        if let Some(name) = module.arena.resolve_symbol(interner, param.name) {
                            out.push(ResolvedSymbol {
                                name,
                                span: param.span,
                            });
                        }
                    }
                    collect_declarations(module, interner, &[constructor.body], out);
                }
                for (_, _, method_type) in class_def.methods.values() {
                    if let MethodType::User(method) = method_type {
                        if let Some(name) = module.arena.resolve_symbol(interner, method.name) {
                            out.push(ResolvedSymbol {
                                name,
                                span: method.span,
                            });
                        }
                        for param in &method.params {
                            if let Some(name) = module.arena.resolve_symbol(interner, param.name) {
                                out.push(ResolvedSymbol {
                                    name,
                                    span: param.span,
                                });
                            }
                        }
                        collect_declarations(module, interner, &[method.body], out);
                    }
                }
            }
            StatementKind::Expression(_)
            | StatementKind::IndexAssign { .. }
            | StatementKind::PropertyAssign { .. }
            | StatementKind::Return(_)
            | StatementKind::NativeLibraryDefinition(_)
            | StatementKind::Empty => {}
        }
    }
}

fn collect_imports(module: &Module, interner: &SharedInterner) -> HashMap<String, String> {
    let mut imports = HashMap::new();
    for import in &module.imports {
        let Some(alias) = module.arena.resolve_symbol(interner, import.item.alias) else {
            continue;
        };
        let Some(path) = module.arena.resolve_symbol(interner, import.item.path) else {
            continue;
        };
        imports.insert(alias, path);
    }
    imports
}

fn resolve_import_path(
    current_file: &Path,
    import_path: &str,
    workspace_roots: &[PathBuf],
) -> Option<PathBuf> {
    let normalized = import_path.replace('/', std::path::MAIN_SEPARATOR_STR);
    let mut with_ext = PathBuf::from(normalized);
    if with_ext.extension().is_none() {
        with_ext.set_extension("goida");
    }

    let mut candidates = Vec::new();
    if with_ext.is_absolute() {
        candidates.push(with_ext);
    } else {
        if let Some(parent) = current_file.parent() {
            candidates.push(parent.join(&with_ext));
        }
        for root in workspace_roots {
            candidates.push(root.join(&with_ext));
        }
    }

    candidates.into_iter().find(|path| path.exists())
}

fn collect_goida_files(roots: &[PathBuf]) -> Vec<PathBuf> {
    let mut files = Vec::new();
    for root in roots {
        walk_goida_files(root, &mut files);
    }
    files
}

fn walk_goida_files(dir: &Path, files: &mut Vec<PathBuf>) {
    let entries = match fs::read_dir(dir) {
        Ok(v) => v,
        Err(_) => return,
    };
    for entry in entries {
        let Ok(entry) = entry else {
            continue;
        };
        let path = entry.path();
        if path.is_dir() {
            walk_goida_files(&path, files);
            continue;
        }
        if path
            .extension()
            .and_then(|v| v.to_str())
            .map(|ext| ext.eq_ignore_ascii_case("goida"))
            .unwrap_or(false)
        {
            files.push(path);
        }
    }
}

fn span_to_location(text: &str, uri: Url, span: Span) -> Option<Location> {
    let range = span.as_ariadne(text);
    let start = char_offset_to_position(text, range.start)?;
    let end = char_offset_to_position(text, range.end)?;
    Some(Location::new(uri, Range::new(start, end)))
}

fn compute_line_starts(text: &str) -> Vec<usize> {
    let mut starts = vec![0usize];
    let mut offset = 0usize;
    for ch in text.chars() {
        offset += 1;
        if ch == '\n' {
            starts.push(offset);
        }
    }
    starts
}

fn char_offset_to_position(text: &str, char_offset: usize) -> Option<Position> {
    let starts = compute_line_starts(text);
    let mut line = 0usize;
    for (idx, start) in starts.iter().enumerate() {
        if *start > char_offset {
            break;
        }
        line = idx;
    }
    let col = char_offset.saturating_sub(*starts.get(line)?);
    Some(Position::new(line as u32, col as u32))
}

fn position_to_char_offset(text: &str, position: Position) -> usize {
    let target_line = position.line as usize;
    let target_col = position.character as usize;
    let mut current_line = 0usize;
    let mut offset = 0usize;

    for line in text.split('\n') {
        if current_line == target_line {
            return offset + min(target_col, line.chars().count());
        }
        offset += line.chars().count() + 1;
        current_line += 1;
    }

    offset
}

fn find_identifier_at_char_offset(text: &str, char_offset: usize) -> Option<LocatedIdentifier> {
    let chars: Vec<char> = text.chars().collect();
    if chars.is_empty() {
        return None;
    }

    let mut pos = min(char_offset, chars.len().saturating_sub(1));
    if !is_identifier_continue(chars[pos]) && pos > 0 && is_identifier_continue(chars[pos - 1]) {
        pos -= 1;
    }
    if !is_identifier_continue(chars[pos]) {
        return None;
    }

    let mut start = pos;
    while start > 0 && is_identifier_continue(chars[start - 1]) {
        start -= 1;
    }
    if !is_identifier_start(chars[start]) {
        return None;
    }

    let mut end = pos + 1;
    while end < chars.len() && is_identifier_continue(chars[end]) {
        end += 1;
    }

    let name: String = chars[start..end].iter().collect();
    let module_alias = if start >= 2 && chars[start - 1] == '.' {
        let alias_end = start - 1;
        let mut alias_start = alias_end;
        while alias_start > 0 && is_identifier_continue(chars[alias_start - 1]) {
            alias_start -= 1;
        }
        if alias_start < alias_end && is_identifier_start(chars[alias_start]) {
            Some(chars[alias_start..alias_end].iter().collect())
        } else {
            None
        }
    } else {
        None
    };

    Some(LocatedIdentifier {
        name,
        start_char: start,
        module_alias,
    })
}

fn is_identifier_start(ch: char) -> bool {
    ch == '_' || ch.is_alphabetic()
}

fn is_identifier_continue(ch: char) -> bool {
    ch == '_' || ch.is_alphanumeric()
}

fn find_name_char_range(text: &str, span: Span, name: &str) -> Option<std::ops::Range<usize>> {
    let start = span.start as usize;
    let end = min(span.end as usize, text.len());
    if start >= end || !text.is_char_boundary(start) || !text.is_char_boundary(end) {
        return None;
    }

    let segment = &text[start..end];
    let local_match = segment.find(name)?;
    let absolute_start = start + local_match;
    let absolute_end = absolute_start + name.len();

    if !text.is_char_boundary(absolute_start) || !text.is_char_boundary(absolute_end) {
        return None;
    }

    let char_start = text[..absolute_start].chars().count();
    let char_end = text[..absolute_end].chars().count();
    Some(char_start..char_end)
}

fn char_range_to_semantic_token(
    text: &str,
    start_char: usize,
    end_char: usize,
    token_type: u32,
    declaration: bool,
) -> Option<SemanticTokenAbsolute> {
    if start_char >= end_char {
        return None;
    }
    let start = char_offset_to_position(text, start_char)?;
    let end = char_offset_to_position(text, end_char)?;
    if start.line != end.line {
        return None;
    }

    Some(SemanticTokenAbsolute {
        line: start.line,
        start: start.character,
        length: end.character.saturating_sub(start.character),
        token_type,
        token_modifiers_bitset: if declaration { 1 } else { 0 },
    })
}

fn encode_semantic_tokens(mut tokens: Vec<SemanticTokenAbsolute>) -> Vec<SemanticToken> {
    tokens.sort_by_key(|tok| (tok.line, tok.start, tok.length, tok.token_type));

    let mut encoded = Vec::with_capacity(tokens.len());
    let mut prev_line = 0u32;
    let mut prev_start = 0u32;

    for (idx, tok) in tokens.into_iter().enumerate() {
        let (delta_line, delta_start) = if idx == 0 {
            (tok.line, tok.start)
        } else if tok.line == prev_line {
            (0, tok.start.saturating_sub(prev_start))
        } else {
            (tok.line.saturating_sub(prev_line), tok.start)
        };

        encoded.push(SemanticToken {
            delta_line,
            delta_start,
            length: tok.length,
            token_type: tok.token_type,
            token_modifiers_bitset: tok.token_modifiers_bitset,
        });

        prev_line = tok.line;
        prev_start = tok.start;
    }

    encoded
}

#[tokio::main]
async fn main() {
    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();

    let interner = SharedInterner::new(StringInterner::new());
    let interpreter = Arc::new(RwLock::new(Interpreter::new(interner)));
    let state = Arc::new(RwLock::new(ServerState::default()));

    let (service, socket) = LspService::new(|client| Backend {
        client,
        interpreter,
        state,
    });
    Server::new(stdin, stdout, socket).serve(service).await;
}
