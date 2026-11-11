use std::collections::HashMap;
use std::io::BufReader;
use std::path::{Path, PathBuf};
use std::process::Command;

use chrono::{Local, NaiveDate, NaiveDateTime, TimeZone, Timelike};
use serde_json::Value as JsonValue;

use crate::error::{LumioError, Result};
use crate::ingest::types::{
    CaptureInfo, CaptureSource, ExifToolInfo, IngestFileRole, IngestSourceFile, IngestTools,
};
use crate::path_utils::canonical_existing;

const EXIFTOOL_CANDIDATES: &[&str] = &[
    "exiftool",
    "/opt/homebrew/bin/exiftool",
    "/usr/local/bin/exiftool",
    "/usr/bin/exiftool",
];

pub fn detect_tools(configured: Option<&Path>) -> IngestTools {
    IngestTools {
        exiftool: discover_exiftool(configured),
    }
}

pub fn discover_exiftool(configured: Option<&Path>) -> Option<ExifToolInfo> {
    let mut candidates: Vec<PathBuf> = Vec::new();
    if let Some(path) = configured {
        candidates.push(path.to_path_buf());
    }
    candidates.extend(EXIFTOOL_CANDIDATES.iter().map(PathBuf::from));

    for candidate in candidates {
        // A bare command name is resolved through PATH by the OS. Absolute
        // paths must exist before spawning so packaged apps get a useful nil.
        if candidate.components().count() > 1 || candidate.is_absolute() {
            if !candidate.is_file() {
                continue;
            }
        }
        let Ok(output) = Command::new(&candidate).arg("-ver").output() else {
            continue;
        };
        if !output.status.success() {
            continue;
        }
        let version = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if version.is_empty() {
            continue;
        }
        let path = if candidate.is_absolute() {
            candidate
                .canonicalize()
                .unwrap_or_else(|_| candidate.clone())
        } else {
            find_in_path(&candidate).unwrap_or_else(|| candidate.clone())
        };
        return Some(ExifToolInfo { path, version });
    }
    None
}

fn find_in_path(command: &Path) -> Option<PathBuf> {
    let name = command.to_str()?;
    let path = std::env::var_os("PATH")?;
    for directory in std::env::split_paths(&path) {
        let candidate = directory.join(name);
        if candidate.is_file() {
            return Some(candidate.canonicalize().unwrap_or(candidate));
        }
    }
    None
}

#[derive(Debug, Clone)]
pub struct ExifToolDates {
    pub dates: HashMap<PathBuf, String>,
    pub warnings: Vec<String>,
}

pub fn read_exiftool_dates(paths: &[PathBuf], exiftool: &Path) -> ExifToolDates {
    let mut dates = HashMap::new();
    let mut warnings = Vec::new();
    for chunk in paths.chunks(60) {
        if chunk.is_empty() {
            continue;
        }
        let output = Command::new(exiftool)
            .arg("-json")
            .arg("-n")
            .arg("-EXIF:DateTimeOriginal")
            .arg("-EXIF:CreateDate")
            .arg("-EXIF:SubSecTimeOriginal")
            .arg("-File:FileModifyDate")
            .arg("-charset")
            .arg("filename=UTF8")
            .args(chunk)
            .output();
        let output = match output {
            Ok(output) => output,
            Err(error) => {
                warnings.push(format!("ExifTool could not be started: {error}"));
                break;
            }
        };
        if !output.status.success() {
            warnings.push(format!(
                "ExifTool could not read capture metadata: {}",
                String::from_utf8_lossy(&output.stderr).trim()
            ));
            continue;
        }
        let parsed: JsonValue = match serde_json::from_slice(&output.stdout) {
            Ok(value) => value,
            Err(error) => {
                warnings.push(format!("ExifTool returned invalid JSON: {error}"));
                continue;
            }
        };
        let Some(records) = parsed.as_array() else {
            continue;
        };
        for record in records {
            let Some(source) = record.get("SourceFile").and_then(JsonValue::as_str) else {
                continue;
            };
            let date = record
                .get("DateTimeOriginal")
                .and_then(JsonValue::as_str)
                .or_else(|| record.get("CreateDate").and_then(JsonValue::as_str));
            let Some(date) = date.filter(|value| parse_capture_string(value).is_some()) else {
                continue;
            };
            let path = PathBuf::from(source);
            dates.insert(path.clone(), date.to_string());
            if let Ok(canonical) = path.canonicalize() {
                dates.insert(canonical, date.to_string());
            }
        }
    }
    ExifToolDates { dates, warnings }
}

pub fn parse_capture_string(value: &str) -> Option<NaiveDateTime> {
    let value = value.trim();
    if value.is_empty() {
        return None;
    }
    // EXIF uses `YYYY:MM:DD`, XMP uses `YYYY-MM-DD`. Both can carry a time,
    // fractional seconds and a timezone suffix. The wall-clock part is what
    // matters for naming; the offset is deliberately ignored.
    let normalized = if let Some((date, time)) = value.split_once('T') {
        format!("{} {}", date.replace(':', "-"), time)
    } else if value.contains(':') && value.len() >= 10 && value.as_bytes().get(4) == Some(&b':') {
        value.replacen(':', "-", 2)
    } else {
        value.to_string()
    };
    let mut normalized = normalized.as_str();
    if let Some(position) = normalized.find(|character| character == '+' || character == 'Z') {
        if position > 10 {
            normalized = &normalized[..position];
        }
    }
    let normalized = normalized.trim();
    let candidates = [
        "%Y-%m-%d %H:%M:%S%.f",
        "%Y-%m-%d %H:%M:%S",
        "%Y-%m-%d %H:%M",
        "%Y-%m-%d",
    ];
    for format in candidates {
        if let Ok(value) = NaiveDateTime::parse_from_str(normalized, format) {
            return Some(value.with_nanosecond(0).unwrap_or(value));
        }
        if let Ok(value) = NaiveDate::parse_from_str(normalized, format) {
            return value.and_hms_opt(0, 0, 0);
        }
    }
    None
}

pub fn capture_from_naive(value: NaiveDateTime, source: CaptureSource) -> CaptureInfo {
    let display = value.format("%Y-%m-%d %H:%M:%S").to_string();
    let timestamp_ms = Local
        .from_local_datetime(&value)
        .single()
        .map(|date| date.timestamp_millis())
        .unwrap_or_else(|| {
            // Ambiguous DST wall clocks still need a deterministic timestamp.
            Local
                .from_local_datetime(&value)
                .earliest()
                .map(|date| date.timestamp_millis())
                .unwrap_or(0)
        });
    CaptureInfo {
        display,
        timestamp_ms,
        source,
        warning: None,
    }
}

fn modified_fallback(modified_ms: u64) -> CaptureInfo {
    let value = Local
        .timestamp_millis_opt(modified_ms as i64)
        .single()
        .unwrap_or_else(Local::now)
        .naive_local();
    let mut capture = capture_from_naive(value, CaptureSource::FileModified);
    capture.warning = Some(
        "No capture date was found in the RAW, JPEG or XMP metadata. The file modification time is used instead."
            .into(),
    );
    capture
}

fn read_raw_builtin(path: &Path) -> Option<NaiveDateTime> {
    let source = rawler::rawsource::RawSource::new(path).ok()?;
    let decoder = rawler::get_decoder(&source).ok()?;
    let metadata = decoder
        .raw_metadata(&source, &rawler::decoders::RawDecodeParams::default())
        .ok()?;
    metadata
        .exif
        .date_time_original
        .as_deref()
        .and_then(parse_capture_string)
        .or_else(|| {
            metadata
                .exif
                .create_date
                .as_deref()
                .and_then(parse_capture_string)
        })
}

fn read_jpeg_builtin(path: &Path) -> Option<NaiveDateTime> {
    let file = std::fs::File::open(path).ok()?;
    let reader = exif::Reader::new();
    let exif = reader.read_from_container(&mut BufReader::new(file)).ok()?;
    for tag in [
        exif::Tag::DateTimeOriginal,
        exif::Tag::DateTimeDigitized,
        exif::Tag::DateTime,
    ] {
        if let Some(field) = exif.get_field(tag, exif::In::PRIMARY) {
            let value = field.value.display_as(field.tag).to_string();
            if let Some(date) = parse_capture_string(&value) {
                return Some(date);
            }
        }
    }
    None
}

fn xml_tag_text<'a>(content: &'a str, tag: &str) -> Option<&'a str> {
    let start = content.find(&format!("<{tag}"))?;
    let after_name = &content[start + tag.len() + 1..];
    let open_end = after_name.find('>')?;
    let after_open = &after_name[open_end + 1..];
    let close = after_open.find('<')?;
    let value = after_open[..close].trim();
    if value.is_empty() { None } else { Some(value) }
}

pub fn read_xmp_capture(path: &Path) -> Option<NaiveDateTime> {
    let content = std::fs::read_to_string(path).ok()?;
    for tag in [
        "photoshop:DateCreated",
        "xmp:CreateDate",
        "exif:DateTimeOriginal",
        "dc:date",
        "xap:CreateDate",
    ] {
        if let Some(value) = xml_tag_text(&content, tag) {
            if let Some(date) = parse_capture_string(value) {
                return Some(date);
            }
        }
    }
    None
}

/// Determine the capture date for an entity. RAW metadata wins over JPEG, then
/// XMP, then the file modification time (with a visible warning).
pub fn capture_for_entity(
    files: &[IngestSourceFile],
    primary_modified_ms: u64,
    exif_dates: &HashMap<PathBuf, String>,
) -> CaptureInfo {
    let mut ordered: Vec<&IngestSourceFile> = files.iter().collect();
    ordered.sort_by_key(|file| match file.role {
        IngestFileRole::Raw => 0,
        IngestFileRole::Jpeg => 1,
        IngestFileRole::Xmp => 2,
    });

    for file in &ordered {
        match file.role {
            IngestFileRole::Raw => {
                if let Some(date) = exif_dates
                    .get(&file.path)
                    .and_then(|value| parse_capture_string(value))
                    .or_else(|| read_raw_builtin(&file.path))
                {
                    return capture_from_naive(date, CaptureSource::RawExif);
                }
            }
            IngestFileRole::Jpeg => {
                if let Some(date) = exif_dates
                    .get(&file.path)
                    .and_then(|value| parse_capture_string(value))
                    .or_else(|| read_jpeg_builtin(&file.path))
                {
                    return capture_from_naive(date, CaptureSource::JpegExif);
                }
            }
            IngestFileRole::Xmp => {}
        }
    }
    for file in &ordered {
        if file.role == IngestFileRole::Xmp {
            if let Some(date) = read_xmp_capture(&file.path) {
                return capture_from_naive(date, CaptureSource::Xmp);
            }
        }
    }
    modified_fallback(primary_modified_ms)
}

/// Resolve every source path once so planning can stay deterministic.
pub fn read_exif_dates_for_sources(
    files: &[IngestSourceFile],
    exiftool: Option<&Path>,
) -> ExifToolDates {
    let Some(exiftool) = exiftool else {
        return ExifToolDates {
            dates: HashMap::new(),
            warnings: Vec::new(),
        };
    };
    let paths: Vec<PathBuf> = files
        .iter()
        .filter(|file| file.role.is_photo())
        .map(|file| file.path.clone())
        .collect();
    read_exiftool_dates(&paths, exiftool)
}

pub fn validate_destination(destination: &Path, sources: &[PathBuf]) -> Result<PathBuf> {
    let canonical = canonical_existing(destination)?;
    let metadata =
        std::fs::metadata(&canonical).map_err(|error| LumioError::io(&canonical, error))?;
    if !metadata.is_dir() {
        return Err(LumioError::NotADirectory(canonical));
    }
    if canonical.parent().is_none() {
        return Err(LumioError::InvalidPath(
            "The filesystem root cannot be used as an ingest destination.".into(),
        ));
    }
    if let Some(home) = std::env::var_os("HOME").map(PathBuf::from) {
        let home = home.canonicalize().unwrap_or(home);
        if canonical == home {
            return Err(LumioError::InvalidPath(
                "Choose a folder inside your home folder rather than the home folder itself."
                    .into(),
            ));
        }
    }
    for source in sources {
        let source = canonical_existing(source)?;
        let source_dir = if source.is_dir() {
            source.clone()
        } else {
            source
                .parent()
                .map(Path::to_path_buf)
                .unwrap_or_else(|| source.clone())
        };
        if canonical == source_dir || canonical.starts_with(&source_dir) {
            return Err(LumioError::InvalidPath(format!(
                "The destination cannot be the source folder or inside it ({}).",
                source_dir.display()
            )));
        }
        if source.is_file() && source.starts_with(&canonical) {
            return Err(LumioError::InvalidPath(
                "A selected source file is inside the destination folder.".into(),
            ));
        }
    }
    // Probe writability without leaving anything behind.
    let probe = canonical.join(format!(".lumio-ingest-probe-{}", std::process::id()));
    match std::fs::write(&probe, b"probe") {
        Ok(()) => {
            let _ = std::fs::remove_file(&probe);
        }
        Err(error) => return Err(LumioError::io(&canonical, error)),
    }
    Ok(canonical)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_exif_and_xmp_date_shapes() {
        assert_eq!(
            parse_capture_string("2024:05:03 14:22:01").unwrap(),
            NaiveDate::from_ymd_opt(2024, 5, 3)
                .unwrap()
                .and_hms_opt(14, 22, 1)
                .unwrap()
        );
        assert_eq!(
            parse_capture_string("2024-05-03T14:22:01.123+08:00").unwrap(),
            NaiveDate::from_ymd_opt(2024, 5, 3)
                .unwrap()
                .and_hms_opt(14, 22, 1)
                .unwrap()
        );
        assert_eq!(
            parse_capture_string("2024-05-03").unwrap(),
            NaiveDate::from_ymd_opt(2024, 5, 3)
                .unwrap()
                .and_hms_opt(0, 0, 0)
                .unwrap()
        );
        assert!(parse_capture_string("not a date").is_none());
    }

    #[test]
    fn extracts_capture_date_from_xmp() {
        let temp = tempfile::tempdir().unwrap();
        let xmp = temp.path().join("photo.xmp");
        std::fs::write(
            &xmp,
            r#"<?xpacket begin="﻿"?>
            <x:xmpmeta xmlns:x="adobe:ns:meta/">
              <rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#">
                <rdf:Description rdf:about=""
                  xmlns:photoshop="http://ns.adobe.com/photoshop/1.0/">
                  <photoshop:DateCreated>2023-11-09T08:15:00</photoshop:DateCreated>
                </rdf:Description>
              </rdf:RDF>
            </x:xmpmeta>"#,
        )
        .unwrap();
        assert_eq!(
            read_xmp_capture(&xmp).unwrap(),
            NaiveDate::from_ymd_opt(2023, 11, 9)
                .unwrap()
                .and_hms_opt(8, 15, 0)
                .unwrap()
        );
    }

    #[test]
    fn falls_back_to_file_modified_time_with_warning() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("photo.jpg");
        std::fs::write(&path, b"jpeg").unwrap();
        let files = vec![IngestSourceFile {
            path: path.clone(),
            role: IngestFileRole::Jpeg,
            size: 4,
            modified_ms: 1_700_000_000_000,
        }];
        let capture = capture_for_entity(&files, 1_700_000_000_000, &HashMap::new());
        assert_eq!(capture.source, CaptureSource::FileModified);
        assert!(capture.warning.is_some());
    }

    #[test]
    fn jpeg_exif_is_read_by_the_builtin_reader() {
        let Some(exiftool) = discover_exiftool(None) else {
            return;
        };
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("dated.jpg");
        let image = image::RgbImage::new(4, 4);
        image.save(&path).unwrap();
        let status = Command::new(exiftool.path)
            .arg("-overwrite_original")
            .arg("-EXIF:DateTimeOriginal=2022:07:14 09:10:11")
            .arg(&path)
            .status()
            .unwrap();
        assert!(status.success());
        assert_eq!(
            read_jpeg_builtin(&path).unwrap(),
            NaiveDate::from_ymd_opt(2022, 7, 14)
                .unwrap()
                .and_hms_opt(9, 10, 11)
                .unwrap()
        );
    }

    #[test]
    fn detects_exiftool_when_installed() {
        let tools = detect_tools(None);
        if let Some(info) = tools.exiftool {
            assert!(info.path.is_absolute());
            assert!(!info.version.is_empty());
        }
    }

    #[test]
    fn rejects_destination_inside_source() {
        let temp = tempfile::tempdir().unwrap();
        let source = temp.path().join("source");
        let destination = source.join("ingested");
        std::fs::create_dir_all(&source).unwrap();
        std::fs::create_dir_all(&destination).unwrap();
        let result = validate_destination(&destination, &[source]);
        assert!(result.is_err());
    }
}
