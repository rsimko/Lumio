use std::path::Path;
use std::process::Command;

use serde_json::Value as JsonValue;

use crate::error::{LumioError, Result};
use crate::ingest::types::IngestIptcValues;

const XMP_PACKET: &str = "\u{feff}<?xpacket begin=\"\u{feff}\" id=\"W5M0MpCehiHzreSzNTczkc9d\"?>\n\
<x:xmpmeta xmlns:x=\"adobe:ns:meta/\">\n\
 <rdf:RDF xmlns:rdf=\"http://www.w3.org/1999/02/22-rdf-syntax-ns#\">\n\
  <rdf:Description rdf:about=\"\"\n\
    xmlns:dc=\"http://purl.org/dc/elements/1.1/\"\n\
    xmlns:xmp=\"http://ns.adobe.com/xap/1.0/\"\n\
    xmlns:photoshop=\"http://ns.adobe.com/photoshop/1.0/\"\n\
    xmlns:exif=\"http://ns.adobe.com/exif/1.0/\"\n\
    xmlns:xmpMM=\"http://ns.adobe.com/xap/1.0/mm/\"\n\
    xmlns:Iptc4xmpCore=\"http://iptc.org/std/Iptc4xmpCore/1.0/xmlns/\">\n\
  </rdf:Description>\n\
 </rdf:RDF>\n\
</x:xmpmeta>\n\
<?xpacket end=\"w\"?>\n";

/// Create a minimal but valid XMP packet when an entity has no source sidecar
/// but the user supplied IPTC values. ExifTool then fills only the requested
/// properties, leaving the packet structure and namespaces intact.
pub fn create_empty_xmp(path: &Path) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|error| LumioError::io(parent, error))?;
    }
    std::fs::write(path, XMP_PACKET).map_err(|error| LumioError::io(path, error))
}

fn push_value(args: &mut Vec<String>, tag: &str, value: Option<&String>) {
    if let Some(value) = value.filter(|value| !value.trim().is_empty()) {
        args.push(format!("-{tag}={value}"));
    }
}

pub fn write_iptc(path: &Path, values: &IngestIptcValues, exiftool: &Path) -> Result<()> {
    if values.is_empty() {
        return Ok(());
    }
    if !path.exists() {
        return Err(LumioError::Ingest(format!(
            "The destination metadata file is missing: {}",
            path.display()
        )));
    }

    let mut args: Vec<String> = vec![
        "-overwrite_original".into(),
        "-P".into(),
        "-charset".into(),
        "iptc=UTF8".into(),
    ];
    let keywords: Vec<&String> = values
        .keywords
        .iter()
        .filter(|keyword| !keyword.trim().is_empty())
        .collect();

    push_value(&mut args, "XMP-dc:Title", values.title.as_ref());
    push_value(&mut args, "XMP-dc:Description", values.description.as_ref());
    push_value(&mut args, "XMP-dc:Creator", values.creator.as_ref());
    push_value(&mut args, "XMP-dc:Rights", values.copyright.as_ref());
    if !keywords.is_empty() {
        // Clear the list first so editing the keyword field replaces the old
        // list instead of endlessly appending duplicates.
        args.push("-XMP-dc:Subject=".into());
        for keyword in &keywords {
            args.push(format!("-XMP-dc:Subject={keyword}"));
        }
    }

    // JPEG files also carry legacy IPTC IIM. Writing both keeps other tools
    // happy while the XMP sidecar remains the authoritative metadata store for
    // RAW files.
    let is_xmp = path
        .extension()
        .map(|value| value.to_string_lossy().eq_ignore_ascii_case("xmp"))
        .unwrap_or(false);
    if !is_xmp {
        push_value(&mut args, "IPTC:ObjectName", values.title.as_ref());
        push_value(
            &mut args,
            "IPTC:Caption-Abstract",
            values.description.as_ref(),
        );
        push_value(&mut args, "IPTC:By-line", values.creator.as_ref());
        push_value(&mut args, "IPTC:CopyrightNotice", values.copyright.as_ref());
        if !keywords.is_empty() {
            args.push("-IPTC:Keywords=".into());
            for keyword in &keywords {
                args.push(format!("-IPTC:Keywords={keyword}"));
            }
        }
    }

    args.push(path.to_string_lossy().into_owned());
    let output = Command::new(exiftool)
        .args(&args)
        .output()
        .map_err(|error| {
            LumioError::Ingest(format!("ExifTool could not write metadata: {error}"))
        })?;
    if !output.status.success() {
        return Err(LumioError::Ingest(format!(
            "ExifTool could not write metadata to {}: {}",
            path.display(),
            String::from_utf8_lossy(&output.stderr).trim()
        )));
    }
    verify_iptc(path, values, exiftool)
}

fn json_string(record: &JsonValue, key: &str) -> Option<String> {
    record
        .get(key)
        .and_then(JsonValue::as_str)
        .map(str::to_string)
}

fn json_list(record: &JsonValue, key: &str) -> Vec<String> {
    match record.get(key) {
        Some(JsonValue::Array(values)) => values
            .iter()
            .filter_map(JsonValue::as_str)
            .map(str::to_string)
            .collect(),
        Some(JsonValue::String(value)) => vec![value.clone()],
        _ => Vec::new(),
    }
}

/// Read the fields back after writing. This catches silent ExifTool failures
/// before an entity is committed to the destination.
pub fn verify_iptc(path: &Path, values: &IngestIptcValues, exiftool: &Path) -> Result<()> {
    let output = Command::new(exiftool)
        .arg("-json")
        .arg("-n")
        .arg("-XMP-dc:Title")
        .arg("-XMP-dc:Description")
        .arg("-XMP-dc:Creator")
        .arg("-XMP-dc:Rights")
        .arg("-XMP-dc:Subject")
        .arg(path)
        .output()
        .map_err(|error| {
            LumioError::Ingest(format!("ExifTool could not verify metadata: {error}"))
        })?;
    if !output.status.success() {
        return Err(LumioError::Ingest(format!(
            "ExifTool could not verify metadata in {}: {}",
            path.display(),
            String::from_utf8_lossy(&output.stderr).trim()
        )));
    }
    let parsed: JsonValue = serde_json::from_slice(&output.stdout).map_err(|error| {
        LumioError::Ingest(format!(
            "ExifTool returned invalid verification JSON: {error}"
        ))
    })?;
    let record = parsed
        .as_array()
        .and_then(|records| records.first())
        .ok_or_else(|| LumioError::Ingest("ExifTool returned no metadata record.".into()))?;

    if let Some(expected) = values
        .title
        .as_ref()
        .filter(|value| !value.trim().is_empty())
    {
        if json_string(record, "Title").as_deref() != Some(expected.as_str()) {
            return Err(LumioError::Ingest(
                "The XMP title could not be verified after writing.".into(),
            ));
        }
    }
    if let Some(expected) = values
        .description
        .as_ref()
        .filter(|value| !value.trim().is_empty())
    {
        if json_string(record, "Description").as_deref() != Some(expected.as_str()) {
            return Err(LumioError::Ingest(
                "The XMP description could not be verified after writing.".into(),
            ));
        }
    }
    if let Some(expected) = values
        .creator
        .as_ref()
        .filter(|value| !value.trim().is_empty())
    {
        if json_string(record, "Creator").as_deref() != Some(expected.as_str()) {
            return Err(LumioError::Ingest(
                "The XMP creator could not be verified after writing.".into(),
            ));
        }
    }
    if let Some(expected) = values
        .copyright
        .as_ref()
        .filter(|value| !value.trim().is_empty())
    {
        if json_string(record, "Rights").as_deref() != Some(expected.as_str()) {
            return Err(LumioError::Ingest(
                "The XMP copyright could not be verified after writing.".into(),
            ));
        }
    }
    let expected_keywords: Vec<String> = values
        .keywords
        .iter()
        .filter(|value| !value.trim().is_empty())
        .cloned()
        .collect();
    if !expected_keywords.is_empty() {
        let actual = json_list(record, "Subject");
        let mut expected = expected_keywords;
        expected.sort();
        let mut actual_sorted = actual.clone();
        actual_sorted.sort();
        if actual_sorted != expected {
            return Err(LumioError::Ingest(
                "The XMP keywords could not be verified after writing.".into(),
            ));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn exiftool() -> Option<std::path::PathBuf> {
        crate::ingest::metadata::discover_exiftool(None).map(|info| info.path)
    }

    #[test]
    fn writes_iptc_and_preserves_unrelated_xmp_fields() {
        let Some(exiftool) = exiftool() else {
            return;
        };
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("photo.RAF.xmp");
        std::fs::write(
            &path,
            r#"<?xpacket begin="﻿" id="W5M0MpCehiHzreSzNTczkc9d"?>
<x:xmpmeta xmlns:x="adobe:ns:meta/">
 <rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#">
  <rdf:Description rdf:about=""
    xmlns:xmp="http://ns.adobe.com/xap/1.0/"
    xmlns:photoshop="http://ns.adobe.com/photoshop/1.0/"
    xmlns:dc="http://purl.org/dc/elements/1.1/">
   <xmp:Label>Green</xmp:Label>
   <photoshop:City>Palawan</photoshop:City>
  </rdf:Description>
 </rdf:RDF>
</x:xmpmeta>
<?xpacket end="w"?>"#,
        )
        .unwrap();

        let values = IngestIptcValues {
            title: Some("Morning light".into()),
            description: Some("A quiet beach".into()),
            creator: Some("Radek".into()),
            copyright: Some("© Radek".into()),
            keywords: vec!["travel".into(), "beach".into()],
        };
        write_iptc(&path, &values, &exiftool).unwrap();

        let content = std::fs::read_to_string(&path).unwrap();
        assert!(content.contains("Green"));
        assert!(content.contains("Palawan"));
        assert!(content.contains("Morning light"));
        assert!(content.contains("travel"));
        assert!(content.contains("beach"));
    }

    #[test]
    fn blank_values_are_treated_as_empty() {
        let values = IngestIptcValues {
            title: Some("  ".into()),
            description: None,
            creator: None,
            copyright: None,
            keywords: vec![" ".into()],
        };
        assert!(values.is_empty());
    }

    #[test]
    fn writes_metadata_into_a_standalone_jpeg() {
        let Some(exiftool) = exiftool() else {
            return;
        };
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("photo.jpg");
        let image = image::RgbImage::new(8, 8);
        image.save(&path).unwrap();

        let values = IngestIptcValues {
            title: Some("JPEG title".into()),
            description: None,
            creator: None,
            copyright: None,
            keywords: vec!["one".into(), "two".into()],
        };
        write_iptc(&path, &values, &exiftool).unwrap();

        let output = Command::new(&exiftool)
            .arg("-json")
            .arg("-XMP-dc:Title")
            .arg("-XMP-dc:Subject")
            .arg(&path)
            .output()
            .unwrap();
        let json: JsonValue = serde_json::from_slice(&output.stdout).unwrap();
        let record = &json.as_array().unwrap()[0];
        assert_eq!(record["Title"], "JPEG title");
        assert_eq!(record["Subject"].as_array().unwrap().len(), 2);
    }

    #[test]
    fn creates_a_sidecar_from_scratch() {
        let Some(exiftool) = exiftool() else {
            return;
        };
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("new.xmp");
        create_empty_xmp(&path).unwrap();
        let values = IngestIptcValues {
            title: Some("Created".into()),
            description: None,
            creator: None,
            copyright: None,
            keywords: Vec::new(),
        };
        write_iptc(&path, &values, &exiftool).unwrap();
        let content = std::fs::read_to_string(&path).unwrap();
        assert!(content.contains("Created"));
    }
}
