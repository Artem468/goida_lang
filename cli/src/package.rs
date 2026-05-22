use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
    process::Command,
};

const MANIFEST_FILE: &str = "goida.toml";
const LOCK_FILE: &str = "goida.lock";
const DEPS_DIR: &str = ".goida/deps";

#[derive(Debug, Serialize, Deserialize)]
struct Manifest {
    package: PackageInfo,
    #[serde(default)]
    dependencies: BTreeMap<String, Dependency>,
}

#[derive(Debug, Serialize, Deserialize)]
struct PackageInfo {
    name: String,
    description: String,
    version: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct Dependency {
    #[serde(skip_serializing_if = "Option::is_none")]
    git: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    rev: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    branch: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tag: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Default)]
struct LockFile {
    #[serde(default)]
    package: Vec<LockedPackage>,
}

#[derive(Debug, Serialize, Deserialize)]
struct LockedPackage {
    name: String,
    source: String,
    revision: String,
    path: String,
}

struct ResolvedDependency {
    source: String,
    revision: String,
}

pub(crate) fn new_project(name: &str, description: &str, version: &str) -> Result<(), String> {
    let root = PathBuf::from(name);
    if root.exists() {
        return Err(format!("Проект '{}' уже существует", root.display()));
    }

    fs::create_dir_all(&root).map_err(|err| format!("Не удалось создать проект: {err}"))?;
    fs::write(root.join("главный.goida"), starter_source())
        .map_err(|err| format!("Не удалось записать главный.goida: {err}"))?;

    let manifest = Manifest {
        package: PackageInfo {
            name: name.to_string(),
            description: description.to_string(),
            version: version.to_string(),
        },
        dependencies: BTreeMap::new(),
    };
    write_manifest(&root, &manifest)?;
    write_lock(&root, &LockFile::default())?;

    println!("Создан проект '{}'", root.display());
    Ok(())
}

pub(crate) fn add_dependency(
    name: &str,
    git: Option<String>,
    path: Option<String>,
    rev: Option<String>,
    branch: Option<String>,
    tag: Option<String>,
) -> Result<(), String> {
    if git.is_some() == path.is_some() {
        return Err("Укажите ровно один источник зависимости: --git или --path".into());
    }

    if path.is_some() && (rev.is_some() || branch.is_some() || tag.is_some()) {
        return Err("--rev, --branch и --tag доступны только для --git зависимостей".into());
    }

    if [rev.is_some(), branch.is_some(), tag.is_some()]
        .into_iter()
        .filter(|value| *value)
        .count()
        > 1
    {
        return Err("Укажите только один вариант: --rev, --branch или --tag".into());
    }

    let root = std::env::current_dir().map_err(|err| format!("Не удалось получить cwd: {err}"))?;
    let mut manifest = read_manifest(&root)?;
    let dependency = Dependency {
        git,
        path,
        rev,
        branch,
        tag,
    };
    let resolved = resolve_dependency(&root, name, &dependency)?;

    manifest.dependencies.insert(name.to_string(), dependency);
    write_manifest(&root, &manifest)?;

    let mut lock = read_lock(&root)?;
    lock.package.retain(|package| package.name != name);
    lock.package.push(LockedPackage {
        name: name.to_string(),
        source: resolved.source,
        revision: resolved.revision,
        path: format!("{DEPS_DIR}/{name}"),
    });
    lock.package
        .sort_by(|left, right| left.name.cmp(&right.name));
    write_lock(&root, &lock)?;

    println!("Добавлена зависимость '{name}'");
    Ok(())
}

pub(crate) fn remove_dependency(name: &str) -> Result<(), String> {
    let root = std::env::current_dir().map_err(|err| format!("Не удалось получить cwd: {err}"))?;
    let mut manifest = read_manifest(&root)?;
    if manifest.dependencies.remove(name).is_none() {
        return Err(format!("Зависимость '{name}' не найдена"));
    }
    write_manifest(&root, &manifest)?;

    let mut lock = read_lock(&root)?;
    lock.package.retain(|package| package.name != name);
    write_lock(&root, &lock)?;

    let dep_path = root.join(DEPS_DIR).join(name);
    if dep_path.exists() {
        ensure_inside(&root.join(DEPS_DIR), &dep_path)?;
        fs::remove_dir_all(&dep_path)
            .map_err(|err| format!("Не удалось удалить '{}': {err}", dep_path.display()))?;
    }

    println!("Удалена зависимость '{name}'");
    Ok(())
}

fn starter_source() -> &'static str {
    "функция привет_мир() {\n    печать(\"Привет, мир!\")\n}\n\nпривет_мир()\n"
}

fn read_manifest(root: &Path) -> Result<Manifest, String> {
    let path = root.join(MANIFEST_FILE);
    let content = fs::read_to_string(&path)
        .map_err(|err| format!("Не удалось прочитать '{}': {err}", path.display()))?;
    toml::from_str(&content).map_err(|err| format!("Некорректный {MANIFEST_FILE}: {err}"))
}

fn write_manifest(root: &Path, manifest: &Manifest) -> Result<(), String> {
    let content = toml::to_string_pretty(manifest)
        .map_err(|err| format!("Не удалось сериализовать {MANIFEST_FILE}: {err}"))?;
    fs::write(root.join(MANIFEST_FILE), content)
        .map_err(|err| format!("Не удалось записать {MANIFEST_FILE}: {err}"))
}

fn read_lock(root: &Path) -> Result<LockFile, String> {
    let path = root.join(LOCK_FILE);
    if !path.exists() {
        return Ok(LockFile::default());
    }
    let content = fs::read_to_string(&path)
        .map_err(|err| format!("Не удалось прочитать '{}': {err}", path.display()))?;
    toml::from_str(&content).map_err(|err| format!("Некорректный {LOCK_FILE}: {err}"))
}

fn write_lock(root: &Path, lock: &LockFile) -> Result<(), String> {
    let content = toml::to_string_pretty(lock)
        .map_err(|err| format!("Не удалось сериализовать {LOCK_FILE}: {err}"))?;
    fs::write(root.join(LOCK_FILE), content)
        .map_err(|err| format!("Не удалось записать {LOCK_FILE}: {err}"))
}

fn resolve_dependency(
    root: &Path,
    name: &str,
    dependency: &Dependency,
) -> Result<ResolvedDependency, String> {
    if let Some(git) = &dependency.git {
        let revision = resolve_git_revision(git, dependency)?;
        checkout_git_dependency(root, name, git, &revision)?;
        return Ok(ResolvedDependency {
            source: format!("git+{git}"),
            revision,
        });
    }

    let Some(path) = &dependency.path else {
        return Err("У зависимости не указан источник".into());
    };
    let source_path = resolve_local_source_path(root, path)?;
    copy_local_dependency(root, name, &source_path)?;
    Ok(ResolvedDependency {
        source: format!("path+{}", source_path.display()),
        revision: "local".into(),
    })
}

fn resolve_git_revision(git: &str, dependency: &Dependency) -> Result<String, String> {
    if let Some(rev) = &dependency.rev {
        if is_full_git_sha(rev) {
            return Ok(rev.clone());
        }
        return ls_remote(git, rev);
    }

    if let Some(branch) = &dependency.branch {
        return ls_remote(git, &format!("refs/heads/{branch}"));
    }

    if let Some(tag) = &dependency.tag {
        return ls_remote(git, &format!("refs/tags/{tag}"));
    }

    ls_remote(git, "HEAD")
}

fn ls_remote(git: &str, reference: &str) -> Result<String, String> {
    let output = Command::new("git")
        .args(["ls-remote", git, reference])
        .output()
        .map_err(|err| format!("Не удалось запустить git ls-remote: {err}"))?;
    if !output.status.success() {
        return Err(format!(
            "git ls-remote завершился с ошибкой: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    stdout
        .lines()
        .find_map(|line| line.split_whitespace().next())
        .map(str::to_string)
        .ok_or_else(|| format!("Git-ссылка '{reference}' не найдена"))
}

fn checkout_git_dependency(
    root: &Path,
    name: &str,
    git: &str,
    revision: &str,
) -> Result<(), String> {
    let deps_root = root.join(DEPS_DIR);
    fs::create_dir_all(&deps_root)
        .map_err(|err| format!("Не удалось создать '{}': {err}", deps_root.display()))?;
    let dep_path = deps_root.join(name);
    ensure_inside(&deps_root, &dep_path)?;

    if dep_path.exists() {
        fs::remove_dir_all(&dep_path)
            .map_err(|err| format!("Не удалось обновить '{}': {err}", dep_path.display()))?;
    }

    run_git(
        &deps_root,
        &["clone", "--recurse-submodules", git, name],
        "git clone",
    )?;
    run_git(&dep_path, &["checkout", revision], "git checkout")?;
    Ok(())
}

fn resolve_local_source_path(root: &Path, path: &str) -> Result<PathBuf, String> {
    let raw_path = PathBuf::from(path);
    let source_path = if raw_path.is_absolute() {
        raw_path
    } else {
        root.join(raw_path)
    };
    let source_path = source_path
        .canonicalize()
        .map_err(|err| format!("Не удалось найти локальную зависимость '{}': {err}", path))?;
    if !source_path.is_dir() {
        return Err(format!(
            "Локальная зависимость '{}' должна быть каталогом",
            source_path.display()
        ));
    }
    Ok(source_path)
}

fn copy_local_dependency(root: &Path, name: &str, source_path: &Path) -> Result<(), String> {
    let deps_root = root.join(DEPS_DIR);
    fs::create_dir_all(&deps_root)
        .map_err(|err| format!("Не удалось создать '{}': {err}", deps_root.display()))?;
    let dep_path = deps_root.join(name);
    ensure_inside(&deps_root, &dep_path)?;

    if dep_path.exists() {
        fs::remove_dir_all(&dep_path)
            .map_err(|err| format!("Не удалось обновить '{}': {err}", dep_path.display()))?;
    }

    copy_dir_all(source_path, &dep_path)
}

fn copy_dir_all(source: &Path, destination: &Path) -> Result<(), String> {
    fs::create_dir_all(destination)
        .map_err(|err| format!("Не удалось создать '{}': {err}", destination.display()))?;

    for entry in fs::read_dir(source)
        .map_err(|err| format!("Не удалось прочитать '{}': {err}", source.display()))?
    {
        let entry = entry.map_err(|err| format!("Не удалось прочитать элемент каталога: {err}"))?;
        let file_name = entry.file_name();
        if file_name == ".git" {
            continue;
        }

        let source_path = entry.path();
        let destination_path = destination.join(file_name);
        let file_type = entry
            .file_type()
            .map_err(|err| format!("Не удалось проверить '{}': {err}", source_path.display()))?;

        if file_type.is_dir() {
            copy_dir_all(&source_path, &destination_path)?;
        } else if file_type.is_file() {
            fs::copy(&source_path, &destination_path).map_err(|err| {
                format!(
                    "Не удалось скопировать '{}' в '{}': {err}",
                    source_path.display(),
                    destination_path.display()
                )
            })?;
        }
    }

    Ok(())
}

fn run_git(cwd: &Path, args: &[&str], label: &str) -> Result<(), String> {
    let output = Command::new("git")
        .current_dir(cwd)
        .args(args)
        .output()
        .map_err(|err| format!("Не удалось запустить {label}: {err}"))?;
    if output.status.success() {
        return Ok(());
    }

    Err(format!(
        "{label} завершился с ошибкой: {}",
        String::from_utf8_lossy(&output.stderr).trim()
    ))
}

fn ensure_inside(root: &Path, child: &Path) -> Result<(), String> {
    let root = root
        .canonicalize()
        .map_err(|err| format!("Не удалось проверить '{}': {err}", root.display()))?;
    let child_parent = child
        .parent()
        .ok_or_else(|| format!("Некорректный путь '{}'", child.display()))?
        .canonicalize()
        .map_err(|err| format!("Не удалось проверить '{}': {err}", child.display()))?;

    if child_parent.starts_with(root) {
        Ok(())
    } else {
        Err(format!(
            "Путь '{}' вне каталога зависимостей",
            child.display()
        ))
    }
}

fn is_full_git_sha(value: &str) -> bool {
    value.len() == 40 && value.chars().all(|ch| ch.is_ascii_hexdigit())
}
