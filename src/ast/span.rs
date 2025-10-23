#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SourceLocation {
    pub line: usize,
    pub column: usize,
    pub offset: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Span {
    pub start: SourceLocation,
    pub end: SourceLocation,
}

impl Default for Span {
    fn default() -> Self {
        Self {
            start: SourceLocation {
                line: 0,
                column: 0,
                offset: 0,
            },
            end: SourceLocation {
                line: 0,
                column: 0,
                offset: 0,
            },
        }
    }
}