#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SourceLocation {
    pub line: u32,
    pub column: u32,
    pub offset: u32,
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