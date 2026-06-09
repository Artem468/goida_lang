use crate::ast::prelude::ErrorData;
use crate::interpreter::prelude::{Module, SharedInterner};
use crate::shared::SharedMut;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// Keyword language used when rendering source code.
pub enum FormatLanguage {
    English,
    Russian,
}

impl FormatLanguage {
    pub(crate) fn select(self, english: &'static str, russian: &'static str) -> &'static str {
        match self {
            Self::English => english,
            Self::Russian => russian,
        }
    }

    /// Detects the dominant keyword language in source, defaulting to English.
    pub fn detect(source: &str) -> Self {
        const ENGLISH: &[&str] = &[
            "import",
            "from",
            "function",
            "library",
            "variable",
            "class",
            "constructor",
            "public",
            "private",
            "static",
            "const",
            "if",
            "else",
            "while",
            "for",
            "thread",
            "try",
            "catch",
            "raise",
            "as",
            "new",
            "return",
            "and",
            "or",
            "true",
            "false",
            "void",
            "macro",
        ];
        const RUSSIAN: &[&str] = &[
            "подключить",
            "из",
            "функция",
            "библиотека",
            "переменная",
            "класс",
            "конструктор",
            "публичный",
            "приватный",
            "статичный",
            "константа",
            "если",
            "иначе",
            "пока",
            "для",
            "поток",
            "попробовать",
            "перехватить",
            "выбросить",
            "как",
            "новый",
            "вернуть",
            "и",
            "или",
            "истина",
            "ложь",
            "пустота",
            "макрос",
        ];

        let mut english = 0;
        let mut russian = 0;
        for word in source.split(|ch: char| !ch.is_alphanumeric() && ch != '_') {
            english += usize::from(ENGLISH.contains(&word));
            russian += usize::from(RUSSIAN.contains(&word));
        }

        if russian > english {
            Self::Russian
        } else {
            Self::English
        }
    }
}

#[derive(Debug)]
/// Stateful parser for a single module.
pub struct Parser {
    /// Module under construction.
    pub module: Module,
    pub(crate) interner: SharedInterner,
    pub(crate) module_loader: SharedMut<ModuleLoader>,
}

#[derive(Debug, Default)]
pub(crate) struct ModuleLoader {
    pub(crate) modules: HashMap<PathBuf, ModuleLoadState>,
}

#[derive(Debug)]
pub(crate) enum ModuleLoadState {
    Loading,
    Loaded(Arc<Module>),
    Failed(String),
}

#[derive(Debug)]
/// Errors produced while parsing or validating source.
pub enum ParseError {
    TypeError(ErrorData),
    InvalidSyntax(ErrorData),
    ImportError(ErrorData),
}
