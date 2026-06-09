mod backend;
mod completion;
mod diagnostics;
mod document;
mod semantic;
mod state;
mod symbols;
mod workspace;

use backend::Backend;
use goida_model::new_interner;
use state::ServerState;
use std::sync::Arc;
use tokio::sync::RwLock;
use tower_lsp::{LspService, Server};

#[tokio::main]
async fn main() {
    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();

    let interner = new_interner();
    let state = Arc::new(RwLock::new(ServerState::default()));

    let (service, socket) = LspService::new(|client| Backend {
        client,
        interner,
        state,
    });
    Server::new(stdin, stdout, socket).serve(service).await;
}
