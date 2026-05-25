use crate::ast::prelude::*;
use crate::import_paths::resolve_import_path;
use crate::parser::prelude::{ParseError, Parser as ParserTrait};
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

        let _file_stem = full_path
            .file_stem()
            .and_then(|s| s.to_str())
            .ok_or_else(|| {
                ParseError::ImportError(ErrorData::new(
                    span,
                    format!("Неверный путь: {}", full_path.display()),
                ))
            })?;

        let normalized_full_path = full_path
            .canonicalize()
            .unwrap_or(full_path.clone())
            .to_string_lossy()
            .to_string();
        let module_symbol = self
            .interner
            .write(|i| i.get_or_intern(normalized_full_path.as_str()));

        if self.module.modules.contains_key(&module_symbol) {
            return Ok(module_symbol);
        }

        let code = std::fs::read_to_string(&full_path).map_err(|e| {
            ParseError::ImportError(ErrorData::new(
                span,
                format!("Не нашел файл {}: {}", full_path.display(), e),
            ))
        })?;

        let sub_parser = ParserTrait::new(
            self.interner.clone(),
            normalized_full_path.as_str(),
            full_path.clone(),
        );
        let new_module = sub_parser.parse(&code)?;

        for class_name_symbol in new_module.classes.keys() {
            let class_name = self.interner.read(|i| {
                i.resolve(*class_name_symbol)
                    .unwrap_or_default()
                    .to_string()
            });
            self.module
                .arena
                .register_custom_type(&self.interner, class_name.as_str());
        }

        self.module.modules.insert(module_symbol, new_module);

        Ok(module_symbol)
    }
}
