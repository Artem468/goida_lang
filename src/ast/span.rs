#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Span {
    pub start: u32,
    pub end: u32,
}

impl Default for Span {
    fn default() -> Self {
        Self {
            start: 0,
            end: 0,
        }
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

