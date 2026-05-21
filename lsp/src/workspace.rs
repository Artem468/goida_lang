use std::fs;
use std::path::{Path, PathBuf};

pub(crate) fn resolve_import_path(
    current_file: &Path,
    import_path: &str,
    workspace_roots: &[PathBuf],
) -> Option<PathBuf> {
    let normalized = import_path.replace('/', std::path::MAIN_SEPARATOR_STR);
    let mut with_ext = PathBuf::from(normalized);
    if with_ext.extension().is_none() {
        with_ext.set_extension("goida");
    }

    let mut candidates = Vec::new();
    if with_ext.is_absolute() {
        candidates.push(with_ext);
    } else {
        if let Some(parent) = current_file.parent() {
            candidates.push(parent.join(&with_ext));
        }
        for root in workspace_roots {
            candidates.push(root.join(&with_ext));
        }
    }

    candidates.into_iter().find(|path| path.exists())
}

pub(crate) fn collect_goida_files(roots: &[PathBuf]) -> Vec<PathBuf> {
    let mut files = Vec::new();
    for root in roots {
        walk_goida_files(root, &mut files);
    }
    files
}

fn walk_goida_files(dir: &Path, files: &mut Vec<PathBuf>) {
    if dir
        .file_name()
        .and_then(|name| name.to_str())
        .map(|name| matches!(name, ".git" | "target" | "node_modules" | ".idea"))
        .unwrap_or(false)
    {
        return;
    }

    let entries = match fs::read_dir(dir) {
        Ok(v) => v,
        Err(_) => return,
    };
    for entry in entries {
        let Ok(entry) = entry else {
            continue;
        };
        let path = entry.path();
        if path.is_dir() {
            walk_goida_files(&path, files);
            continue;
        }
        if path
            .extension()
            .and_then(|v| v.to_str())
            .map(|ext| ext.eq_ignore_ascii_case("goida"))
            .unwrap_or(false)
        {
            files.push(path);
        }
    }
}
