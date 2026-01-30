use crate::ast::prelude::Span;

pub struct SourceCode {
    text: String,
    line_starts: Vec<u32>, // Индексы символов начала строк
}

impl SourceCode {
    pub fn new(text: String) -> Self {
        let mut line_starts = vec![0]; // Первая строка всегда начинается с 0
        let mut char_idx = 0u32;

        for c in text.chars() {
            char_idx += 1;
            if c == '\n' {
                line_starts.push(char_idx);
            }
        }

        Self { text, line_starts }
    }
    
    pub fn get_coords(&self, span: Span) -> (usize, usize) {
        let line_idx = self.line_starts
            .binary_search(&span.start)
            .unwrap_or_else(|e| e - 1);

        let line = line_idx + 1;
        let col = (span.start - self.line_starts[line_idx]) as usize + 1;

        (line, col)
    }

    /// Безопасно извлекает текст подстроки (для кириллицы)
    pub fn get_text(&self, span: Span) -> String {
        self.text
            .chars()
            .skip(span.start as usize)
            .take((span.end - span.start) as usize)
            .collect()
    }
}
