use ariadne::{Cache, Source};
use std::collections::{hash_map::Entry, HashMap};
use std::fmt::{Debug, Display};
use std::sync::RwLock;

#[derive(Debug)]
/// File cache used by diagnostics to retrieve source text by path.
pub struct SourceManager {
    files: RwLock<HashMap<String, Box<Source<String>>>>,
}

impl SourceManager {
    /// Creates an empty source cache.
    pub fn new() -> Self {
        Self {
            files: RwLock::new(HashMap::new()),
        }
    }

    /// Loads a file into the cache if it is not present yet.
    pub fn load_file(&self, path: &str) {
        let mut files = self.files.write().unwrap();
        if let Entry::Vacant(entry) = files.entry(path.to_string()) {
            if let Ok(content) = std::fs::read_to_string(entry.key()) {
                entry.insert(Box::new(Source::from(content)));
            }
        }
    }

    /// Reads the current file content from disk.
    pub fn get_file_content(&self, path: &str) -> String {
        std::fs::read_to_string(path).unwrap_or_default()
    }

    /// Converts a character offset into zero-based line and column.
    pub fn get_line_col_from_char_offset(&self, code: &str, char_offset: usize) -> (usize, usize) {
        let mut line = 0;
        let mut col = 0;

        for (i, c) in code.chars().enumerate() {
            if i == char_offset {
                break;
            }
            if c == '\n' {
                line += 1;
                col = 0;
            } else {
                col += 1;
            }
        }
        (line, col)
    }
}

impl Default for SourceManager {
    fn default() -> Self {
        Self::new()
    }
}

impl<'a> Cache<&'a String> for &SourceManager {
    type Storage = String;

    fn fetch(
        &mut self,
        path: &&'a String,
    ) -> Result<&Source<<Self as Cache<&'a String>>::Storage>, impl Debug> {
        let path_str: &str = path;

        if let Some(source) = self.files.read().unwrap().get(path_str) {
            let source = source.as_ref() as *const Source<String>;
            // Sources are boxed, never replaced or removed, and cannot outlive the manager.
            return Ok::<&Source, String>(unsafe { &*source });
        }

        let content =
            std::fs::read_to_string(path).map_err(|e| format!("Ошибка чтения {}: {}", path, e))?;

        let mut map = self.files.write().unwrap();
        let source = map
            .entry(path_str.to_string())
            .or_insert_with(|| Box::new(Source::from(content)))
            .as_ref() as *const Source<String>;

        // Dropping the lock guard cannot move the boxed source.
        Ok(unsafe { &*source })
    }

    fn display<'d>(&self, id: &'d &'a String) -> Option<impl Display + 'd> {
        Some(id.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn loaded_source_preserves_crlf_offsets() {
        let path = std::env::temp_dir().join(format!(
            "goida-source-manager-crlf-{}.goida",
            std::process::id()
        ));
        std::fs::write(&path, "first\r\nsecond\r\n").unwrap();

        let manager = SourceManager::new();
        let path = path.to_string_lossy().into_owned();
        manager.load_file(&path);

        let mut cache = &manager;
        assert_eq!(cache.fetch(&&path).unwrap().text(), "first\r\nsecond\r\n");

        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn cached_source_address_stays_stable_when_map_grows() {
        let manager = SourceManager::new();
        let path = String::from("first");
        manager
            .files
            .write()
            .unwrap()
            .insert(path.clone(), Box::new(Source::from(String::from("first"))));

        let mut cache = &manager;
        let original = cache.fetch(&&path).unwrap() as *const Source<String>;

        let mut files = manager.files.write().unwrap();
        for index in 0..10_000 {
            files.insert(
                format!("source-{index}"),
                Box::new(Source::from(index.to_string())),
            );
        }
        drop(files);

        let current = cache.fetch(&&path).unwrap() as *const Source<String>;
        assert_eq!(original, current);
    }
}
