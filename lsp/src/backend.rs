use crate::completion::{completion_items, module_member_completion_items};
use crate::diagnostics::collect_lsp_diagnostics;
use crate::document::{find_identifier_at_char_offset, Document};
use crate::semantic::{
    collect_semantic_tokens, encode_semantic_tokens, TOKEN_MODIFIERS, TOKEN_TYPES,
};
use crate::state::{CachedModule, ServerState};
use crate::symbols::{
    collect_declarations, collect_imports, find_top_level_symbol, ResolvedSymbol,
};
use crate::workspace::{collect_goida_files, resolve_import_path};
use goida_model::SharedInterner;
use goida_runtime::interpreter::prelude::Module;
use goida_runtime::parser::prelude::{ParseError, Parser};
use goida_syntax::ast::prelude::Span;
use std::fs;
use std::path::Path;
use std::sync::Arc;
use tokio::sync::RwLock;
use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LanguageServer};

pub(crate) struct Backend {
    pub(crate) client: Client,
    pub(crate) interner: SharedInterner,
    pub(crate) state: Arc<RwLock<ServerState>>,
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
                text_document_sync: Some(text_document_sync_capability()),
                definition_provider: Some(OneOf::Left(true)),
                completion_provider: Some(CompletionOptions {
                    resolve_provider: Some(false),
                    trigger_characters: Some(vec![".".into()]),
                    ..Default::default()
                }),
                document_formatting_provider: Some(OneOf::Left(true)),
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
        let uri = params.text_document.uri;
        let document = Document::new(params.text_document.text);
        self.state
            .write()
            .await
            .documents
            .insert(uri.clone(), document.clone());
        self.validate(uri, &document).await;
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        if let Some(change) = params.content_changes.into_iter().next() {
            let uri = params.text_document.uri;
            let document = Document::new(change.text);
            self.state
                .write()
                .await
                .documents
                .insert(uri.clone(), document.clone());
            self.validate(uri, &document).await;
        }
    }

    async fn did_save(&self, params: DidSaveTextDocumentParams) {
        let uri = params.text_document.uri;
        let text = match params.text {
            Some(text) => Some(text),
            None => uri
                .to_file_path()
                .ok()
                .and_then(|path| fs::read_to_string(path).ok()),
        };

        if let Some(text) = text {
            let document = Document::new(text);
            self.state
                .write()
                .await
                .documents
                .insert(uri.clone(), document.clone());
            self.validate(uri, &document).await;
        } else {
            self.client.publish_diagnostics(uri, Vec::new(), None).await;
        }
    }

    async fn did_close(&self, params: DidCloseTextDocumentParams) {
        let mut state = self.state.write().await;
        state.documents.remove(&params.text_document.uri);
        if let Ok(path) = params.text_document.uri.to_file_path() {
            state.modules.remove(&path);
        }
        drop(state);

        self.client
            .publish_diagnostics(params.text_document.uri, Vec::new(), None)
            .await;
    }

    async fn did_change_watched_files(&self, params: DidChangeWatchedFilesParams) {
        for event in params.changes {
            let uri = event.uri;
            let Ok(path) = uri.to_file_path() else {
                self.client.publish_diagnostics(uri, Vec::new(), None).await;
                continue;
            };

            if event.typ == FileChangeType::DELETED {
                self.state.write().await.modules.remove(&path);
                self.client.publish_diagnostics(uri, Vec::new(), None).await;
                continue;
            }

            let document = {
                let state = self.state.read().await;
                state.documents.get(&uri).cloned()
            }
            .or_else(|| fs::read_to_string(&path).ok().map(Document::new));

            if let Some(document) = document {
                self.validate(uri, &document).await;
            } else {
                self.state.write().await.modules.remove(&path);
                self.client.publish_diagnostics(uri, Vec::new(), None).await;
            }
        }
    }

    async fn semantic_tokens_full(
        &self,
        params: SemanticTokensParams,
    ) -> Result<Option<SemanticTokensResult>> {
        let uri = params.text_document.uri;
        let Some(path) = uri.to_file_path().ok() else {
            return Ok(None);
        };
        let Some(cached) = self.cached_module_for_uri(&uri, &path).await else {
            return Ok(None);
        };

        let tokens = collect_semantic_tokens(
            &cached.module,
            &self.interner,
            cached.document.text(),
            cached.document.line_starts(),
        );
        let encoded = encode_semantic_tokens(tokens);

        Ok(Some(SemanticTokensResult::Tokens(SemanticTokens {
            result_id: None,
            data: encoded,
        })))
    }

    async fn completion(&self, params: CompletionParams) -> Result<Option<CompletionResponse>> {
        let uri = params.text_document_position.text_document.uri;
        let position = params.text_document_position.position;
        let Some(path) = uri.to_file_path().ok() else {
            return Ok(None);
        };

        let document = {
            let state = self.state.read().await;
            state.documents.get(&uri).cloned()
        }
        .or_else(|| fs::read_to_string(&path).ok().map(Document::new));
        let Some(document) = document else {
            return Ok(None);
        };
        let cached = self.cached_module_for_uri(&uri, &path).await;

        if let (Some(cached), Some(alias)) = (
            cached.as_ref(),
            module_alias_before_completion(
                document.text(),
                document.position_to_char_offset(position),
            ),
        ) {
            if let Some(target) = self
                .resolve_import_module(&cached.module, &path, &alias)
                .await
            {
                return Ok(Some(CompletionResponse::Array(
                    module_member_completion_items(&target.module, &self.interner),
                )));
            }
        }

        Ok(Some(CompletionResponse::Array(completion_items(
            cached.as_ref().map(|cached| cached.module.as_ref()),
            &self.interner,
        ))))
    }

    async fn formatting(&self, params: DocumentFormattingParams) -> Result<Option<Vec<TextEdit>>> {
        let uri = params.text_document.uri;
        let document = {
            let state = self.state.read().await;
            state.documents.get(&uri).cloned()
        }
        .or_else(|| {
            uri.to_file_path()
                .ok()
                .and_then(|path| fs::read_to_string(path).ok())
                .map(Document::new)
        });
        let Some(document) = document else {
            return Ok(None);
        };

        let end = document
            .char_offset_to_position(document.text().chars().count())
            .unwrap_or_else(|| Position::new(0, 0));
        Ok(Some(vec![TextEdit {
            range: Range::new(Position::new(0, 0), end),
            new_text: goida_syntax::formatter::format_source(document.text()),
        }]))
    }

    async fn goto_definition(
        &self,
        params: GotoDefinitionParams,
    ) -> Result<Option<GotoDefinitionResponse>> {
        let uri = params.text_document_position_params.text_document.uri;
        let position = params.text_document_position_params.position;

        let Some(current_path) = uri.to_file_path().ok() else {
            return Ok(None);
        };
        let Some(cached) = self.cached_module_for_uri(&uri, &current_path).await else {
            return Ok(None);
        };

        let char_offset = cached.document.position_to_char_offset(position);
        let Some(ident) = find_identifier_at_char_offset(cached.document.text(), char_offset)
        else {
            return Ok(None);
        };

        if let Some(alias) = ident.module_alias.as_ref() {
            if let Some(location) = self
                .resolve_import_member_definition(&cached.module, &current_path, alias, &ident.name)
                .await
            {
                return Ok(Some(GotoDefinitionResponse::Scalar(location)));
            }
        }

        if let Some(location) =
            self.resolve_local_definition(&cached, &uri, &ident.name, ident.start_char)
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
    async fn validate(&self, uri: Url, document: &Document) {
        let Ok(path) = uri.to_file_path() else {
            self.client.publish_diagnostics(uri, Vec::new(), None).await;
            return;
        };

        let mut diagnostics = Vec::new();
        match self.parse_and_cache_module(&path, document).await {
            Ok(cached) => {
                diagnostics.extend(collect_lsp_diagnostics(
                    &cached.module,
                    &self.interner,
                    document.text(),
                    document.line_starts(),
                ));
            }
            Err(err) => {
                let (msg, err_data) = match err {
                    ParseError::TypeError(e) => ("Ошибка типов", e),
                    ParseError::InvalidSyntax(e) => ("Некорректный синтаксис", e),
                    ParseError::ImportError(e) => ("Ошибка импортов", e),
                };

                let span = err_data.location.as_ariadne(document.text());
                let start = document
                    .char_offset_to_position(span.start)
                    .unwrap_or_else(|| Position::new(0, 0));
                let end = document.char_offset_to_position(span.end).unwrap_or(start);

                diagnostics.push(Diagnostic {
                    range: Range::new(start, end),
                    message: format!("{}: {}", msg, err_data.message),
                    severity: Some(DiagnosticSeverity::ERROR),
                    source: Some("goida-lsp".into()),
                    ..Default::default()
                });
            }
        }

        self.client
            .publish_diagnostics(uri, diagnostics, None)
            .await;
    }

    async fn cached_module_for_uri(&self, uri: &Url, path: &Path) -> Option<CachedModule> {
        let state = self.state.read().await;
        let open_document = state.documents.get(uri).cloned();
        let cached = state.modules.get(path).cloned();
        drop(state);

        if open_document.is_some() {
            if let Some(cached) = cached {
                return Some(cached);
            }
        }

        if let Some(document) = open_document {
            return self.parse_and_cache_module(path, &document).await.ok();
        }
        self.cached_module_for_path(path).await
    }

    async fn cached_module_for_path(&self, path: &Path) -> Option<CachedModule> {
        if let Some(cached) = self.state.read().await.modules.get(path).cloned() {
            return Some(cached);
        }

        let text = fs::read_to_string(path).ok()?;
        let document = Document::new(text);
        self.parse_and_cache_module(path, &document).await.ok()
    }

    async fn parse_and_cache_module(
        &self,
        path: &Path,
        document: &Document,
    ) -> std::result::Result<CachedModule, ParseError> {
        let filename = path.to_string_lossy();
        let module = match Parser::new(self.interner.clone(), &filename, path.to_path_buf())
            .parse_syntax(document.text())
        {
            Ok(module) => module,
            Err(err) => {
                self.state.write().await.modules.remove(path);
                return Err(err);
            }
        };
        let cached = CachedModule {
            document: document.clone(),
            module: Arc::new(module),
        };
        self.state
            .write()
            .await
            .modules
            .insert(path.to_path_buf(), cached.clone());
        Ok(cached)
    }

    async fn resolve_workspace_definition(
        &self,
        current_file: &Path,
        symbol_name: &str,
    ) -> Option<Location> {
        let roots = self.state.read().await.workspace_roots.clone();
        let files = collect_goida_files(&roots);

        for file in files {
            if file == current_file {
                continue;
            }

            let cached = match self.cached_module_for_path(&file).await {
                Some(v) => v,
                None => continue,
            };

            if let Some(symbol) = find_top_level_symbol(&cached.module, &self.interner, symbol_name)
            {
                if let Ok(uri) = Url::from_file_path(&file) {
                    if let Some(location) = span_to_location(&cached.document, uri, symbol.span) {
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
        let imports = collect_imports(module, &self.interner);
        let import_path = imports.get(module_alias)?;

        let workspace_roots = self.state.read().await.workspace_roots.clone();
        let target_file = resolve_import_path(current_file, import_path, &workspace_roots)?;
        let cached = self.cached_module_for_path(&target_file).await?;
        let symbol = find_top_level_symbol(&cached.module, &self.interner, member_name)?;

        let uri = Url::from_file_path(target_file).ok()?;
        span_to_location(&cached.document, uri, symbol.span)
    }

    async fn resolve_import_module(
        &self,
        module: &Module,
        current_file: &Path,
        module_alias: &str,
    ) -> Option<CachedModule> {
        let imports = collect_imports(module, &self.interner);
        let import_path = imports.get(module_alias)?;

        let workspace_roots = self.state.read().await.workspace_roots.clone();
        let target_file = resolve_import_path(current_file, import_path, &workspace_roots)?;
        self.cached_module_for_path(&target_file).await
    }

    fn resolve_local_definition(
        &self,
        cached: &CachedModule,
        uri: &Url,
        symbol_name: &str,
        usage_start_char: usize,
    ) -> Option<Location> {
        let mut declarations = Vec::new();
        collect_declarations(
            &cached.module,
            &self.interner,
            &cached.module.body,
            &mut declarations,
        );
        declarations.retain(|decl| decl.name == symbol_name);

        let mut best: Option<ResolvedSymbol> = None;
        for decl in declarations {
            let decl_start = decl.span.as_ariadne(cached.document.text()).start;
            if decl_start > usage_start_char {
                continue;
            }
            match &best {
                Some(current) => {
                    if current.span.as_ariadne(cached.document.text()).start < decl_start {
                        best = Some(decl);
                    }
                }
                None => best = Some(decl),
            }
        }

        if best.is_none() {
            if let Some(top_level) =
                find_top_level_symbol(&cached.module, &self.interner, symbol_name)
            {
                return span_to_location(&cached.document, uri.clone(), top_level.span);
            }
            return None;
        }

        span_to_location(&cached.document, uri.clone(), best?.span)
    }
}

fn span_to_location(document: &Document, uri: Url, span: Span) -> Option<Location> {
    let range = span.as_ariadne(document.text());
    let start = document.char_offset_to_position(range.start)?;
    let end = document.char_offset_to_position(range.end)?;
    Some(Location::new(uri, Range::new(start, end)))
}

fn module_alias_before_completion(text: &str, char_offset: usize) -> Option<String> {
    let chars = text.chars().collect::<Vec<_>>();
    let mut pos = char_offset.min(chars.len());

    while pos > 0 && is_identifier_continue(chars[pos - 1]) {
        pos -= 1;
    }
    if pos == 0 || chars[pos - 1] != '.' {
        return None;
    }

    let alias_end = pos - 1;
    let mut alias_start = alias_end;
    while alias_start > 0 && is_identifier_continue(chars[alias_start - 1]) {
        alias_start -= 1;
    }
    if alias_start == alias_end || !is_identifier_start(chars[alias_start]) {
        return None;
    }

    Some(chars[alias_start..alias_end].iter().collect())
}

fn is_identifier_start(ch: char) -> bool {
    ch == '_' || ch.is_alphabetic()
}

fn is_identifier_continue(ch: char) -> bool {
    ch == '_' || ch.is_alphanumeric()
}

fn text_document_sync_capability() -> TextDocumentSyncCapability {
    TextDocumentSyncCapability::Options(TextDocumentSyncOptions {
        open_close: Some(true),
        change: Some(TextDocumentSyncKind::FULL),
        save: Some(TextDocumentSyncSaveOptions::SaveOptions(SaveOptions {
            include_text: Some(true),
        })),
        ..Default::default()
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sync_capability_requests_full_text_on_save() {
        let TextDocumentSyncCapability::Options(options) = text_document_sync_capability() else {
            panic!("sync capability should use detailed options");
        };

        assert_eq!(options.open_close, Some(true));
        assert_eq!(options.change, Some(TextDocumentSyncKind::FULL));
        let Some(TextDocumentSyncSaveOptions::SaveOptions(save)) = options.save else {
            panic!("sync capability should include save options");
        };
        assert_eq!(save.include_text, Some(true));
    }
}
