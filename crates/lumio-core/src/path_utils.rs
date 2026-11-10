use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::error::{LumioError, Result};

const PACKAGE_SUFFIXES: &[&str] = &[
    ".app",
    ".bundle",
    ".framework",
    ".photoslibrary",
    ".lrlibrary",
    ".lrdata",
    ".captureone",
    ".aplibrary",
];

pub fn canonical_dir(path: &Path) -> Result<PathBuf> {
    let metadata = fs::metadata(path).map_err(|error| LumioError::io(path, error))?;
    if !metadata.is_dir() {
        return Err(LumioError::NotADirectory(path.to_path_buf()));
    }
    fs::canonicalize(path).map_err(|error| LumioError::io(path, error))
}

pub fn canonical_existing(path: &Path) -> Result<PathBuf> {
    fs::canonicalize(path).map_err(|error| LumioError::io(path, error))
}

pub fn is_within_roots(path: &Path, roots: &[PathBuf]) -> bool {
    roots.iter().any(|root| path.starts_with(root))
}

pub fn is_hidden_or_package(name: &OsStr) -> bool {
    let Some(name) = name.to_str() else {
        return false;
    };
    if name.starts_with('.') {
        return true;
    }
    let lower = name.to_ascii_lowercase();
    if lower == "@eadir" || lower == "#recycle" || lower == "__macosx" {
        return true;
    }
    PACKAGE_SUFFIXES
        .iter()
        .any(|suffix| lower.ends_with(suffix))
}

pub fn modified_ms(metadata: &fs::Metadata) -> u64 {
    metadata
        .modified()
        .ok()
        .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
        .map(|duration| duration.as_millis() as u64)
        .unwrap_or(0)
}

pub fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis() as u64)
        .unwrap_or(0)
}

pub fn display_name(path: &Path) -> String {
    path.file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| path.to_string_lossy().into_owned())
}
