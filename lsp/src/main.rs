use goida_core::interpreter::prelude::{Interpreter, SharedInterner};
use goida_core::parser::prelude::{ParseError, Parser};
use goida_core::traits::prelude::CoreOperations;
use std::sync::Arc;
use string_interner::StringInterner;
use tokio::sync::RwLock;
use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LanguageServer, LspService, Server};

struct Backend {
    client: Client,
    interpreter: Arc<RwLock<Interpreter>>,
}

#[tower_lsp::async_trait]
impl LanguageServer for Backend {
    async fn initialize(&self, _: InitializeParams) -> Result<InitializeResult> {
        Ok(InitializeResult {
            capabilities: ServerCapabilities {
                text_document_sync: Some(TextDocumentSyncCapability::Kind(
                    TextDocumentSyncKind::FULL,
                )),
                ..Default::default()
            },
            ..Default::default()
        })
    }

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        self.validate(params.text_document.uri, params.text_document.text)
            .await;
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        if let Some(change) = params.content_changes.into_iter().next() {
            self.validate(params.text_document.uri, change.text).await;
        }
    }

    async fn shutdown(&self) -> Result<()> {
        Ok(())
    }
}

impl Backend {
    async fn validate(&self, uri: Url, text: String) {
        let path_buf = uri.to_file_path().expect("Invalid file URI");
        let filename = path_buf.to_str().expect("Path is not valid UTF-8");

        let intp = self.interpreter.read().await;

        let parser = Parser::new(intp.interner.clone(), filename, path_buf.clone());

        let mut diagnostics = Vec::new();

        if let Err(err) = parser.parse(&text) {
            let (msg, err_data) = match err {
                ParseError::TypeError(e) => ("Ошибка типов", e),
                ParseError::InvalidSyntax(e) => ("Ошибка синтаксиса", e),
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

#[tokio::main]
async fn main() {
    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();

    let interner = SharedInterner::new(StringInterner::new());
    let interpreter = Arc::new(RwLock::new(Interpreter::new(interner)));

    let (service, socket) = LspService::new(|client| Backend {
        client,
        interpreter,
    });
    Server::new(stdin, stdout, socket).serve(service).await;
}
