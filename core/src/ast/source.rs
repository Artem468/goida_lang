use ariadne::{Cache, Source};
use std::collections::HashMap;
use std::fmt::{Debug, Display};
use std::sync::RwLock;

#[derive(Debug)]
pub struct SourceManager {
    pub files: RwLock<HashMap<String, Source<String>>>,
}

impl SourceManager {
    pub fn new() -> Self {
        Self {
            files: RwLock::new(HashMap::new()),
        }
    }

    pub fn load_file(&self, path: &str) {
        let mut files = self.files.write().unwrap();
        if !files.contains_key(path) {
            if let Ok(content) = std::fs::read_to_string(path) {
                let normalized = content.replace("\r\n", "\n");
                files.insert(path.to_string(), Source::from(normalized));
            }
        }
    }

    pub fn get_file_content(&self, path: &str) -> String {
        if self.files.read().unwrap().get(path).is_some() {
            return std::fs::read_to_string(path).unwrap_or_default();
        }
        std::fs::read_to_string(path).unwrap_or_default()
    }

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

impl<'a> Cache<&'a String> for &SourceManager {
    type Storage = String;

    fn fetch(
        &mut self,
        path: &&'a String,
    ) -> Result<&Source<<Self as Cache<&'a String>>::Storage>, impl Debug> {
        let path_str: &str = path;

        if let Some(source) = self.files.read().unwrap().get(path_str) {
            return Ok::<&Source, String>(unsafe {
                std::mem::transmute::<&Source, &Source>(source)
            });
        }

        let content =
            std::fs::read_to_string(path).map_err(|e| format!("Ошибка чтения {}: {}", path, e))?;

        let mut map = self.files.write().unwrap();
        map.insert(path_str.to_string(), Source::from(content));

        Ok(unsafe { std::mem::transmute::<&Source, &Source>(map.get(path_str).unwrap()) })
    }

    fn display<'d>(&self, id: &'d &'a String) -> Option<impl Display + 'd> {
        Some(id.to_string())
    }
}
