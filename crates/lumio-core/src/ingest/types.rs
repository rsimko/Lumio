use std::path::PathBuf;

use serde::{Deserialize, Serialize};

/// How XMP sidecars are named next to the copied photo.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub enum XmpNaming {
    /// `NEW.CR3.xmp`
    #[default]
    FilenameExt,
    /// `NEW.xmp`
    Filename,
}

/// One persisted, named destination/filename template pair.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct IngestTemplate {
    pub id: String,
    pub name: String,
    pub folder_template: String,
    pub filename_template: String,
    pub created_at: u64,
    pub updated_at: u64,
}

/// Values written to the destination XMP sidecar (or directly into a
/// standalone JPEG). Every string is a template that can use the same tokens as
/// the filename editor.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", default)]
pub struct IptcTemplates {
    pub title: String,
    pub description: String,
    pub creator: String,
    pub copyright: String,
    pub keywords: Vec<String>,
}

impl IptcTemplates {
    pub fn is_empty(&self) -> bool {
        self.title.trim().is_empty()
            && self.description.trim().is_empty()
            && self.creator.trim().is_empty()
            && self.copyright.trim().is_empty()
            && self
                .keywords
                .iter()
                .all(|keyword| keyword.trim().is_empty())
    }
}

/// Default pattern appended to a name when ingest has to avoid an existing
/// destination file. `#` is replaced by the numeric suffix.
pub const DEFAULT_RENAME_POSTFIX: &str = "_#";

/// Keep rename patterns safe for a single path segment. A missing `#` is
/// appended so the suffix can never repeat the exact same filename.
pub fn normalize_rename_postfix(value: &str) -> String {
    let cleaned: String = value
        .chars()
        .filter(|ch| !ch.is_control() && !matches!(ch, '/' | '\\' | ':'))
        .collect();
    let trimmed = cleaned.trim();
    let mut postfix = if trimmed.is_empty() {
        DEFAULT_RENAME_POSTFIX.to_string()
    } else {
        trimmed.chars().take(32).collect()
    };
    if !postfix.contains('#') {
        postfix.push('#');
    }
    postfix
}

/// Persisted ingest preferences, including named templates.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", default)]
pub struct IngestSettings {
    /// Pre-filled source folder for a new ingest run.
    pub default_source: Option<PathBuf>,
    pub folder_template: String,
    pub filename_template: String,
    pub job_code: String,
    pub custom_text: String,
    pub counter_start: u32,
    pub counter_padding: u32,
    pub xmp_naming: XmpNaming,
    #[serde(default = "default_rename_postfix")]
    pub rename_postfix: String,
    pub exiftool_path: Option<PathBuf>,
    pub add_to_library: bool,
    pub templates: Vec<IngestTemplate>,
    pub iptc: IptcTemplates,
}

fn default_rename_postfix() -> String {
    DEFAULT_RENAME_POSTFIX.to_string()
}

impl Default for IngestSettings {
    fn default() -> Self {
        Self {
            default_source: None,
            folder_template: "{YYYY}/{MM}".into(),
            filename_template: "{YYYY}{MM}{DD}_{default_filename}".into(),
            job_code: String::new(),
            custom_text: String::new(),
            counter_start: 1,
            counter_padding: 4,
            xmp_naming: XmpNaming::FilenameExt,
            rename_postfix: default_rename_postfix(),
            exiftool_path: None,
            add_to_library: true,
            templates: Vec::new(),
            iptc: IptcTemplates::default(),
        }
    }
}

/// Snapshot of the rules used to build one plan. Persisting this with the plan
/// is what lets ingestion execute without rendering templates a second time.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct IngestRules {
    pub folder_template: String,
    pub filename_template: String,
    pub job_code: String,
    pub custom_text: String,
    pub counter_start: u32,
    pub counter_padding: u32,
    pub xmp_naming: XmpNaming,
    #[serde(default = "default_rename_postfix")]
    pub rename_postfix: String,
    pub iptc: IptcTemplates,
}

impl Default for IngestRules {
    fn default() -> Self {
        let settings = IngestSettings::default();
        Self::from_settings(&settings)
    }
}

impl IngestRules {
    pub fn from_settings(settings: &IngestSettings) -> Self {
        Self {
            folder_template: settings.folder_template.clone(),
            filename_template: settings.filename_template.clone(),
            job_code: settings.job_code.clone(),
            custom_text: settings.custom_text.clone(),
            counter_start: settings.counter_start,
            counter_padding: settings.counter_padding,
            xmp_naming: settings.xmp_naming,
            rename_postfix: normalize_rename_postfix(&settings.rename_postfix),
            iptc: settings.iptc.clone(),
        }
    }

    /// Build the stem used when `base` has to be renamed with `suffix`.
    pub fn renamed_stem(&self, base: &str, suffix: u32) -> String {
        let postfix = normalize_rename_postfix(&self.rename_postfix);
        format!("{base}{}", postfix.replace('#', &suffix.to_string()))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct IngestRequest {
    /// Folders are scanned recursively; individual files bring their associated
    /// RAW/JPEG/XMP companions along.
    pub sources: Vec<PathBuf>,
    pub destination: PathBuf,
    pub rules: IngestRules,
    pub exiftool_path: Option<PathBuf>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ExifToolInfo {
    pub path: PathBuf,
    pub version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct IngestTools {
    pub exiftool: Option<ExifToolInfo>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum IngestFileRole {
    Raw,
    Jpeg,
    Xmp,
}

impl IngestFileRole {
    pub fn is_photo(self) -> bool {
        matches!(self, Self::Raw | Self::Jpeg)
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum CaptureSource {
    RawExif,
    JpegExif,
    Xmp,
    FileModified,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CaptureInfo {
    /// Local wall-clock time in `YYYY-MM-DD HH:MM:SS` form.
    pub display: String,
    pub timestamp_ms: i64,
    pub source: CaptureSource,
    pub warning: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct IngestSourceFile {
    pub path: PathBuf,
    pub role: IngestFileRole,
    pub size: u64,
    pub modified_ms: u64,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum IngestConflictResolution {
    Skip,
    KeepBoth,
    Replace,
}

impl Default for IngestConflictResolution {
    fn default() -> Self {
        Self::Skip
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum IngestEntityStatus {
    Ready,
    Conflict,
    Error,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum IngestMetadataTargetKind {
    /// Existing or newly created XMP sidecar next to a RAW.
    XmpSidecar,
    /// Metadata is written directly into a standalone JPEG.
    Jpeg,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct IngestMetadataTarget {
    pub path: PathBuf,
    pub kind: IngestMetadataTargetKind,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct IngestPlannedFile {
    /// `None` for an XMP sidecar generated solely from IPTC templates.
    pub source: Option<PathBuf>,
    pub destination: PathBuf,
    pub role: IngestFileRole,
    pub size: u64,
    pub primary: bool,
    pub generated: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct IngestConflict {
    pub existing: Vec<PathBuf>,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct IngestIptcValues {
    pub title: Option<String>,
    pub description: Option<String>,
    pub creator: Option<String>,
    pub copyright: Option<String>,
    pub keywords: Vec<String>,
}

impl IngestIptcValues {
    pub fn is_empty(&self) -> bool {
        let blank = |value: &Option<String>| {
            value
                .as_ref()
                .map(|value| value.trim().is_empty())
                .unwrap_or(true)
        };
        blank(&self.title)
            && blank(&self.description)
            && blank(&self.creator)
            && blank(&self.copyright)
            && self
                .keywords
                .iter()
                .all(|keyword| keyword.trim().is_empty())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct IngestEntity {
    pub id: String,
    pub source_dir: PathBuf,
    pub stem: String,
    pub display_name: String,
    pub primary_path: PathBuf,
    pub primary_role: IngestFileRole,
    pub files: Vec<IngestSourceFile>,
    pub capture: CaptureInfo,
    /// Destination folder and stem before conflict renames. Rebuilding the plan
    /// from these values never needs to rescan or re-render a template.
    pub base_destination_dir: PathBuf,
    pub base_destination_stem: String,
    pub destination_dir: PathBuf,
    pub destination_stem: String,
    pub planned_files: Vec<IngestPlannedFile>,
    pub metadata_target: Option<IngestMetadataTarget>,
    pub status: IngestEntityStatus,
    pub resolution: IngestConflictResolution,
    pub conflict: Option<IngestConflict>,
    pub renamed: bool,
    pub rename_reason: Option<String>,
    pub warnings: Vec<String>,
    pub iptc: IngestIptcValues,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct IngestPlanSummary {
    pub entities: u32,
    pub files: u32,
    pub bytes: u64,
    pub conflicts: u32,
    pub renamed: u32,
    pub warnings: u32,
    pub errors: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct IngestWarning {
    pub path: Option<PathBuf>,
    pub entity_id: Option<String>,
    pub message: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum IngestLogLevel {
    Info,
    Warning,
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct IngestLogEntry {
    pub at: String,
    pub level: IngestLogLevel,
    pub message: String,
    pub path: Option<PathBuf>,
    pub entity_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct IngestPlan {
    pub id: String,
    pub created_at: u64,
    pub sources: Vec<PathBuf>,
    pub destination: PathBuf,
    pub rules: IngestRules,
    /// The resolved ExifTool path used by this plan.
    pub exiftool_path: Option<PathBuf>,
    /// The ExifTool setting at Dry Run time. `None` means auto-detect. This is
    /// kept separately so the UI can detect setting changes without having to
    /// know which executable auto-detection picked.
    pub requested_exiftool_path: Option<PathBuf>,
    pub tools: IngestTools,
    pub entities: Vec<IngestEntity>,
    pub warnings: Vec<IngestWarning>,
    pub log: Vec<IngestLogEntry>,
    pub summary: IngestPlanSummary,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum IngestResultStatus {
    Copied,
    Skipped,
    Failed,
    Cancelled,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct IngestEntityResult {
    pub entity_id: String,
    pub display_name: String,
    pub status: IngestResultStatus,
    pub files: u32,
    pub bytes: u64,
    pub destination: Option<PathBuf>,
    pub message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct IngestOutcome {
    pub plan_id: String,
    pub destination: PathBuf,
    pub copied_entities: u32,
    pub skipped_entities: u32,
    pub failed_entities: u32,
    pub copied_files: u32,
    pub copied_bytes: u64,
    pub cancelled: bool,
    pub elapsed_ms: u64,
    pub results: Vec<IngestEntityResult>,
    pub log: Vec<IngestLogEntry>,
}

/// Progress events streamed to the webview while a plan runs.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(
    tag = "type",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum IngestEvent {
    Started {
        total_entities: u32,
        total_files: u32,
        total_bytes: u64,
    },
    EntityStarted {
        entity_id: String,
        display_name: String,
        index: u32,
        total: u32,
    },
    FileCopied {
        entity_id: String,
        source: PathBuf,
        destination: PathBuf,
        bytes: u64,
        index: u32,
        total: u32,
    },
    EntityCompleted {
        entity_id: String,
        display_name: String,
        destination: PathBuf,
        files: u32,
        bytes: u64,
    },
    EntitySkipped {
        entity_id: String,
        display_name: String,
        reason: String,
    },
    EntityFailed {
        entity_id: String,
        display_name: String,
        message: String,
    },
    Warning {
        entity_id: Option<String>,
        message: String,
    },
    Log {
        entry: IngestLogEntry,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ingest_settings_default_to_the_expected_rename_postfix() {
        let settings = IngestSettings::default();
        assert_eq!(settings.rename_postfix, DEFAULT_RENAME_POSTFIX);
        assert_eq!(settings.default_source, None);

        let rules = IngestRules::from_settings(&settings);
        assert_eq!(rules.renamed_stem("IMG_0001", 2), "IMG_0001_2");
    }

    #[test]
    fn rename_postfix_is_normalized_to_a_safe_numbered_pattern() {
        assert_eq!(normalize_rename_postfix(""), DEFAULT_RENAME_POSTFIX);
        assert_eq!(normalize_rename_postfix("   "), DEFAULT_RENAME_POSTFIX);
        assert_eq!(normalize_rename_postfix("_v"), "_v#");
        assert_eq!(normalize_rename_postfix(" /bad\\:_# "), "bad_#");
        assert_eq!(normalize_rename_postfix(" ## "), "##");
    }

    #[test]
    fn ingest_events_use_camel_case_fields_for_the_frontend() {
        let event = IngestEvent::EntityStarted {
            entity_id: "entity-1".into(),
            display_name: "palawan-112.RAF".into(),
            index: 0,
            total: 3,
        };
        let value = serde_json::to_value(event).expect("event serializes");
        assert_eq!(value["type"], "entityStarted");
        assert_eq!(value["entityId"], "entity-1");
        assert_eq!(value["displayName"], "palawan-112.RAF");

        let started = serde_json::to_value(IngestEvent::Started {
            total_entities: 3,
            total_files: 6,
            total_bytes: 12_345,
        })
        .expect("started event serializes");
        assert_eq!(started["type"], "started");
        assert_eq!(started["totalEntities"], 3);
        assert_eq!(started["totalFiles"], 6);
        assert_eq!(started["totalBytes"], 12_345);
    }

    #[test]
    fn ingest_events_round_trip_through_camel_case_json() {
        let value = serde_json::json!({
            "type": "entityCompleted",
            "entityId": "entity-1",
            "displayName": "palawan-112.RAF",
            "destination": "/photos/2026/11",
            "files": 3,
            "bytes": 4096
        });
        let event: IngestEvent = serde_json::from_value(value).expect("event deserializes");
        match event {
            IngestEvent::EntityCompleted {
                entity_id,
                display_name,
                files,
                ..
            } => {
                assert_eq!(entity_id, "entity-1");
                assert_eq!(display_name, "palawan-112.RAF");
                assert_eq!(files, 3);
            }
            other => panic!("unexpected event: {other:?}"),
        }
    }
}
