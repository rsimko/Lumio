use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};

use crate::error::{LumioError, Result};
use crate::path_utils::{canonical_dir, is_hidden_or_package};

const MAX_NAME_BYTES: usize = 255;

/// Create a new folder inside an existing directory. The directory is expected
/// to already be validated as part of the registered library.
pub fn create_folder(parent: &Path, name: &str) -> Result<PathBuf> {
    let parent = canonical_dir(parent)?;
    let name = name.trim();
    validate_folder_name(name)?;

    let path = parent.join(name);
    fs::create_dir(&path).map_err(|error| LumioError::io(&path, error))?;
    Ok(path)
}

fn validate_folder_name(name: &str) -> Result<()> {
    if name.is_empty() {
        return Err(LumioError::InvalidPath(
            "Folder names cannot be empty.".into(),
        ));
    }
    if name.len() > MAX_NAME_BYTES {
        return Err(LumioError::InvalidPath(format!(
            "Folder names cannot be longer than {MAX_NAME_BYTES} bytes."
        )));
    }
    if name.contains(['/', '\\', '\0', ':']) {
        return Err(LumioError::InvalidPath(
            "Folder names cannot contain /, \\, : or null characters.".into(),
        ));
    }
    if name.chars().any(char::is_control) {
        return Err(LumioError::InvalidPath(
            "Folder names cannot contain control characters.".into(),
        ));
    }
    if is_hidden_or_package(OsStr::new(name)) {
        return Err(LumioError::InvalidPath(
            "Hidden folders and application/photo packages are not allowed.".into(),
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn creates_folders_and_trims_names() {
        let temp = tempfile::tempdir().unwrap();
        let parent = fs::canonicalize(temp.path()).unwrap();
        let created = create_folder(&parent, "  New Folder  ").unwrap();
        assert_eq!(created, parent.join("New Folder"));
        assert!(created.is_dir());
    }

    #[test]
    fn rejects_invalid_names() {
        let temp = tempfile::tempdir().unwrap();
        for name in [
            "", "   ", ".", "..", ".hidden", "a/b", "a\\b", "a:b", "bad.app",
        ] {
            assert!(
                create_folder(temp.path(), name).is_err(),
                "expected {name:?} to be rejected"
            );
        }
    }

    #[test]
    fn refuses_to_overwrite_an_existing_folder() {
        let temp = tempfile::tempdir().unwrap();
        fs::create_dir(temp.path().join("Exists")).unwrap();
        assert!(create_folder(temp.path(), "Exists").is_err());
    }
}
