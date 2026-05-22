use std::path::{Path, PathBuf};

pub const GOIDA_VENV_ENV: &str = "GOIDA_VENV";

pub fn resolve_import_path(current_module_path: &Path, import_path: &str) -> PathBuf {
    let requested = Path::new(import_path);
    let requested = requested.with_extension("goida");

    if requested.is_absolute() {
        return requested;
    }

    let module_dir = current_module_path
        .parent()
        .unwrap_or_else(|| Path::new("."));
    let local_candidate = module_dir.join(&requested);
    if local_candidate.exists() {
        return local_candidate;
    }

    if let Some(project_root) = find_project_root(module_dir) {
        let project_dep = project_root.join(".goida").join("deps").join(&requested);
        if project_dep.exists() {
            return project_dep;
        }
    }

    if let Ok(venv_path) = std::env::var(GOIDA_VENV_ENV) {
        let venv_dep = Path::new(&venv_path).join("deps").join(&requested);
        if venv_dep.exists() {
            return venv_dep;
        }
    }

    local_candidate
}

fn find_project_root(start: &Path) -> Option<PathBuf> {
    let mut current = start
        .canonicalize()
        .ok()
        .or_else(|| Some(start.to_path_buf()))?;

    loop {
        if current.join("goida.toml").exists() || current.join(".goida").is_dir() {
            return Some(current);
        }

        if !current.pop() {
            return None;
        }
    }
}
