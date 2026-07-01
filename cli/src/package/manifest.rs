use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, fs, path::Path};

use super::MANIFEST_FILE;

pub(super) const LOCK_FILE: &str = "goida.lock";

#[derive(Debug, Serialize, Deserialize)]
pub(super) struct Manifest {
    pub(super) package: PackageInfo,
    #[serde(default)]
    pub(super) dependencies: BTreeMap<String, Dependency>,
    #[serde(default)]
    pub(super) build: BuildConfig,
}

#[derive(Debug, Serialize, Deserialize)]
pub(super) struct PackageInfo {
    pub(super) name: String,
    #[serde(default)]
    pub(super) description: String,
    pub(super) version: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(super) entry: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub(super) struct BuildConfig {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub(super) command: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(super) workdir: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub(super) artifacts: Vec<BuildArtifact>,
}

#[derive(Debug, Serialize, Deserialize)]
pub(super) struct BuildArtifact {
    pub(super) source: String,
    pub(super) destination: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub(super) platforms: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub(super) struct Dependency {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) git: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) rev: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) branch: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) tag: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub(super) struct LockFile {
    #[serde(default = "lock_version")]
    pub(super) version: u32,
    #[serde(default)]
    pub(super) package: Vec<LockedPackage>,
}

impl Default for LockFile {
    fn default() -> Self {
        Self {
            version: lock_version(),
            package: Vec::new(),
        }
    }
}

const fn lock_version() -> u32 {
    1
}

#[derive(Debug, Serialize, Deserialize)]
pub(super) struct LockedPackage {
    pub(super) name: String,
    pub(super) source: String,
    pub(super) revision: String,
    pub(super) path: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub(super) artifacts: Vec<String>,
}

pub(super) fn read_manifest(root: &Path) -> Result<Manifest, String> {
    let path = root.join(MANIFEST_FILE);
    let content = fs::read_to_string(&path)
        .map_err(|err| format!("Не удалось прочитать '{}': {err}", path.display()))?;
    toml::from_str(&content).map_err(|err| format!("Некорректный {MANIFEST_FILE}: {err}"))
}

pub(super) fn write_manifest(root: &Path, manifest: &Manifest) -> Result<(), String> {
    let content = toml::to_string_pretty(manifest)
        .map_err(|err| format!("Не удалось сериализовать {MANIFEST_FILE}: {err}"))?;
    fs::write(root.join(MANIFEST_FILE), content)
        .map_err(|err| format!("Не удалось записать {MANIFEST_FILE}: {err}"))
}

pub(super) fn read_lock(root: &Path) -> Result<LockFile, String> {
    let path = root.join(LOCK_FILE);
    if !path.exists() {
        return Ok(LockFile::default());
    }
    let content = fs::read_to_string(&path)
        .map_err(|err| format!("Не удалось прочитать '{}': {err}", path.display()))?;
    let lock: LockFile =
        toml::from_str(&content).map_err(|err| format!("Некорректный {LOCK_FILE}: {err}"))?;
    if lock.version != lock_version() {
        return Err(format!(
            "Unsupported {LOCK_FILE} version {}; expected {}",
            lock.version,
            lock_version()
        ));
    }
    Ok(lock)
}

pub(super) fn write_lock(root: &Path, lock: &LockFile) -> Result<(), String> {
    let content = toml::to_string_pretty(lock)
        .map_err(|err| format!("Не удалось сериализовать {LOCK_FILE}: {err}"))?;
    fs::write(root.join(LOCK_FILE), content)
        .map_err(|err| format!("Не удалось записать {LOCK_FILE}: {err}"))
}
