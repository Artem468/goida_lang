mod manifest;
mod venv;

pub(crate) use venv::create_venv;

use manifest::{
    read_lock, read_manifest, write_lock, write_manifest, BuildArtifact, BuildConfig, Dependency,
    LockFile, LockedPackage, Manifest, PackageInfo, LOCK_FILE,
};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Component, Path, PathBuf},
    process::Command,
};

use goida_syntax::import_paths::GOIDA_VENV_ENV;

const MANIFEST_FILE: &str = "goida.toml";
const DEPS_DIR: &str = ".goida/deps";
const VENV_CONFIG_FILE: &str = "goida-venv.toml";

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
            entry: Some("главный.goida".to_string()),
        },
        dependencies: BTreeMap::new(),
        build: BuildConfig::default(),
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
    validate_dependency_name(name)?;

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
    let manifest_path = root.join(MANIFEST_FILE);
    let previous_manifest = fs::read_to_string(&manifest_path)
        .map_err(|err| format!("Failed to read '{}': {err}", manifest_path.display()))?;
    let lock_path = root.join(LOCK_FILE);
    let previous_lock = match fs::read_to_string(&lock_path) {
        Ok(content) => Some(content),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => None,
        Err(err) => return Err(format!("Failed to read '{}': {err}", lock_path.display())),
    };
    let mut manifest = read_manifest(&root)?;
    let dependency = Dependency {
        git,
        path,
        rev,
        branch,
        tag,
    };
    manifest.dependencies.insert(name.to_string(), dependency);
    write_manifest(&root, &manifest)?;
    if let Err(error) = sync_dependencies() {
        fs::write(&manifest_path, previous_manifest).map_err(|restore_error| {
            format!(
                "{error}; additionally failed to restore '{}': {restore_error}",
                manifest_path.display()
            )
        })?;
        match previous_lock {
            Some(content) => fs::write(&lock_path, content).map_err(|restore_error| {
                format!(
                    "{error}; additionally failed to restore '{}': {restore_error}",
                    lock_path.display()
                )
            })?,
            None => {
                if let Err(restore_error) = fs::remove_file(&lock_path) {
                    if restore_error.kind() != std::io::ErrorKind::NotFound {
                        return Err(format!(
                            "{error}; additionally failed to remove '{}': {restore_error}",
                            lock_path.display()
                        ));
                    }
                }
            }
        }
        return Err(error);
    }

    println!("Добавлена зависимость '{name}'");
    Ok(())
}

pub(crate) fn sync_dependencies() -> Result<(), String> {
    let root = current_project_root()?;
    let manifest = read_manifest(&root)?;
    let previous_lock = read_lock(&root)?;
    let mut lock = LockFile::default();
    let mut sources = BTreeMap::new();
    let mut resolving = BTreeSet::new();
    sync_manifest_dependencies(
        &root,
        &root,
        &manifest,
        &mut lock,
        &mut sources,
        &mut resolving,
    )?;
    lock.package
        .sort_by(|left, right| left.name.cmp(&right.name));
    remove_stale_dependencies(&root, &previous_lock, &lock)?;
    write_lock(&root, &lock)?;
    println!("Dependencies synchronized");
    Ok(())
}

fn remove_stale_dependencies(
    project_root: &Path,
    previous: &LockFile,
    current: &LockFile,
) -> Result<(), String> {
    let current_names = current
        .package
        .iter()
        .map(|package| package.name.as_str())
        .collect::<BTreeSet<_>>();
    for package in &previous.package {
        if current_names.contains(package.name.as_str()) {
            continue;
        }
        let Some(path) = resolve_locked_dep_path(project_root, &package.path) else {
            continue;
        };
        if path.exists() {
            let install_root = dependency_install_root(project_root)?;
            ensure_inside(&install_root, &path)?;
            fs::remove_dir_all(&path).map_err(|err| {
                format!(
                    "Failed to remove stale dependency '{}': {err}",
                    path.display()
                )
            })?;
        }
    }
    Ok(())
}

fn sync_manifest_dependencies(
    project_root: &Path,
    manifest_root: &Path,
    manifest: &Manifest,
    lock: &mut LockFile,
    sources: &mut BTreeMap<String, String>,
    resolving: &mut BTreeSet<String>,
) -> Result<(), String> {
    for (name, dependency) in &manifest.dependencies {
        validate_dependency_name(name)?;

        let identity = dependency_identity(manifest_root, dependency)?;
        if let Some(existing) = sources.get(name) {
            if existing != &identity {
                return Err(format!(
                    "Dependency name conflict for '{name}': '{existing}' and '{identity}'"
                ));
            }
            continue;
        }
        if !resolving.insert(name.clone()) {
            return Err(format!("Cyclic package dependency detected at '{name}'"));
        }

        let resolved = resolve_dependency(project_root, manifest_root, name, dependency)?;
        let installed_root = resolve_locked_dep_path(project_root, &resolved.path)
            .ok_or_else(|| format!("Failed to resolve installed dependency path for '{name}'"))?;
        sources.insert(name.clone(), identity);
        if installed_root.join(MANIFEST_FILE).is_file() {
            let dependency_manifest = read_manifest(&installed_root)?;
            sync_manifest_dependencies(
                project_root,
                &installed_root,
                &dependency_manifest,
                lock,
                sources,
                resolving,
            )?;
        }
        let artifacts = build_installed_dependency(&installed_root)?;
        lock.package.push(LockedPackage {
            name: name.clone(),
            source: resolved.source,
            revision: resolved.revision,
            path: resolved.path,
            artifacts,
        });
        resolving.remove(name);
    }
    Ok(())
}

fn dependency_identity(root: &Path, dependency: &Dependency) -> Result<String, String> {
    if let Some(git) = &dependency.git {
        return Ok(format!(
            "git+{}#{}",
            git,
            dependency
                .rev
                .as_deref()
                .or(dependency.branch.as_deref())
                .or(dependency.tag.as_deref())
                .unwrap_or("HEAD")
        ));
    }
    let path = dependency
        .path
        .as_deref()
        .ok_or_else(|| "Dependency source is missing".to_string())?;
    Ok(format!(
        "path+{}",
        resolve_local_source_path(root, path)?.display()
    ))
}

pub(crate) fn build_project() -> Result<(), String> {
    sync_dependencies()?;
    let root = current_project_root()?;
    let manifest = read_manifest(&root)?;
    let artifacts = build_package(&root, &manifest)?;
    println!(
        "Built package '{}' for {} ({} artifacts)",
        manifest.package.name,
        current_platform(),
        artifacts.len()
    );
    Ok(())
}

pub(crate) fn remove_dependency(name: &str) -> Result<(), String> {
    validate_dependency_name(name)?;

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

fn starter_source() -> &'static str {
    "функция главная() {\n    печать(\"Привет, мир!\")\n}\n\nглавная()\n"
}

fn resolve_dependency(
    project_root: &Path,
    source_root: &Path,
    name: &str,
    dependency: &Dependency,
) -> Result<ResolvedDependency, String> {
    if let Some(git) = &dependency.git {
        let revision = resolve_git_revision(git, dependency)?;
        let dep_path = checkout_git_dependency(project_root, name, git, &revision)?;
        return Ok(ResolvedDependency {
            source: format!("git+{git}"),
            revision,
            path: lock_dep_path(project_root, &dep_path),
        });
    }

    let Some(path) = &dependency.path else {
        return Err("У зависимости не указан источник".into());
    };
    let source_path = resolve_local_source_path(source_root, path)?;
    let dep_path = copy_local_dependency(project_root, name, &source_path)?;
    Ok(ResolvedDependency {
        source: format!("path+{}", source_path.display()),
        revision: "local".into(),
        path: lock_dep_path(project_root, &dep_path),
    })
}

fn build_installed_dependency(root: &Path) -> Result<Vec<String>, String> {
    let manifest_path = root.join(MANIFEST_FILE);
    if !manifest_path.is_file() {
        return Ok(Vec::new());
    }
    let manifest = read_manifest(root)?;
    build_package(root, &manifest)
}

fn build_package(root: &Path, manifest: &Manifest) -> Result<Vec<String>, String> {
    validate_build_config(&manifest.build)?;
    let selected = manifest
        .build
        .artifacts
        .iter()
        .filter(|artifact| artifact_matches_platform(artifact))
        .collect::<Vec<_>>();

    if !manifest.build.command.is_empty() {
        run_build_command(root, &manifest.build)?;
    }

    let mut installed = Vec::with_capacity(selected.len());
    for artifact in selected {
        let source = resolve_package_path(root, &artifact.source, true)?;
        if !source.is_file() {
            let mode = if manifest.build.command.is_empty() {
                "prebuilt package"
            } else {
                "build command"
            };
            return Err(format!(
                "{mode} '{}' did not provide required artifact '{}' for {}",
                manifest.package.name,
                artifact.source,
                current_platform()
            ));
        }

        let destination = resolve_package_path(root, &artifact.destination, false)?;
        if source != destination {
            prepare_artifact_destination(root, &destination)?;
            fs::copy(&source, &destination).map_err(|err| {
                format!(
                    "Failed to install native artifact '{}' to '{}': {err}",
                    source.display(),
                    destination.display()
                )
            })?;
        }
        installed.push(
            destination
                .strip_prefix(root)
                .map_err(|err| {
                    format!(
                        "Failed to make artifact path '{}' relative to '{}': {err}",
                        destination.display(),
                        root.display()
                    )
                })?
                .to_string_lossy()
                .replace('\\', "/"),
        );
    }
    Ok(installed)
}

fn prepare_artifact_destination(root: &Path, destination: &Path) -> Result<(), String> {
    let parent = destination
        .parent()
        .ok_or_else(|| format!("Invalid artifact destination '{}'", destination.display()))?;
    fs::create_dir_all(parent).map_err(|err| {
        format!(
            "Failed to create artifact directory '{}': {err}",
            parent.display()
        )
    })?;

    let canonical_root = root
        .canonicalize()
        .map_err(|err| format!("Failed to resolve package root '{}': {err}", root.display()))?;
    let canonical_parent = parent.canonicalize().map_err(|err| {
        format!(
            "Failed to resolve artifact directory '{}': {err}",
            parent.display()
        )
    })?;
    if !canonical_parent.starts_with(canonical_root) {
        return Err(format!(
            "Artifact destination '{}' escapes the package",
            destination.display()
        ));
    }
    if fs::symlink_metadata(destination).is_ok_and(|metadata| metadata.file_type().is_symlink()) {
        return Err(format!(
            "Artifact destination '{}' must not be a symbolic link",
            destination.display()
        ));
    }
    Ok(())
}

fn validate_build_config(build: &BuildConfig) -> Result<(), String> {
    if build
        .command
        .first()
        .is_some_and(|program| program.is_empty())
    {
        return Err("build.command program must not be empty".into());
    }
    for artifact in &build.artifacts {
        if artifact.source.is_empty() || artifact.destination.is_empty() {
            return Err("build artifact source and destination must not be empty".into());
        }
    }
    Ok(())
}

fn run_build_command(root: &Path, build: &BuildConfig) -> Result<(), String> {
    let workdir = match &build.workdir {
        Some(path) => resolve_package_path(root, path, true)?,
        None => root.to_path_buf(),
    };
    if !workdir.is_dir() {
        return Err(format!(
            "Build workdir '{}' is not a directory",
            workdir.display()
        ));
    }

    let (program, args) = build
        .command
        .split_first()
        .ok_or_else(|| "build.command must contain a program".to_string())?;
    let status = Command::new(program)
        .args(args)
        .current_dir(&workdir)
        .env("GOIDA_PACKAGE_ROOT", root)
        .env("GOIDA_TARGET_PLATFORM", current_platform())
        .status()
        .map_err(|err| format!("Failed to start build command '{}': {err}", program))?;
    if status.success() {
        Ok(())
    } else {
        Err(format!(
            "Build command for '{}' failed with status {status}",
            root.display()
        ))
    }
}

fn artifact_matches_platform(artifact: &BuildArtifact) -> bool {
    artifact.platforms.is_empty()
        || artifact.platforms.iter().any(|platform| {
            platform == "*" || platform == std::env::consts::OS || platform == &current_platform()
        })
}

fn current_platform() -> String {
    format!("{}-{}", std::env::consts::OS, std::env::consts::ARCH)
}

fn resolve_package_path(root: &Path, path: &str, must_exist: bool) -> Result<PathBuf, String> {
    let relative = Path::new(path);
    if relative.is_absolute()
        || relative
            .components()
            .any(|component| matches!(component, Component::ParentDir | Component::Prefix(_)))
    {
        return Err(format!(
            "Package path '{}' must stay inside the package",
            path
        ));
    }

    let joined = root.join(relative);
    if must_exist {
        let canonical = joined.canonicalize().map_err(|err| {
            format!(
                "Failed to resolve package path '{}': {err}",
                joined.display()
            )
        })?;
        let canonical_root = root
            .canonicalize()
            .map_err(|err| format!("Failed to resolve package root '{}': {err}", root.display()))?;
        if !canonical.starts_with(&canonical_root) {
            return Err(format!("Package path '{}' escapes the package", path));
        }
        Ok(canonical)
    } else {
        Ok(joined)
    }
}

fn current_project_root() -> Result<PathBuf, String> {
    let root = std::env::current_dir().map_err(|err| format!("Failed to get cwd: {err}"))?;
    if root.join(MANIFEST_FILE).is_file() {
        Ok(root)
    } else {
        Err(format!(
            "Current directory does not contain {MANIFEST_FILE}: '{}'",
            root.display()
        ))
    }
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

fn validate_dependency_name(name: &str) -> Result<(), String> {
    if name.is_empty() || name.starts_with('-') {
        return Err(format!("Некорректное имя зависимости '{name}'"));
    }

    let mut components = Path::new(name).components();
    match (components.next(), components.next()) {
        (Some(Component::Normal(component)), None) if component == name => Ok(()),
        _ => Err(format!("Некорректное имя зависимости '{name}'")),
    }
}
