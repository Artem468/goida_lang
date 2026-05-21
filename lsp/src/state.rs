use crate::document::Document;
use goida_core::interpreter::prelude::Module;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tower_lsp::lsp_types::Url;

#[derive(Clone)]
pub(crate) struct CachedModule {
    pub(crate) document: Document,
    pub(crate) module: Arc<Module>,
}

#[derive(Default)]
pub(crate) struct ServerState {
    pub(crate) documents: HashMap<Url, Document>,
    pub(crate) modules: HashMap<PathBuf, CachedModule>,
    pub(crate) workspace_roots: Vec<PathBuf>,
}
