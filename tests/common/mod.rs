use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::OnceLock;

static RELEASE_BINARY: OnceLock<PathBuf> = OnceLock::new();

pub fn goida_command() -> GoidaCommand {
    GoidaCommand {
        command: Command::new(release_binary()),
    }
}

fn release_binary() -> &'static Path {
    RELEASE_BINARY.get_or_init(|| {
        let workspace = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let status = Command::new(env!("CARGO"))
            .current_dir(&workspace)
            .args(["build", "--release", "-p", "goida-cli"])
            .status()
            .expect("failed to build release goida CLI");
        assert!(status.success(), "release goida CLI build failed");

        let target_dir = std::env::var_os("CARGO_TARGET_DIR")
            .map(PathBuf::from)
            .map(|path| {
                if path.is_absolute() {
                    path
                } else {
                    workspace.join(path)
                }
            })
            .unwrap_or_else(|| workspace.join("target"));

        target_dir
            .join("release")
            .join(format!("goida{}", std::env::consts::EXE_SUFFIX))
    })
}

pub struct GoidaCommand {
    command: Command,
}

#[allow(dead_code)]
impl GoidaCommand {
    pub fn args<I, S>(&mut self, args: I) -> &mut Self
    where
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>,
    {
        let args = args
            .into_iter()
            .map(|arg| arg.as_ref().to_os_string())
            .collect::<Vec<_>>();
        let cli_args = args
            .iter()
            .position(|arg| arg == "--")
            .map_or(args.as_slice(), |separator| &args[separator + 1..]);
        self.command.args(cli_args);
        self
    }

    pub fn current_dir<P: AsRef<Path>>(&mut self, dir: P) -> &mut Self {
        self.command.current_dir(dir);
        self
    }

    pub fn env<K, V>(&mut self, key: K, value: V) -> &mut Self
    where
        K: AsRef<OsStr>,
        V: AsRef<OsStr>,
    {
        self.command.env(key, value);
        self
    }

    pub fn output(&mut self) -> std::io::Result<Output> {
        self.command.output()
    }
}
