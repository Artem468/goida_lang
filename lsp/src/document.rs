use std::cmp::min;
use std::sync::Arc;
use tower_lsp::lsp_types::Position;

#[derive(Clone)]
pub(crate) struct Document {
    text: Arc<str>,
    line_starts: Arc<[usize]>,
}

#[derive(Clone)]
pub(crate) struct LocatedIdentifier {
    pub(crate) name: String,
    pub(crate) start_char: usize,
    pub(crate) module_alias: Option<String>,
}

impl Document {
    pub(crate) fn new(text: impl Into<Arc<str>>) -> Self {
        let text = text.into();
        let line_starts = compute_line_starts(&text).into();
        Self { text, line_starts }
    }

    pub(crate) fn text(&self) -> &str {
        &self.text
    }

    pub(crate) fn line_starts(&self) -> &[usize] {
        &self.line_starts
    }

    pub(crate) fn position_to_char_offset(&self, position: Position) -> usize {
        position_to_char_offset(self.text(), self.line_starts(), position)
    }

    pub(crate) fn char_offset_to_position(&self, char_offset: usize) -> Option<Position> {
        char_offset_to_position(self.line_starts(), char_offset)
    }
}

pub(crate) fn compute_line_starts(text: &str) -> Vec<usize> {
    let mut starts = vec![0usize];
    let mut offset = 0usize;
    for ch in text.chars() {
        offset += 1;
        if ch == '\n' {
            starts.push(offset);
        }
    }
    starts
}

pub(crate) fn char_offset_to_position(
    line_starts: &[usize],
    char_offset: usize,
) -> Option<Position> {
    let line = match line_starts.binary_search(&char_offset) {
        Ok(line) => line,
        Err(0) => 0,
        Err(next_line) => next_line.saturating_sub(1),
    };
    let col = char_offset.saturating_sub(*line_starts.get(line)?);
    Some(Position::new(line as u32, col as u32))
}

pub(crate) fn position_to_char_offset(
    text: &str,
    line_starts: &[usize],
    position: Position,
) -> usize {
    let target_line = position.line as usize;
    let target_col = position.character as usize;
    let Some(line_start) = line_starts.get(target_line).copied() else {
        return text.chars().count();
    };
    let next_line_start = line_starts
        .get(target_line + 1)
        .copied()
        .unwrap_or_else(|| text.chars().count());
    line_start + min(target_col, next_line_start.saturating_sub(line_start))
}

pub(crate) fn find_identifier_at_char_offset(
    text: &str,
    char_offset: usize,
) -> Option<LocatedIdentifier> {
    let chars: Vec<char> = text.chars().collect();
    if chars.is_empty() {
        return None;
    }

    let mut pos = min(char_offset, chars.len().saturating_sub(1));
    if !is_identifier_continue(chars[pos]) && pos > 0 && is_identifier_continue(chars[pos - 1]) {
        pos -= 1;
    }
    if !is_identifier_continue(chars[pos]) {
        return None;
    }

    let mut start = pos;
    while start > 0 && is_identifier_continue(chars[start - 1]) {
        start -= 1;
    }
    if !is_identifier_start(chars[start]) {
        return None;
    }

    let mut end = pos + 1;
    while end < chars.len() && is_identifier_continue(chars[end]) {
        end += 1;
    }

    let name: String = chars[start..end].iter().collect();
    let module_alias = if start >= 2 && chars[start - 1] == '.' {
        let alias_end = start - 1;
        let mut alias_start = alias_end;
        while alias_start > 0 && is_identifier_continue(chars[alias_start - 1]) {
            alias_start -= 1;
        }
        if alias_start < alias_end && is_identifier_start(chars[alias_start]) {
            Some(chars[alias_start..alias_end].iter().collect())
        } else {
            None
        }
    } else {
        None
    };

    Some(LocatedIdentifier {
        name,
        start_char: start,
        module_alias,
    })
}

fn is_identifier_start(ch: char) -> bool {
    ch == '_' || ch.is_alphabetic()
}

fn is_identifier_continue(ch: char) -> bool {
    ch == '_' || ch.is_alphanumeric()
}
