use std::path::PathBuf;

use serde::{Deserialize, Serialize};

pub const CACHE_VERSION: u32 = 2;

/// RAW extensions we recognise as primary assets. Preview support depends on
/// `rawler`, but the association rules apply regardless of decoder support.
pub const RAW_EXTENSIONS: &[&str] = &[
    "3fr", "arw", "cr2", "cr3", "crw", "dcr", "dng", "erf", "gpr", "iiq", "kdc", "mef", "mos",
    "mrw", "nef", "nrw", "orf", "pef", "raf", "raw", "rw2", "rwl", "sr2", "srf", "srw", "x3f",
];

/// Standard image formats displayed as photographs. HEIC/AVIF are deliberately
/// deferred to a later milestone.
pub const IMAGE_EXTENSIONS: &[&str] = &["jpg", "jpeg", "png", "tif", "tiff", "webp"];

pub const JPEG_EXTENSIONS: &[&str] = &["jpg", "jpeg"];

/// Preference order when several RAW variants share a stem. The companion JPEG
/// is attached to the highest-priority RAW; other RAWs use embedded previews.
pub const RAW_PRIORITY: &[&str] = &[
    "cr3", "cr2", "crw", "nef", "nrw", "arw", "sr2", "srf", "raf", "rw2", "orf", "pef", "dng",
    "srw", "rwl", "mrw", "3fr", "mos", "dcr", "kdc", "erf", "mef", "raw", "iiq", "gpr", "x3f",
];

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RootFolder {
    pub id: String,
    pub path: PathBuf,
    pub added_at: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RootInfo {
    pub id: String,
    pub path: PathBuf,
    pub name: String,
    pub available: bool,
    pub error: Option<String>,
    pub added_at: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct FolderNode {
    pub name: String,
    pub path: PathBuf,
    pub has_children: bool,
    pub accessible: bool,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ScanWarning {
    pub path: PathBuf,
    pub message: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum AssetKind {
    Raw,
    Jpeg,
    Png,
    Tiff,
    Webp,
    Other,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum PreviewSource {
    Original,
    CompanionJpeg,
    EmbeddedRaw,
    Placeholder,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PhotoAsset {
    pub id: String,
    pub primary_path: PathBuf,
    pub file_name: String,
    pub stem: String,
    pub extension: String,
    pub kind: AssetKind,
    pub companion_jpeg: Option<PathBuf>,
    pub xmp_sidecar: Option<PathBuf>,
    pub extra_sidecars: Vec<PathBuf>,
    pub extra_companions: Vec<PathBuf>,
    pub preview_hint: PreviewSource,
    pub file_size: u64,
    pub modified_ms: u64,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub relative_dir: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ScanResult {
    pub folder: PathBuf,
    pub recursive: bool,
    pub assets: Vec<PhotoAsset>,
    pub warnings: Vec<ScanWarning>,
    pub elapsed_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ThumbnailResult {
    pub cache_path: Option<PathBuf>,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub source: PreviewSource,
    pub edge: u32,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct UiSettings {
    pub theme: String,
    pub view_mode: String,
    pub recursive: bool,
    pub thumbnail_size: u32,
    pub library_width: u32,
    pub info_width: u32,
    pub last_root_id: Option<String>,
    pub last_folder: Option<PathBuf>,
    pub filmstrip_visible: bool,
}

impl Default for UiSettings {
    fn default() -> Self {
        Self {
            theme: "system".into(),
            view_mode: "grid".into(),
            recursive: false,
            thumbnail_size: 184,
            library_width: 276,
            info_width: 320,
            last_root_id: None,
            last_folder: None,
            filmstrip_visible: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AppConfig {
    pub version: u32,
    pub roots: Vec<RootFolder>,
    pub ui: UiSettings,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            version: 1,
            roots: Vec::new(),
            ui: UiSettings::default(),
        }
    }
}

pub fn extension_of(path: &std::path::Path) -> Option<String> {
    path.extension()
        .map(|value| value.to_string_lossy().to_ascii_lowercase())
}

pub fn is_raw_extension(ext: &str) -> bool {
    RAW_EXTENSIONS.contains(&ext)
}

pub fn is_jpeg_extension(ext: &str) -> bool {
    JPEG_EXTENSIONS.contains(&ext)
}

pub fn is_image_extension(ext: &str) -> bool {
    IMAGE_EXTENSIONS.contains(&ext)
}

pub fn asset_kind_for(ext: &str) -> AssetKind {
    match ext {
        "jpg" | "jpeg" => AssetKind::Jpeg,
        "png" => AssetKind::Png,
        "tif" | "tiff" => AssetKind::Tiff,
        "webp" => AssetKind::Webp,
        ext if is_raw_extension(ext) => AssetKind::Raw,
        _ => AssetKind::Other,
    }
}

pub fn raw_priority(ext: &str) -> usize {
    RAW_PRIORITY
        .iter()
        .position(|candidate| *candidate == ext)
        .unwrap_or(RAW_PRIORITY.len())
}
