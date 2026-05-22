use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
    process::Command,
};

use goida_core::import_paths::GOIDA_VENV_ENV;

const MANIFEST_FILE: &str = "goida.toml";
const LOCK_FILE: &str = "goida.lock";
const DEPS_DIR: &str = ".goida/deps";
const VENV_CONFIG_FILE: &str = "goida-venv.toml";

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
    path: String,
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
        path: resolved.path,
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
    let locked_path = lock
        .package
        .iter()
        .find(|package| package.name == name)
        .map(|package| package.path.clone());
    lock.package.retain(|package| package.name != name);
    write_lock(&root, &lock)?;

    let dep_path = locked_path
        .as_deref()
        .and_then(|path| resolve_locked_dep_path(&root, path))
        .unwrap_or_else(|| root.join(DEPS_DIR).join(name));
    if dep_path.exists() {
        let dep_root = locked_path
            .as_deref()
            .filter(|path| path.starts_with("$GOIDA_VENV/"))
            .map(|_| dependency_install_root(&root))
            .transpose()?
            .unwrap_or_else(|| root.join(DEPS_DIR));
        ensure_inside(&dep_root, &dep_path)?;
        fs::remove_dir_all(&dep_path)
            .map_err(|err| format!("Не удалось удалить '{}': {err}", dep_path.display()))?;
    }

    println!("Удалена зависимость '{name}'");
    Ok(())
}

pub(crate) fn create_venv(path: &str) -> Result<(), String> {
    let root = PathBuf::from(path);
    fs::create_dir_all(root.join("deps"))
        .map_err(|err| format!("Не удалось создать каталог зависимостей окружения: {err}"))?;
    fs::create_dir_all(root.join("Scripts"))
        .map_err(|err| format!("Не удалось создать каталог Scripts: {err}"))?;
    fs::create_dir_all(root.join("bin"))
        .map_err(|err| format!("Не удалось создать каталог bin: {err}"))?;

    let absolute_root = root.canonicalize().map_err(|err| {
        format!(
            "Не удалось определить путь окружения '{}': {err}",
            root.display()
        )
    })?;
    let prompt_name = absolute_root
        .file_name()
        .map(|name| name.to_string_lossy().to_string())
        .unwrap_or_else(|| "goida".to_string());
    let absolute = absolute_root.to_string_lossy().to_string();
    let shell_absolute = absolute_root.to_string_lossy().replace('\\', "/");

    fs::write(
        root.join(VENV_CONFIG_FILE),
        format!(
            "[venv]\nversion = \"1\"\ndeps = \"{}\"\n",
            absolute_root.join("deps").display()
        ),
    )
    .map_err(|err| format!("Не удалось записать {VENV_CONFIG_FILE}: {err}"))?;

    fs::write(
        root.join("Scripts").join("Activate.ps1"),
        powershell_activate(&absolute, &prompt_name),
    )
    .map_err(|err| format!("Не удалось записать Activate.ps1: {err}"))?;
    fs::write(
        root.join("Scripts").join("Deactivate.ps1"),
        powershell_deactivate(),
    )
    .map_err(|err| format!("Не удалось записать Deactivate.ps1: {err}"))?;
    fs::write(
        root.join("Scripts").join("activate.bat"),
        cmd_activate(&absolute, &prompt_name),
    )
    .map_err(|err| format!("Не удалось записать activate.bat: {err}"))?;
    fs::write(
        root.join("Scripts").join("deactivate.bat"),
        cmd_deactivate(),
    )
    .map_err(|err| format!("Не удалось записать deactivate.bat: {err}"))?;
    fs::write(
        root.join("bin").join("activate"),
        sh_activate(&shell_absolute, &prompt_name),
    )
    .map_err(|err| format!("Не удалось записать bin/activate: {err}"))?;
    fs::write(root.join("bin").join("deactivate"), sh_deactivate())
        .map_err(|err| format!("Не удалось записать bin/deactivate: {err}"))?;

    println!(
        "Создано виртуальное окружение '{}'",
        absolute_root.display()
    );
    println!(
        "PowerShell: . '{}'",
        absolute_root.join("Scripts/Activate.ps1").display()
    );
    println!(
        "cmd.exe:    {}",
        absolute_root.join("Scripts/activate.bat").display()
    );
    println!(
        "sh/bash:    source '{}'",
        absolute_root.join("bin/activate").display()
    );
    Ok(())
}

fn starter_source() -> &'static str {
    "функция главная() {\n    печать(\"Привет, мир!\")\n}\n\nглавная()\n"
}

fn powershell_activate(venv_path: &str, prompt_name: &str) -> String {
    let venv_path = escape_powershell_double_quoted(venv_path);
    let prompt_name = escape_powershell_double_quoted(prompt_name);
    format!(
        r#"$env:GOIDA_OLD_VENV = $env:GOIDA_VENV
$env:GOIDA_VENV = "{venv_path}"

if (Test-Path Function:prompt) {{
    Copy-Item Function:prompt Function:_goida_old_prompt -Force
}}

function global:prompt {{
    "({prompt_name}) " + $(if (Test-Path Function:_goida_old_prompt) {{ & _goida_old_prompt }} else {{ "PS $($executionContext.SessionState.Path.CurrentLocation)$('>' * ($nestedPromptLevel + 1)) " }})
}}

function global:deactivate {{
    if ($env:GOIDA_OLD_VENV) {{
        $env:GOIDA_VENV = $env:GOIDA_OLD_VENV
        Remove-Item Env:GOIDA_OLD_VENV -ErrorAction SilentlyContinue
    }} else {{
        Remove-Item Env:GOIDA_VENV -ErrorAction SilentlyContinue
    }}
    if (Test-Path Function:_goida_old_prompt) {{
        Copy-Item Function:_goida_old_prompt Function:prompt -Force
        Remove-Item Function:_goida_old_prompt -ErrorAction SilentlyContinue
    }} else {{
        Remove-Item Function:prompt -ErrorAction SilentlyContinue
    }}
    Remove-Item Function:deactivate -ErrorAction SilentlyContinue
}}

Write-Host "Activated Goida venv: $env:GOIDA_VENV"
"#
    )
}

fn powershell_deactivate() -> &'static str {
    r#"if ($env:GOIDA_OLD_VENV) {
    $env:GOIDA_VENV = $env:GOIDA_OLD_VENV
    Remove-Item Env:GOIDA_OLD_VENV -ErrorAction SilentlyContinue
} else {
    Remove-Item Env:GOIDA_VENV -ErrorAction SilentlyContinue
}
if (Test-Path Function:_goida_old_prompt) {
    Copy-Item Function:_goida_old_prompt Function:prompt -Force
    Remove-Item Function:_goida_old_prompt -ErrorAction SilentlyContinue
} else {
    Remove-Item Function:prompt -ErrorAction SilentlyContinue
}
Remove-Item Function:deactivate -ErrorAction SilentlyContinue
Write-Host "Deactivated Goida venv"
"#
}

fn cmd_activate(venv_path: &str, prompt_name: &str) -> String {
    let venv_path = escape_cmd_value(venv_path);
    let prompt_name = escape_cmd_prompt_name(prompt_name);
    format!(
        "@echo off\r\nset GOIDA_OLD_VENV=%GOIDA_VENV%\r\nset \"GOIDA_VENV={venv_path}\"\r\nset \"GOIDA_OLD_PROMPT=%PROMPT%\"\r\nset \"PROMPT=({prompt_name}) %PROMPT%\"\r\necho Activated Goida venv: %GOIDA_VENV%\r\n"
    )
}

fn cmd_deactivate() -> &'static str {
    "@echo off\r\nif defined GOIDA_OLD_VENV (set GOIDA_VENV=%GOIDA_OLD_VENV%) else (set GOIDA_VENV=)\r\nif defined GOIDA_OLD_PROMPT (set \"PROMPT=%GOIDA_OLD_PROMPT%\")\r\nset GOIDA_OLD_VENV=\r\nset GOIDA_OLD_PROMPT=\r\necho Deactivated Goida venv\r\n"
}

fn sh_activate(venv_path: &str, prompt_name: &str) -> String {
    let prompt_name = escape_sh_double_quoted(prompt_name);
    format!(
        r#"export GOIDA_OLD_VENV="${{GOIDA_VENV-}}"
export GOIDA_VENV="{venv_path}"
export GOIDA_OLD_PS1="${{PS1-}}"
export PS1="({prompt_name}) ${{PS1-}}"

deactivate() {{
    if [ -n "${{GOIDA_OLD_VENV-}}" ]; then
        export GOIDA_VENV="$GOIDA_OLD_VENV"
        unset GOIDA_OLD_VENV
    else
        unset GOIDA_VENV
    fi
    if [ -n "${{GOIDA_OLD_PS1+x}}" ]; then
        export PS1="$GOIDA_OLD_PS1"
        unset GOIDA_OLD_PS1
    fi
    unset -f deactivate
}}

echo "Activated Goida venv: $GOIDA_VENV"
"#
    )
}

fn sh_deactivate() -> &'static str {
    r#"if [ -n "${GOIDA_OLD_VENV-}" ]; then
    export GOIDA_VENV="$GOIDA_OLD_VENV"
else
    unset GOIDA_VENV
fi
unset GOIDA_OLD_VENV
if [ -n "${GOIDA_OLD_PS1+x}" ]; then
    export PS1="$GOIDA_OLD_PS1"
    unset GOIDA_OLD_PS1
fi
unset -f deactivate 2>/dev/null || true
echo "Deactivated Goida venv"
"#
}

fn escape_powershell_double_quoted(value: &str) -> String {
    value
        .replace('`', "``")
        .replace('"', "`\"")
        .replace('$', "`$")
}

fn escape_cmd_prompt_name(value: &str) -> String {
    value.replace('%', "%%")
}

fn escape_cmd_value(value: &str) -> String {
    value.replace('%', "%%")
}

fn escape_sh_double_quoted(value: &str) -> String {
    value
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('$', "\\$")
        .replace('`', "\\`")
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
        let dep_path = checkout_git_dependency(root, name, git, &revision)?;
        return Ok(ResolvedDependency {
            source: format!("git+{git}"),
            revision,
            path: lock_dep_path(root, &dep_path),
        });
    }

    let Some(path) = &dependency.path else {
        return Err("У зависимости не указан источник".into());
    };
    let source_path = resolve_local_source_path(root, path)?;
    let dep_path = copy_local_dependency(root, name, &source_path)?;
    Ok(ResolvedDependency {
        source: format!("path+{}", source_path.display()),
        revision: "local".into(),
        path: lock_dep_path(root, &dep_path),
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
) -> Result<PathBuf, String> {
    let deps_root = dependency_install_root(root)?;
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
    Ok(dep_path)
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

fn copy_local_dependency(root: &Path, name: &str, source_path: &Path) -> Result<PathBuf, String> {
    let deps_root = dependency_install_root(root)?;
    fs::create_dir_all(&deps_root)
        .map_err(|err| format!("Не удалось создать '{}': {err}", deps_root.display()))?;
    let dep_path = deps_root.join(name);
    ensure_inside(&deps_root, &dep_path)?;

    if dep_path.exists() {
        fs::remove_dir_all(&dep_path)
            .map_err(|err| format!("Не удалось обновить '{}': {err}", dep_path.display()))?;
    }

    copy_dir_all(source_path, &dep_path)?;
    Ok(dep_path)
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

fn dependency_install_root(project_root: &Path) -> Result<PathBuf, String> {
    let venv_path = dependency_venv_path(project_root)?;
    let deps_root = venv_path.join("deps");
    if !deps_root.is_dir() {
        return Err(format!(
            "Окружение '{}' повреждено: отсутствует каталог deps",
            venv_path.display()
        ));
    }
    Ok(deps_root)
}

fn dependency_venv_path(project_root: &Path) -> Result<PathBuf, String> {
    match std::env::var(GOIDA_VENV_ENV) {
        Ok(path) => validate_venv_path(PathBuf::from(path)),
        Err(_) => validate_venv_path(project_root.join(".goida"))
            .map_err(|_| "Сначала создайте окружение командой: goida venv".to_string()),
    }
}

fn validate_venv_path(venv_path: PathBuf) -> Result<PathBuf, String> {
    if venv_path.join(VENV_CONFIG_FILE).is_file() || is_legacy_venv(&venv_path) {
        return Ok(venv_path);
    }

    Err(format!(
        "'{}' не является окружением Goida: отсутствует {}",
        venv_path.display(),
        VENV_CONFIG_FILE
    ))
}

fn is_legacy_venv(venv_path: &Path) -> bool {
    venv_path.join("deps").is_dir()
        && (venv_path.join("Scripts/Activate.ps1").is_file()
            || venv_path.join("Scripts/activate.bat").is_file()
            || venv_path.join("bin/activate").is_file())
}

fn lock_dep_path(project_root: &Path, dep_path: &Path) -> String {
    if let Ok(venv_path) = std::env::var(GOIDA_VENV_ENV) {
        let venv_deps = Path::new(&venv_path).join("deps");
        if let Ok(relative) = dep_path.strip_prefix(&venv_deps) {
            return format!(
                "$GOIDA_VENV/deps/{}",
                relative.to_string_lossy().replace('\\', "/")
            );
        }
    }

    dep_path
        .strip_prefix(project_root)
        .map(|relative| relative.to_string_lossy().replace('\\', "/"))
        .unwrap_or_else(|_| dep_path.to_string_lossy().to_string())
}

fn resolve_locked_dep_path(project_root: &Path, locked_path: &str) -> Option<PathBuf> {
    if let Some(rest) = locked_path.strip_prefix("$GOIDA_VENV/") {
        let venv_path = std::env::var(GOIDA_VENV_ENV).ok()?;
        return Some(PathBuf::from(venv_path).join(rest));
    }

    let path = PathBuf::from(locked_path);
    Some(if path.is_absolute() {
        path
    } else {
        project_root.join(path)
    })
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
