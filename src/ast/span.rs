use std::ops::Range;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Span {
    pub start: u32,
    pub end: u32,
}

impl Default for Span {
    fn default() -> Self {
        Self { start: 0, end: 0 }
    }
}

impl Span {
    pub fn new(start: usize, end: usize) -> Self {
        Self {
            start: start as u32,
            end: end as u32,
        }
    }

    pub fn merge(self, other: Span) -> Self {
        Self {
            start: self.start.min(other.start),
            end: self.end.max(other.end),
        }
    }
}

impl<'a> From<pest::Span<'a>> for Span {
    fn from(pest_span: pest::Span<'a>) -> Self {
        let start = pest_span.start();
        let end = pest_span.end();

        Self {
            start: start as u32,
            end: end as u32,
        }
    }
}

impl From<Span> for Range<usize> {
    fn from(span: Span) -> Self {
        span.start as usize..span.end as usize
    }
}

impl Span {
    pub(crate) fn as_ariadne<'a>(&self, file: &'a str, code: &str) -> (&'a str, Range<usize>) {
        let start = self.start as usize;
        let end = self.end as usize;

        let char_start = code
            .get(..start)
            .map(|s| s.chars().count())
            .unwrap_or(0);
        let char_end = code
            .get(..end)
            .map(|s| s.chars().count())
            .unwrap_or(char_start);
        (file, char_start..char_end)
    }
}
