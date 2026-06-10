use crate::ast::prelude::*;
use crate::builtins::registry::{BuiltinParserTarget, BUILTINS};
use crate::interpreter::prelude::{Module, SharedInterner};
use crate::parser::formatter::format_program;
use crate::parser::grammar;
use crate::parser::lexer::{lex, LexicalError, Token};
use crate::parser::prelude::{FormatLanguage, ParseError, Parser as ParserTrait};
use crate::parser::structs::ModuleLoader;
use crate::shared::SharedMut;
use lalrpop_util::ParseError as LalrpopParseError;
use std::path::PathBuf;

impl ParserTrait {
    pub fn new(interner: SharedInterner, name: &str, path: PathBuf) -> Self {
        Self::with_module_loader(
            interner,
            name,
            path,
            SharedMut::new(ModuleLoader::default()),
        )
    }

    pub(crate) fn with_module_loader(
        interner: SharedInterner,
        name: &str,
        path: PathBuf,
        module_loader: SharedMut<ModuleLoader>,
    ) -> Self {
        Self {
            module: Module::new(&interner, name, path),
            interner,
            module_loader,
        }
    }

    pub fn parse(mut self, code: &str) -> Result<Module, ParseError> {
        self.install_builtins();

        self.parse_into_module(code)?;
        self.validate_module_names()?;
        self.module.arena.optimize_all(&self.interner);
        self.lower_module()?;
        Ok(self.module)
    }

    /// Parses and optimizes source without semantic validation or compilation.
    ///
    /// This path is intended for editors and other tools that must keep a
    /// partial module available while the user is typing.
    pub fn parse_syntax(mut self, code: &str) -> Result<Module, ParseError> {
        self.install_builtins();
        self.parse_into_module(code)?;
        self.module.arena.optimize_all(&self.interner);
        Ok(self.module)
    }

    /// Compatibility alias for callers that previously requested an
    /// unvalidated module.
    pub fn parse_unvalidated(self, code: &str) -> Result<Module, ParseError> {
        self.parse_syntax(code)
    }

    pub fn macro_expansion_preview(&self, code: &str) -> Result<String, ParseError> {
        let syntax = self.parse_source_ast(code)?;
        let syntax = self.expand_macros(syntax)?;
        Ok(format_program(&syntax, FormatLanguage::English))
    }

    pub fn format_source_ast(&self, code: &str) -> Result<String, ParseError> {
        self.format_source_ast_with_language(code, FormatLanguage::English)
    }

    pub fn format_source_ast_with_language(
        &self,
        code: &str,
        language: FormatLanguage,
    ) -> Result<String, ParseError> {
        self.parse_source_ast(code)
            .map(|syntax| format_program(&syntax, language))
    }

    fn install_builtins(&mut self) {
        BUILTINS
            .install(&mut BuiltinParserTarget {
                module: &mut self.module,
                interner: &self.interner,
            })
            .unwrap();
    }

    fn parse_into_module(&mut self, code: &str) -> Result<(), ParseError> {
        let syntax = self.parse_source_ast(code)?;
        let syntax = self.expand_macros(syntax)?;
        self.build_program(syntax)
    }

    fn parse_source_ast(&self, code: &str) -> Result<crate::parser::syntax::Program, ParseError> {
        let mut syntax = grammar::ProgramParser::new()
            .parse(lex(code))
            .map_err(|err| self.convert_parse_error(code, err))?;
        syntax.comments = collect_comments(code);
        Ok(syntax)
    }

    fn lower_module(&mut self) -> Result<(), ParseError> {
        let mut hir = crate::hir::Lowerer::lower(&self.module);
        crate::hir::TypeChecker::check(&mut hir)
            .map_err(|error| ParseError::TypeError(error.data))?;
        let bytecode = crate::bytecode::Compiler::compile(&self.module, &hir);
        self.module.hir = hir;
        self.module.bytecode = bytecode;
        self.module.initialize_global_slots();
        Ok(())
    }

    fn convert_parse_error(
        &self,
        code: &str,
        err: LalrpopParseError<usize, Token, LexicalError>,
    ) -> ParseError {
        match err {
            LalrpopParseError::InvalidToken { location } => {
                let (start, end) = token_range_at(code, location);
                ParseError::InvalidSyntax(ErrorData::new(
                    Span::new(start, end, self.module.name),
                    "Некорректный токен".into(),
                ))
            }
            LalrpopParseError::UnrecognizedEof { location, expected } => {
                ParseError::InvalidSyntax(ErrorData::new(
                    Span::new(location, location, self.module.name),
                    format_expected("Неожиданный конец файла", expected),
                ))
            }
            LalrpopParseError::UnrecognizedToken { token, expected } => {
                let (start, found, end) = token;
                ParseError::InvalidSyntax(ErrorData::new(
                    Span::new(start, end, self.module.name),
                    format_expected(
                        format!("Неожиданный токен {}", token_name(&found)),
                        expected,
                    ),
                ))
            }
            LalrpopParseError::ExtraToken { token } => {
                let (start, found, end) = token;
                ParseError::InvalidSyntax(ErrorData::new(
                    Span::new(start, end, self.module.name),
                    format!("Лишний токен {}", token_name(&found)),
                ))
            }
            LalrpopParseError::User { error } => ParseError::InvalidSyntax(ErrorData::new(
                Span::new(error.span.start, error.span.end, self.module.name),
                error.message,
            )),
        }
    }
}

pub(super) fn collect_comments(code: &str) -> Vec<crate::parser::syntax::Comment> {
    let mut comments = Vec::new();
    let mut in_string = false;
    let mut escaped = false;
    let bytes = code.as_bytes();
    let mut index = 0;

    while index + 1 < bytes.len() {
        match bytes[index] {
            b'\\' if in_string && !escaped => escaped = true,
            b'"' if !escaped => in_string = !in_string,
            b'/' if !in_string && bytes[index + 1] == b'/' => {
                let start = index;
                let end = code[index..]
                    .find('\n')
                    .map(|offset| index + offset)
                    .unwrap_or(code.len());
                comments.push(crate::parser::syntax::Comment {
                    text: code[start + 2..end].trim().to_string(),
                    span: start..end,
                });
                index = end;
                continue;
            }
            _ => escaped = false,
        }
        index += 1;
    }
    comments
}

fn token_range_at(code: &str, location: usize) -> (usize, usize) {
    let start = previous_char_boundary(code, location.min(code.len()));
    let mut end = next_char_boundary(code, location.min(code.len()));
    if end == start && end < code.len() {
        end = next_char_boundary(code, end + 1);
    }
    (start, end)
}

fn previous_char_boundary(s: &str, mut index: usize) -> usize {
    index = index.min(s.len());
    while index > 0 && !s.is_char_boundary(index) {
        index -= 1;
    }
    index
}

fn next_char_boundary(s: &str, mut index: usize) -> usize {
    index = index.min(s.len());
    while index < s.len() && !s.is_char_boundary(index) {
        index += 1;
    }
    index
}

fn format_expected(prefix: impl Into<String>, expected: Vec<String>) -> String {
    if expected.is_empty() {
        prefix.into()
    } else {
        format!("{}; ожидалось: {}", prefix.into(), expected.join(", "))
    }
}

pub(super) fn token_name(token: &Token) -> String {
    match token {
        Token::Ident(value) => format!("'{}'", value),
        Token::String(value) => format!("\"{}\"", value),
        Token::Number(value) => value.to_string(),
        Token::Float(value) => value.to_string(),
        other => format!("{:?}", other),
    }
}

pub(super) fn token_source_text(token: &Token) -> String {
    match token {
        Token::Eof | Token::Newline => String::new(),
        Token::Semi => ";".into(),
        Token::KwImport => "import".into(),
        Token::KwFrom => "from".into(),
        Token::KwFunction => "function".into(),
        Token::KwLibrary => "library".into(),
        Token::KwVariable => "variable".into(),
        Token::KwClass => "class".into(),
        Token::KwConstructor => "constructor".into(),
        Token::KwPublic => "public".into(),
        Token::KwPrivate => "private".into(),
        Token::KwStatic => "static".into(),
        Token::KwConst => "const".into(),
        Token::KwIf => "if".into(),
        Token::KwElse => "else".into(),
        Token::KwWhile => "while".into(),
        Token::KwFor => "for".into(),
        Token::KwThread => "thread".into(),
        Token::KwTry => "try".into(),
        Token::KwCatch => "catch".into(),
        Token::KwRaise => "raise".into(),
        Token::KwAs => "as".into(),
        Token::KwNew => "new".into(),
        Token::KwReturn => "return".into(),
        Token::KwAnd => "and".into(),
        Token::KwOr => "or".into(),
        Token::True => "true".into(),
        Token::False => "false".into(),
        Token::Empty => "void".into(),
        Token::KwMacro => "macro".into(),
        Token::FatArrow => "=>".into(),
        Token::Arrow => "->".into(),
        Token::EqEq => "==".into(),
        Token::NotEq => "!=".into(),
        Token::Le => "<=".into(),
        Token::Ge => ">=".into(),
        Token::PlusEq => "+=".into(),
        Token::MinusEq => "-=".into(),
        Token::StarEq => "*=".into(),
        Token::SlashEq => "/=".into(),
        Token::PercentEq => "%=".into(),
        Token::Eq | Token::TypeEq => "=".into(),
        Token::Lt => "<".into(),
        Token::Gt => ">".into(),
        Token::Plus => "+".into(),
        Token::Minus => "-".into(),
        Token::Star => "*".into(),
        Token::Slash => "/".into(),
        Token::Percent => "%".into(),
        Token::Bang => "!".into(),
        Token::Dollar => "$".into(),
        Token::Dot | Token::MethodDot => ".".into(),
        Token::Comma => ",".into(),
        Token::Colon => ":".into(),
        Token::LParen | Token::LambdaLParen => "(".into(),
        Token::RParen => ")".into(),
        Token::LBrace => "{".into(),
        Token::RBrace => "}".into(),
        Token::LBracket => "[".into(),
        Token::RBracket => "]".into(),
        Token::String(value) => format!("{value:?}"),
        Token::Float(value) => value.to_string(),
        Token::Number(value) => value.to_string(),
        Token::Ident(value) => value.clone(),
    }
}

#[cfg(test)]
#[path = "parser_tests.rs"]
mod tests;
