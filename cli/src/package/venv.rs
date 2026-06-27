use std::{fs, path::PathBuf};

const VENV_CONFIG_FILE: &str = "goida-venv.toml";

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
