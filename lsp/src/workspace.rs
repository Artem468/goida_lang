use std::fs;
use std::path::{Path, PathBuf};

use goida_syntax::import_paths::resolve_import_path as resolve_direct_import_path;

pub(crate) fn resolve_import_path(
    current_file: &Path,
    import_path: &str,
    workspace_roots: &[PathBuf],
) -> Option<PathBuf> {
    let direct_candidate = resolve_direct_import_path(current_file, import_path);
    if direct_candidate.exists() {
        return Some(direct_candidate);
    }

    let mut with_ext = PathBuf::from(import_path);
    if with_ext.extension().is_none() {
        with_ext.set_extension("goida");
    }
    let mut candidates = Vec::new();
    if with_ext.is_absolute() {
        candidates.push(with_ext);
    } else {
        for root in workspace_roots {
            candidates.push(root.join(&with_ext));
            candidates.push(root.join(".goida").join("deps").join(&with_ext));
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

#[cfg(test)]
mod tests {
    use super::resolve_import_path;
    use std::fs;
    use std::path::PathBuf;

    #[test]
    fn resolves_short_import_from_project_deps() {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("target")
            .join("lsp_workspace_import_test");
        if root.exists() {
            fs::remove_dir_all(&root).expect("failed to clear lsp import test dir");
        }

        let deps_dir = root.join(".goida/deps/harpoon");
        fs::create_dir_all(&deps_dir).expect("failed to create deps dir");
        fs::write(
            root.join("main.goida"),
            "подключить \"harpoon/главный\" как web\n",
        )
        .expect("failed to write main file");
        fs::write(deps_dir.join("главный.goida"), "значение = 1\n")
            .expect("failed to write dep module");

        let resolved = resolve_import_path(
            &root.join("main.goida"),
            "harpoon/главный",
            std::slice::from_ref(&root),
        )
        .expect("short import should resolve from .goida/deps");

        assert_eq!(
            deps_dir.join("главный.goida").canonicalize().unwrap(),
            resolved.canonicalize().unwrap()
        );
    }
}
