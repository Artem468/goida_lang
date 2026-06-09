use crate::interpreter::prelude::Value;
use libffi::middle::{Arg, Type};
#[cfg(windows)]
use libloading::os::windows::{Library as WindowsLibrary, LOAD_WITH_ALTERED_SEARCH_PATH};
use libloading::Library;
use std::ffi::c_void;
use std::path::{Path, PathBuf};
pub(super) fn native_library_path_candidates(path: &Path) -> Vec<PathBuf> {
    let mut candidates = vec![path.to_path_buf()];
    if path.extension().is_some() {
        return candidates;
    }

    candidates.push(path.with_extension(std::env::consts::DLL_EXTENSION));

    if let (Some(parent), Some(file_name)) = (path.parent(), path.file_name()) {
        let platform_name = format!(
            "{}{}{}",
            std::env::consts::DLL_PREFIX,
            file_name.to_string_lossy(),
            std::env::consts::DLL_SUFFIX
        );
        let platform_path = parent.join(platform_name);
        if !candidates
            .iter()
            .any(|candidate| candidate == &platform_path)
        {
            candidates.push(platform_path);
        }
    }

    candidates
}

#[cfg(test)]
mod tests {
    use super::{native_library_path_candidates, NativeFfiArgValue};
    use crate::interpreter::prelude::Value;
    use std::ffi::c_void;
    use std::path::Path;

    #[test]
    fn native_library_candidates_include_platform_name_for_stem_paths() {
        let candidates = native_library_path_candidates(Path::new("target/debug/demo"));
        let extension_path =
            Path::new("target/debug").join(format!("demo.{}", std::env::consts::DLL_EXTENSION));
        let platform_path = Path::new("target/debug").join(format!(
            "{}demo{}",
            std::env::consts::DLL_PREFIX,
            std::env::consts::DLL_SUFFIX
        ));

        assert!(candidates
            .iter()
            .any(|path| path == Path::new("target/debug/demo")));
        assert!(candidates.iter().any(|path| path == &extension_path));
        assert!(candidates.iter().any(|path| path == &platform_path));
    }

    #[test]
    fn native_library_candidates_keep_explicit_filenames_exact() {
        let path = Path::new("target/debug/demo.custom");

        assert_eq!(
            native_library_path_candidates(path),
            vec![path.to_path_buf()]
        );
    }

    #[test]
    fn managed_pointer_roundtrips_only_for_its_own_address() {
        let mut value = Box::new(Value::Number(42));
        let pointer = (&mut *value as *mut Value).cast::<c_void>();
        let argument = NativeFfiArgValue::ManagedPointer(value, pointer);

        assert_eq!(
            argument.roundtrip_value_for_pointer(pointer),
            Some(Value::Number(42))
        );
        assert_eq!(
            argument.roundtrip_value_for_pointer(std::ptr::null_mut()),
            None
        );
    }
}

#[cfg(windows)]
pub(super) fn load_native_library(path: &Path) -> Result<Library, libloading::Error> {
    unsafe { WindowsLibrary::load_with_flags(path, LOAD_WITH_ALTERED_SEARCH_PATH) }.map(Into::into)
}

#[cfg(not(windows))]
pub(super) fn load_native_library(path: &Path) -> Result<Library, libloading::Error> {
    unsafe { Library::new(path) }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum NativeFfiKind {
    Void,
    I64,
    F64,
    Pointer,
}

impl NativeFfiKind {
    pub(super) fn libffi_type(self) -> Type {
        match self {
            NativeFfiKind::Void => Type::void(),
            NativeFfiKind::I64 => Type::i64(),
            NativeFfiKind::F64 => Type::f64(),
            NativeFfiKind::Pointer => Type::pointer(),
        }
    }
}

#[derive(Debug, Clone)]
pub(super) enum NativeFfiArgValue {
    I64(i64),
    F64(f64),
    Pointer(*mut c_void),
    ManagedPointer(Box<Value>, *mut c_void),
}

impl NativeFfiArgValue {
    pub(super) fn as_arg(&self) -> Arg<'_> {
        match self {
            NativeFfiArgValue::I64(value) => Arg::new(value),
            NativeFfiArgValue::F64(value) => Arg::new(value),
            NativeFfiArgValue::Pointer(value) => Arg::new(value),
            NativeFfiArgValue::ManagedPointer(_, ptr) => Arg::new(ptr),
        }
    }

    pub(super) fn roundtrip_value_for_pointer(&self, pointer: *mut c_void) -> Option<Value> {
        match self {
            NativeFfiArgValue::ManagedPointer(value, ptr) if *ptr == pointer => {
                Some((**value).clone())
            }
            _ => None,
        }
    }
}
