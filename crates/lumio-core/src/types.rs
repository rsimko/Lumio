use std::collections::HashMap;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::ingest::IngestSettings;

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

/// How a folder should be removed from Lumio.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum RootRemovalMode {
    /// Forget the folder in Lumio; everything on disk is untouched.
    LibraryOnly,
    /// Move the folder and its contents to the operating system Trash.
    MoveToTrash,
}

/// How one or more photos should be removed from Lumio.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum AssetRemovalMode {
    /// Hide the photo and every associated file in Lumio. Files stay on disk.
    LibraryOnly,
    /// Move the photo and every associated file to the operating system Trash.
    MoveToTrash,
}

/// One hidden photo group. Lumio has no catalogue yet, so "delete from the
/// library" is represented by persisting every file path that belonged to the
/// photo at the time it was hidden.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ExcludedAsset {
    /// Stable identifier derived from the primary path.
    pub id: String,
    pub primary_path: PathBuf,
    /// Display name captured when the photo was hidden.
    pub name: String,
    pub hidden_at: u64,
    /// Primary file plus companion JPEG, XMP sidecar and extra associations.
    pub paths: Vec<PathBuf>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AssetRemovalOutcome {
    pub removed_assets: u32,
    pub hidden_assets: Vec<ExcludedAsset>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct MoveAssetsOutcome {
    pub destination: PathBuf,
    pub moved_assets: u32,
    pub moved_files: u32,
    pub skipped_assets: u32,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TrashAssetsOutcome {
    /// Photo groups whose files were moved to the Trash.
    pub trashed_assets: u32,
    /// Individual files moved to the Trash, including companions and sidecars.
    pub trashed_files: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AddFolderOutcome {
    pub root: RootInfo,
    /// Canonical folder that was added, or the hidden subfolder that was restored.
    pub folder: PathBuf,
    pub restored: bool,
    /// The folder was already covered by a registered root. Ingest uses this to
    /// treat adding a destination folder as a successful no-op.
    #[serde(default)]
    pub already_in_library: bool,
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

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct FolderImageCounts {
    /// Assets directly inside this folder.
    pub direct: u32,
    /// Assets in this folder and every visible descendant folder.
    pub recursive: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct FolderNode {
    pub name: String,
    pub path: PathBuf,
    pub has_children: bool,
    pub accessible: bool,
    pub error: Option<String>,
    /// Assets directly inside this folder.
    pub image_count: u32,
    /// Assets in this folder and every visible descendant folder.
    pub recursive_image_count: u32,
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
pub struct ExternalEditor {
    pub name: String,
    pub path: PathBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct UiSettings {
    pub theme: String,
    #[serde(default = "default_ui_font")]
    pub ui_font: String,
    #[serde(default = "default_mono_font")]
    pub mono_font: String,
    #[serde(default = "default_ui_font_size")]
    pub ui_font_size: u32,
    pub view_mode: String,
    pub recursive: bool,
    pub thumbnail_size: u32,
    pub library_width: u32,
    pub info_width: u32,
    pub last_root_id: Option<String>,
    pub last_folder: Option<PathBuf>,
    pub filmstrip_visible: bool,
    #[serde(default)]
    pub list_show_associations: bool,
    /// User-selected applications offered in the photo context menu.
    #[serde(default)]
    pub external_editors: Vec<ExternalEditor>,
    /// Application used by the Edit shortcut. `None` until one is configured.
    #[serde(default)]
    pub default_editor_path: Option<PathBuf>,
    /// User overrides keyed by shortcut action. Missing actions use the
    /// frontend defaults, which lets new defaults ship without migrations.
    #[serde(default)]
    pub shortcuts: HashMap<String, String>,
}

fn default_ui_font() -> String {
    "avenir".into()
}

fn default_mono_font() -> String {
    "jetbrains".into()
}

fn default_ui_font_size() -> u32 {
    13
}

impl Default for UiSettings {
    fn default() -> Self {
        Self {
            theme: "system".into(),
            ui_font: default_ui_font(),
            mono_font: default_mono_font(),
            ui_font_size: default_ui_font_size(),
            view_mode: "grid".into(),
            recursive: false,
            thumbnail_size: 184,
            library_width: 276,
            info_width: 320,
            last_root_id: None,
            last_folder: None,
            filmstrip_visible: true,
            list_show_associations: false,
            external_editors: Vec::new(),
            default_editor_path: None,
            shortcuts: HashMap::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AppConfig {
    pub version: u32,
    pub roots: Vec<RootFolder>,
    /// Subfolders hidden from Lumio with "remove from library only". The files
    /// remain on disk; re-adding the folder restores it.
    #[serde(default)]
    pub excluded_folders: Vec<PathBuf>,
    /// Photos hidden from Lumio with "delete from library only". Every file in
    /// the group remains on disk and can be restored from Settings.
    #[serde(default)]
    pub excluded_assets: Vec<ExcludedAsset>,
    pub ui: UiSettings,
    /// Ingest templates, conflict defaults and metadata-writing preferences.
    #[serde(default)]
    pub ingest: IngestSettings,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            version: 1,
            roots: Vec::new(),
            excluded_folders: Vec::new(),
            excluded_assets: Vec::new(),
            ui: UiSettings::default(),
            ingest: IngestSettings::default(),
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
