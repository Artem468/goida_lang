use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};

mod common;

#[test]
fn package_manager_creates_project_and_updates_git_dependencies() {
    let workspace = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let temp = workspace.join("target/package_manager_test");
    if temp.exists() {
        fs::remove_dir_all(&temp).expect("failed to clear package manager test directory");
    }
    fs::create_dir_all(&temp).expect("failed to create package manager test directory");

    let dep_repo = temp.join("dep_repo");
    create_git_dependency(&dep_repo);

    let new_output = run_goida(&workspace, &temp, &["new", "demo", "--description", "test"]);
    assert_success(&new_output, "goida new");

    let project = temp.join("demo");
    let venv_output = run_goida(&workspace, &project, &["venv"]);
    assert_success(&venv_output, "goida venv");

    let main_source =
        fs::read_to_string(project.join("главный.goida")).expect("missing generated main file");
    assert!(main_source.contains("функция главная()"));
    assert!(main_source.contains("главная()"));

    let manifest = fs::read_to_string(project.join("goida.toml")).expect("missing manifest");
    assert!(manifest.contains("name = \"demo\""));
    assert!(manifest.contains("description = \"test\""));
    assert!(manifest.contains("version = \"0.1.0\""));

    let add_output = run_goida(
        &workspace,
        &project,
        &["add", "dep", "--git", dep_repo.to_str().unwrap()],
    );
    assert_success(&add_output, "goida add");

    let manifest = fs::read_to_string(project.join("goida.toml")).expect("missing manifest");
    assert!(manifest.contains("[dependencies.dep]"));
    assert!(manifest.contains("git = "));
    assert!(manifest.contains("dep_repo"));

    let lock = fs::read_to_string(project.join("goida.lock")).expect("missing lock");
    assert!(lock.contains("[[package]]"));
    assert!(lock.contains("name = \"dep\""));
    assert!(lock.contains("source = "));
    assert!(lock.contains("git+"));
    assert!(lock.contains("revision = \""));
    assert!(project.join(".goida/deps/dep/mod.goida").exists());

    let remove_output = run_goida(&workspace, &project, &["remove", "dep"]);
    assert_success(&remove_output, "goida remove");

    let manifest = fs::read_to_string(project.join("goida.toml")).expect("missing manifest");
    assert!(!manifest.contains("[dependencies.dep]"));
    let lock = fs::read_to_string(project.join("goida.lock")).expect("missing lock");
    assert!(!lock.contains("name = \"dep\""));
    assert!(!project.join(".goida/deps/dep").exists());
}

#[test]
fn package_manager_adds_local_path_dependencies() {
    let workspace = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let temp = workspace.join("target/package_manager_path_test");
    if temp.exists() {
        fs::remove_dir_all(&temp).expect("failed to clear path dependency test directory");
    }
    fs::create_dir_all(&temp).expect("failed to create path dependency test directory");

    let local_dep = temp.join("local_dep");
    fs::create_dir_all(local_dep.join("target/debug"))
        .expect("failed to create local dependency directory");
    fs::write(local_dep.join("главный.goida"), "значение = 1\n")
        .expect("failed to write local dependency module");
    fs::write(local_dep.join("target/debug/beacon.dll"), "native")
        .expect("failed to write local native artifact");

    let new_output = run_goida(&workspace, &temp, &["new", "demo"]);
    assert_success(&new_output, "goida new");

    let project = temp.join("demo");
    let venv_output = run_goida(&workspace, &project, &["venv"]);
    assert_success(&venv_output, "goida venv");

    let add_output = run_goida(
        &workspace,
        &project,
        &["add", "harpoon", "--path", local_dep.to_str().unwrap()],
    );
    assert_success(&add_output, "goida add --path");

    let manifest = fs::read_to_string(project.join("goida.toml")).expect("missing manifest");
    assert!(manifest.contains("[dependencies.harpoon]"));
    assert!(manifest.contains("path = "));
    assert!(manifest.contains("local_dep"));

    let lock = fs::read_to_string(project.join("goida.lock")).expect("missing lock");
    assert!(lock.contains("name = \"harpoon\""));
    assert!(lock.contains("path+"));
    assert!(lock.contains("revision = \"local\""));
    assert!(project.join(".goida/deps/harpoon/главный.goida").exists());
    assert!(project
        .join(".goida/deps/harpoon/target/debug/beacon.dll")
        .exists());
    assert!(!project.join(".goida/deps/harpoon/.git").exists());
}

#[test]
fn package_manager_rejects_add_without_venv() {
    let workspace = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let temp = workspace.join("target/package_manager_requires_venv_test");
    if temp.exists() {
        fs::remove_dir_all(&temp).expect("failed to clear requires venv test directory");
    }
    fs::create_dir_all(&temp).expect("failed to create requires venv test directory");

    let local_dep = temp.join("local_dep");
    fs::create_dir_all(&local_dep).expect("failed to create local dependency");
    fs::write(local_dep.join("mod.goida"), "значение = 7\n")
        .expect("failed to write local dependency module");

    let new_output = run_goida(&workspace, &temp, &["new", "demo"]);
    assert_success(&new_output, "goida new");
    let project = temp.join("demo");

    let add_output = run_goida(
        &workspace,
        &project,
        &["add", "lib", "--path", local_dep.to_str().unwrap()],
    );
    assert_failure(&add_output, "goida add without venv");
    assert!(!project.join(".goida/deps/lib").exists());
}

#[test]
fn package_manager_accepts_legacy_active_venv_without_config() {
    let workspace = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let temp = workspace.join("target/package_manager_legacy_venv_test");
    if temp.exists() {
        fs::remove_dir_all(&temp).expect("failed to clear legacy venv test directory");
    }
    fs::create_dir_all(&temp).expect("failed to create legacy venv test directory");

    let local_dep = temp.join("local_dep");
    fs::create_dir_all(&local_dep).expect("failed to create local dependency");
    fs::write(local_dep.join("mod.goida"), "значение = 9\n")
        .expect("failed to write local dependency module");

    let new_output = run_goida(&workspace, &temp, &["new", "demo"]);
    assert_success(&new_output, "goida new");
    let project = temp.join("demo");
    let venv = project.join(".goida");
    fs::create_dir_all(venv.join("deps")).expect("failed to create legacy deps");
    fs::create_dir_all(venv.join("Scripts")).expect("failed to create legacy scripts");
    fs::write(
        venv.join("Scripts/Activate.ps1"),
        "$env:GOIDA_VENV = '.goida'\n",
    )
    .expect("failed to write legacy activation script");

    let add_output = run_goida_with_env(
        &workspace,
        &project,
        &["add", "lib", "--path", local_dep.to_str().unwrap()],
        &[("GOIDA_VENV", venv.to_str().unwrap())],
    );
    assert_success(&add_output, "goida add with legacy active venv");
    assert!(venv.join("deps/lib/mod.goida").exists());
}

#[test]
fn package_manager_creates_venv_and_installs_active_dependencies_there() {
    let workspace = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let temp = workspace.join("target/package_manager_venv_test");
    if temp.exists() {
        fs::remove_dir_all(&temp).expect("failed to clear venv test directory");
    }
    fs::create_dir_all(&temp).expect("failed to create venv test directory");

    let new_output = run_goida(&workspace, &temp, &["new", "demo"]);
    assert_success(&new_output, "goida new");
    let project = temp.join("demo");
    let venv = project.join(".goida");

    let venv_output = run_goida(&workspace, &project, &["venv"]);
    assert_success(&venv_output, "goida venv");
    assert!(venv.join("deps").exists());
    assert!(venv.join("Scripts/Activate.ps1").exists());
    assert!(venv.join("Scripts/Deactivate.ps1").exists());
    assert!(venv.join("Scripts/activate.bat").exists());
    assert!(venv.join("Scripts/deactivate.bat").exists());
    assert!(venv.join("bin/activate").exists());
    assert!(venv.join("bin/deactivate").exists());
    let powershell_activate =
        fs::read_to_string(venv.join("Scripts/Activate.ps1")).expect("missing Activate.ps1");
    assert!(powershell_activate.contains("(.goida) "));
    assert!(powershell_activate.contains("function global:prompt"));
    assert!(powershell_activate.contains("Function:_goida_old_prompt"));
    let cmd_activate =
        fs::read_to_string(venv.join("Scripts/activate.bat")).expect("missing activate.bat");
    assert!(cmd_activate.contains("PROMPT=(.goida) %PROMPT%"));
    assert!(cmd_activate.contains("GOIDA_OLD_PROMPT"));
    let sh_activate = fs::read_to_string(venv.join("bin/activate")).expect("missing bin/activate");
    assert!(sh_activate.contains("export PS1=\"(.goida) ${PS1-}\""));
    assert!(sh_activate.contains("GOIDA_OLD_PS1"));

    let local_dep = temp.join("local_dep");
    fs::create_dir_all(&local_dep).expect("failed to create local dependency");
    fs::write(local_dep.join("mod.goida"), "значение = 41\n")
        .expect("failed to write local dependency module");

    let add_output = run_goida_with_env(
        &workspace,
        &project,
        &["add", "lib", "--path", local_dep.to_str().unwrap()],
        &[("GOIDA_VENV", venv.to_str().unwrap())],
    );
    assert_success(&add_output, "goida add with active venv");
    assert!(venv.join("deps/lib/mod.goida").exists());

    let lock = fs::read_to_string(project.join("goida.lock")).expect("missing lock");
    assert!(lock.contains("path = \"$GOIDA_VENV/deps/lib\""));

    fs::write(
        project.join("главный.goida"),
        "подключить \"lib/mod\" как m\nпечать(m.значение)\n",
    )
    .expect("failed to write project main");
    let run_output = run_goida_with_env(
        &workspace,
        &project,
        &["run", "главный.goida"],
        &[("GOIDA_VENV", venv.to_str().unwrap())],
    );
    assert_success(&run_output, "goida run with active venv import");
    assert_eq!("41\n", String::from_utf8_lossy(&run_output.stdout));
}

#[test]
fn package_manager_installs_declared_prebuilt_native_artifacts() {
    let workspace = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let temp = fresh_test_dir(&workspace, "package_manager_prebuilt_native_test");
    let dependency = temp.join("native_dep");
    let library_name = native_library_name("beacon");
    fs::create_dir_all(dependency.join("prebuilt")).expect("failed to create prebuilt directory");
    fs::write(dependency.join("mod.goida"), "value = 1\n").expect("failed to write module");
    fs::write(dependency.join("prebuilt").join(&library_name), "native")
        .expect("failed to write prebuilt library");
    write_package_manifest(
        &dependency,
        &format!(
            r#"[package]
name = "native-dep"
description = ""
version = "0.1.0"

[[build.artifacts]]
source = "prebuilt/{library_name}"
destination = "native/{library_name}"
platforms = ["{}"]
"#,
            std::env::consts::OS
        ),
    );

    let project = create_project_with_venv(&workspace, &temp);
    let output = run_goida(
        &workspace,
        &project,
        &["add", "native_dep", "--path", dependency.to_str().unwrap()],
    );
    assert_success(&output, "goida add prebuilt native dependency");
    assert!(project
        .join(".goida/deps/native_dep/native")
        .join(&library_name)
        .is_file());
    let lock = fs::read_to_string(project.join("goida.lock")).expect("missing lock");
    assert!(lock.contains(&format!("native/{library_name}")));
}

#[test]
fn package_manager_builds_native_dependency_and_syncs_transitive_dependencies() {
    let workspace = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let temp = fresh_test_dir(&workspace, "package_manager_native_build_test");
    let dependency = temp.join("native_dep");
    let transitive = dependency.join("vendor/helper");
    let library_name = native_library_name("generated");

    fs::create_dir_all(transitive.join("src")).expect("failed to create transitive dependency");
    fs::write(transitive.join("mod.goida"), "value = 7\n").expect("failed to write helper module");
    write_package_manifest(
        &transitive,
        "[package]\nname = \"helper\"\ndescription = \"\"\nversion = \"0.1.0\"\n",
    );

    fs::create_dir_all(dependency.join("src")).expect("failed to create native dependency");
    fs::write(dependency.join("src/lib.rs"), "").expect("failed to write Rust library");
    fs::write(
        dependency.join("build.rs"),
        format!(
            r#"fn main() {{
    let root = std::path::PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());
    std::fs::create_dir_all(root.join("generated")).unwrap();
    std::fs::write(root.join("generated/{library_name}"), b"native").unwrap();
}}
"#
        ),
    )
    .expect("failed to write build script");
    fs::write(
        dependency.join("Cargo.toml"),
        "[package]\nname = \"goida_native_build_test\"\nversion = \"0.1.0\"\nedition = \"2021\"\nbuild = \"build.rs\"\n\n[workspace]\n",
    )
    .expect("failed to write Cargo manifest");
    write_package_manifest(
        &dependency,
        &format!(
            r#"[package]
name = "native-dep"
description = ""
version = "0.1.0"

[dependencies.helper]
path = "vendor/helper"

[build]
command = ["cargo", "build", "--quiet"]

[[build.artifacts]]
source = "generated/{library_name}"
destination = "native/{library_name}"
"#
        ),
    );

    let project = create_project_with_venv(&workspace, &temp);
    fs::write(
        project.join("goida.toml"),
        format!(
            r#"[package]
name = "demo"
description = ""
version = "0.1.0"

[dependencies.native_dep]
path = "{}"
"#,
            dependency.to_string_lossy().replace('\\', "/")
        ),
    )
    .expect("failed to update project manifest");

    let sync = run_goida(&workspace, &project, &["sync"]);
    assert_success(&sync, "goida sync");
    assert!(project
        .join(".goida/deps/native_dep/native")
        .join(&library_name)
        .is_file());
    assert!(project.join(".goida/deps/helper/mod.goida").is_file());

    fs::remove_dir_all(project.join(".goida/deps/native_dep"))
        .expect("failed to remove installed dependency");
    let build = run_goida(&workspace, &project, &["build"]);
    assert_success(&build, "goida build");
    assert!(project
        .join(".goida/deps/native_dep/native")
        .join(&library_name)
        .is_file());
}

#[test]
fn package_manager_rejects_missing_prebuilt_native_artifact() {
    let workspace = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let temp = fresh_test_dir(&workspace, "package_manager_missing_native_test");
    let dependency = temp.join("native_dep");
    fs::create_dir_all(&dependency).expect("failed to create dependency");
    write_package_manifest(
        &dependency,
        r#"[package]
name = "native-dep"
description = ""
version = "0.1.0"

[[build.artifacts]]
source = "prebuilt/missing.dll"
destination = "native/missing.dll"
"#,
    );

    let project = create_project_with_venv(&workspace, &temp);
    let output = run_goida(
        &workspace,
        &project,
        &["add", "native_dep", "--path", dependency.to_str().unwrap()],
    );
    assert_failure(&output, "goida add missing native dependency");
}

fn fresh_test_dir(workspace: &Path, name: &str) -> PathBuf {
    let temp = workspace.join("target").join(name);
    if temp.exists() {
        fs::remove_dir_all(&temp).expect("failed to clear test directory");
    }
    fs::create_dir_all(&temp).expect("failed to create test directory");
    temp
}

fn create_project_with_venv(workspace: &Path, temp: &Path) -> PathBuf {
    let output = run_goida(workspace, temp, &["new", "demo"]);
    assert_success(&output, "goida new");
    let project = temp.join("demo");
    let output = run_goida(workspace, &project, &["venv"]);
    assert_success(&output, "goida venv");
    project
}

fn write_package_manifest(root: &Path, content: &str) {
    fs::write(root.join("goida.toml"), content).expect("failed to write goida manifest");
}

fn native_library_name(stem: &str) -> String {
    match std::env::consts::OS {
        "windows" => format!("{stem}.dll"),
        "macos" => format!("lib{stem}.dylib"),
        _ => format!("lib{stem}.so"),
    }
}

fn create_git_dependency(path: &Path) {
    fs::create_dir_all(path).expect("failed to create git dependency directory");
    run_git(path, &["init"]);
    run_git(path, &["config", "user.email", "test@example.com"]);
    run_git(path, &["config", "user.name", "Test User"]);
    fs::write(path.join("mod.goida"), "значение = 1\n").expect("failed to write dependency file");
    run_git(path, &["add", "."]);
    run_git(path, &["commit", "-m", "initial"]);
}

fn run_goida(workspace: &Path, cwd: &Path, args: &[&str]) -> std::process::Output {
    run_goida_with_env(workspace, cwd, args, &[])
}

fn run_goida_with_env(
    workspace: &Path,
    cwd: &Path,
    args: &[&str],
    envs: &[(&str, &str)],
) -> std::process::Output {
    let manifest_path = workspace.join("cli/Cargo.toml");
    let mut command_args = vec![
        "run",
        "-q",
        "--manifest-path",
        manifest_path.to_str().unwrap(),
        "--",
    ];
    command_args.extend_from_slice(args);

    let mut command = common::goida_command();
    command.current_dir(cwd).args(command_args);
    for (key, value) in envs {
        command.env(key, value);
    }
    command.output().expect("failed to run goida cli")
}

fn run_git(cwd: &Path, args: &[&str]) {
    let output = Command::new("git")
        .current_dir(cwd)
        .args(args)
        .output()
        .expect("failed to run git");
    assert_success(&output, "git");
}

fn assert_success(output: &std::process::Output, label: &str) {
    assert!(
        output.status.success(),
        "{label} failed\nSTDOUT: {}\nSTDERR: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

fn assert_failure(output: &std::process::Output, label: &str) {
    assert!(
        !output.status.success(),
        "{label} unexpectedly succeeded\nSTDOUT: {}\nSTDERR: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}
