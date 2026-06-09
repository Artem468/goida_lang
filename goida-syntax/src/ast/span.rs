use std::ops::Range;
use string_interner::{DefaultSymbol, Symbol};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
/// Source range in byte offsets plus the interned file/module id.
pub struct Span {
    pub start: u32,
    pub end: u32,
    pub file_id: DefaultSymbol,
}

impl Default for Span {
    fn default() -> Self {
        Self {
            start: 0,
            end: 0,
            file_id: DefaultSymbol::try_from_usize(0).unwrap(),
        }
    }
}

impl Span {
    /// Creates a span from byte offsets and a file id.
    pub fn new(start: usize, end: usize, file_id: DefaultSymbol) -> Self {
        Self {
            start: start as u32,
            end: end as u32,
            file_id,
        }
    }

    /// Returns a copy of this span with another file id.
    pub fn set_file_id(&mut self, file_id: DefaultSymbol) -> Self {
        self.file_id = file_id;
        *self
    }
}

impl From<Span> for Range<usize> {
    fn from(span: Span) -> Self {
        span.start as usize..span.end as usize
    }
}

impl Span {
    /// Converts byte offsets into character offsets expected by ariadne labels.
    pub fn as_ariadne(&self, code: &str) -> Range<usize> {
        let start = self.start as usize;
        let end = self.end as usize;

        let char_start = code.get(..start).map(|s| s.chars().count()).unwrap_or(0);
        let char_end = code
            .get(..end)
            .map(|s| s.chars().count())
            .unwrap_or(char_start);
        char_start..char_end
    }
}
