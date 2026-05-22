use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};

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
    let main_source =
        fs::read_to_string(project.join("главный.goida")).expect("missing generated main file");
    assert!(main_source.contains("функция привет_мир()"));
    assert!(main_source.contains("привет_мир()"));

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
        "подключить \"lib/mod\" в m\nпечать(m.значение)\n",
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

    let mut command = Command::new("cargo");
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
