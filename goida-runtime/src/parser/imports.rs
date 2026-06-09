use crate::ast::prelude::*;
use crate::import_paths::resolve_import_path;
use crate::interpreter::prelude::Module;
use crate::parser::prelude::{ParseError, Parser as ParserTrait};
use crate::parser::structs::ModuleLoadState;
use std::sync::Arc;
use string_interner::DefaultSymbol as Symbol;

impl ParserTrait {
    pub(crate) fn register_imported_type_aliases(
        &mut self,
        alias_symbol: Symbol,
        module_symbol: Symbol,
    ) {
        let alias_name = self
            .interner
            .read(|i| i.resolve(alias_symbol).unwrap_or_default().to_string());

        let qualified_type_names = self
            .module
            .modules
            .get(&module_symbol)
            .map(|module| {
                module
                    .classes
                    .keys()
                    .filter_map(|class_symbol| {
                        self.interner.read(|i| {
                            i.resolve(*class_symbol)
                                .map(|class_name| format!("{alias_name}.{class_name}"))
                        })
                    })
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();

        for qualified_name in qualified_type_names {
            self.module
                .arena
                .register_custom_type(&self.interner, qualified_name.as_str());
        }
    }

    pub(crate) fn parse_and_register_import(
        &mut self,
        import_path_symbol: Symbol,
        span: Span,
    ) -> Result<Symbol, ParseError> {
        let path_str = self.interner.read(|i| {
            i.resolve(import_path_symbol)
                .unwrap_or_default()
                .to_string()
        });
        let full_path = resolve_import_path(&self.module.path, &path_str);

        full_path
            .file_stem()
            .and_then(|stem| stem.to_str())
            .ok_or_else(|| {
                ParseError::ImportError(ErrorData::new(
                    span,
                    format!("Invalid import path: {}", full_path.display()),
                ))
            })?;

        let normalized_path = full_path.canonicalize().unwrap_or(full_path.clone());
        let normalized_full_path = normalized_path.to_string_lossy().to_string();
        let module_symbol = self
            .interner
            .write(|i| i.get_or_intern(normalized_full_path.as_str()));

        if self.module.modules.contains_key(&module_symbol) {
            return Ok(module_symbol);
        }

        let cached = self.module_loader.read(|loader| {
            loader
                .modules
                .get(&normalized_path)
                .map(|state| match state {
                    ModuleLoadState::Loading => Err(format!(
                        "Cyclic module import detected while loading {}",
                        normalized_path.display()
                    )),
                    ModuleLoadState::Loaded(module) => Ok(module.as_ref().clone()),
                    ModuleLoadState::Failed(message) => Err(message.clone()),
                })
        });
        if let Some(result) = cached {
            let module =
                result.map_err(|message| ParseError::ImportError(ErrorData::new(span, message)))?;
            self.register_loaded_module(module_symbol, module);
            return Ok(module_symbol);
        }

        self.module_loader.write(|loader| {
            loader
                .modules
                .insert(normalized_path.clone(), ModuleLoadState::Loading);
        });

        let code = std::fs::read_to_string(&full_path).map_err(|error| {
            let message = format!("Cannot read module {}: {}", full_path.display(), error);
            self.cache_failed_module(normalized_path.clone(), message.clone());
            ParseError::ImportError(ErrorData::new(span, message))
        })?;

        let sub_parser = ParserTrait::with_module_loader(
            self.interner.clone(),
            normalized_full_path.as_str(),
            full_path,
            self.module_loader.clone(),
        );
        let module = match sub_parser.parse(&code) {
            Ok(module) => module,
            Err(error) => {
                self.cache_failed_module(normalized_path, parse_error_message(&error));
                return Err(error);
            }
        };

        self.module_loader.write(|loader| {
            loader.modules.insert(
                normalized_path,
                ModuleLoadState::Loaded(Arc::new(module.clone())),
            );
        });
        self.register_loaded_module(module_symbol, module);
        Ok(module_symbol)
    }

    fn cache_failed_module(&self, path: std::path::PathBuf, message: String) {
        self.module_loader.write(|loader| {
            loader
                .modules
                .insert(path, ModuleLoadState::Failed(message));
        });
    }

    fn register_loaded_module(&mut self, module_symbol: Symbol, module: Module) {
        for class_name_symbol in module.classes.keys() {
            let class_name = self.interner.read(|i| {
                i.resolve(*class_name_symbol)
                    .unwrap_or_default()
                    .to_string()
            });
            self.module
                .arena
                .register_custom_type(&self.interner, class_name.as_str());
        }

        self.module.modules.insert(module_symbol, module);
    }
}

fn parse_error_message(error: &ParseError) -> String {
    match error {
        ParseError::TypeError(data)
        | ParseError::InvalidSyntax(data)
        | ParseError::ImportError(data) => data.message.clone(),
    }
}
