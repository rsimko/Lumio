use std::io;
use std::path::PathBuf;

use thiserror::Error;

pub type Result<T> = std::result::Result<T, LumioError>;

#[derive(Debug, Error)]
pub enum LumioError {
    #[error("I/O error at {path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: io::Error,
    },

    #[error("Permission denied: {0}")]
    PermissionDenied(PathBuf),

    #[error("Path does not exist: {0}")]
    NotFound(PathBuf),

    #[error("Not a directory: {0}")]
    NotADirectory(PathBuf),

    #[error("Path is outside the registered library folders: {0}")]
    OutsideRoots(PathBuf),

    #[error("Invalid path: {0}")]
    InvalidPath(String),

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Preview unavailable: {0}")]
    Preview(String),

    #[error("Thumbnail cache error: {0}")]
    Cache(String),

    #[error("Could not move folder to Trash: {0}")]
    Trash(String),

    #[error("Could not move files: {0}")]
    Move(String),

    #[error("Could not move files to Trash: {0}")]
    TrashFiles(String),

    #[error("Ingest error: {0}")]
    Ingest(String),
}

impl LumioError {
    pub fn io(path: impl Into<PathBuf>, source: io::Error) -> Self {
        let path = path.into();
        if source.kind() == io::ErrorKind::PermissionDenied {
            return Self::PermissionDenied(path);
        }
        if source.kind() == io::ErrorKind::NotFound {
            return Self::NotFound(path);
        }
        Self::Io { path, source }
    }

    pub fn code(&self) -> &'static str {
        match self {
            Self::Io { .. } => "io",
            Self::PermissionDenied(_) => "permissionDenied",
            Self::NotFound(_) => "notFound",
            Self::NotADirectory(_) => "notADirectory",
            Self::OutsideRoots(_) => "outsideRoots",
            Self::InvalidPath(_) => "invalidPath",
            Self::Config(_) => "config",
            Self::Preview(_) => "preview",
            Self::Cache(_) => "cache",
            Self::Trash(_) => "trash",
            Self::Move(_) => "move",
            Self::TrashFiles(_) => "trash",
            Self::Ingest(_) => "ingest",
        }
    }
}
