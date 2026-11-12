use std::fs::File;
use std::io::BufReader;
use std::path::Path;

use exif::{In, Tag, Value};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;

use crate::types::{extension_of, is_raw_extension};

/// Camera and exposure details read from the photo's embedded EXIF.
///
/// Shutter speed is stored in seconds, aperture as an f-number and focal
/// length in millimetres so the frontend can localise the presentation.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CameraInfo {
    pub make: Option<String>,
    pub model: Option<String>,
    pub lens: Option<String>,
    pub shutter_speed: Option<f64>,
    pub aperture: Option<f64>,
    pub iso: Option<u32>,
    pub focal_length: Option<f64>,
    /// GPS latitude in signed decimal degrees, when the camera recorded it.
    pub latitude: Option<f64>,
    /// GPS longitude in signed decimal degrees.
    pub longitude: Option<f64>,
    /// GPS altitude in metres, relative to sea level.
    pub altitude: Option<f64>,
}

impl CameraInfo {
    pub fn is_empty(&self) -> bool {
        self.make.is_none()
            && self.model.is_none()
            && self.lens.is_none()
            && self.shutter_speed.is_none()
            && self.aperture.is_none()
            && self.iso.is_none()
            && self.focal_length.is_none()
            && self.latitude.is_none()
            && self.longitude.is_none()
            && self.altitude.is_none()
    }

    fn is_complete(&self) -> bool {
        self.make.is_some()
            && self.model.is_some()
            && self.lens.is_some()
            && self.shutter_speed.is_some()
            && self.aperture.is_some()
            && self.iso.is_some()
            && self.focal_length.is_some()
    }

    fn fill_missing(&mut self, other: Self) {
        if self.make.is_none() {
            self.make = other.make;
        }
        if self.model.is_none() {
            self.model = other.model;
        }
        if self.lens.is_none() {
            self.lens = other.lens;
        }
        if self.shutter_speed.is_none() {
            self.shutter_speed = other.shutter_speed;
        }
        if self.aperture.is_none() {
            self.aperture = other.aperture;
        }
        if self.iso.is_none() {
            self.iso = other.iso;
        }
        if self.focal_length.is_none() {
            self.focal_length = other.focal_length;
        }
        if self.latitude.is_none() {
            self.latitude = other.latitude;
        }
        if self.longitude.is_none() {
            self.longitude = other.longitude;
        }
        if self.altitude.is_none() {
            self.altitude = other.altitude;
        }
    }
}

/// One tag in the full EXIF view.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExifEntry {
    /// ExifTool group such as `ExifIFD`, `IFD0`, `GPS` or a maker-specific
    /// group like `Canon`.
    pub group: String,
    pub tag: String,
    pub value: String,
}

/// Complete metadata listing for the EXIF viewer.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FullExif {
    /// `exiftool` when the external tool produced the listing, `embedded`
    /// when it came from the built-in EXIF reader, or `none`.
    pub source: String,
    pub entries: Vec<ExifEntry>,
}

/// Reads every EXIF tag for the photo. ExifTool provides grouped maker notes
/// and formats such as RAF that the built-in reader cannot open; the native
/// `exif` crate remains the fallback when ExifTool is unavailable.
pub fn read_full_exif(
    primary: &Path,
    companion: Option<&Path>,
    configured_exiftool: Option<&Path>,
) -> FullExif {
    if let Some(tool) = crate::ingest::metadata::discover_exiftool(configured_exiftool) {
        for path in std::iter::once(primary).chain(companion) {
            if let Some(entries) = read_exiftool_entries(&tool.path, path) {
                if !entries.is_empty() {
                    return FullExif {
                        source: "exiftool".to_string(),
                        entries,
                    };
                }
            }
        }
    }

    for path in std::iter::once(primary).chain(companion) {
        if let Some(entries) = read_embedded_entries(path) {
            if !entries.is_empty() {
                return FullExif {
                    source: "embedded".to_string(),
                    entries,
                };
            }
        }
    }

    FullExif {
        source: "none".to_string(),
        entries: Vec::new(),
    }
}

/// ExifTool groups that are not part of the image's EXIF metadata. XMP, IPTC
/// and file-system details have their own sections elsewhere in the app.
fn is_exif_group(group: &str) -> bool {
    if matches!(
        group,
        "System"
            | "File"
            | "ExifTool"
            | "SourceFile"
            | "IPTC"
            | "IPTC-NAA"
            | "Photoshop"
            | "ICC_Profile"
            | "ICC-header"
            | "XML"
            | "JFIF"
            | "JFXX"
            | "APP0"
            | "APP1"
            | "APP2"
            | "APP3"
            | "APP4"
            | "APP5"
            | "APP6"
            | "APP7"
            | "APP8"
            | "APP9"
            | "APP10"
            | "APP11"
            | "APP12"
            | "APP13"
            | "APP14"
            | "APP15"
    ) {
        return false;
    }
    !group.starts_with("XMP") && !group.starts_with("Photoshop")
}

fn read_exiftool_entries(tool: &Path, path: &Path) -> Option<Vec<ExifEntry>> {
    let output = std::process::Command::new(tool)
        .args(["-json", "-G1", "-s", "-a", "-charset", "filename=UTF8"])
        .arg(path)
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }

    let parsed: JsonValue = serde_json::from_slice(&output.stdout).ok()?;
    let object = parsed.as_array()?.first()?.as_object()?;
    let mut entries = Vec::new();
    for (key, value) in object {
        let Some((group, tag)) = key.split_once(':') else {
            continue;
        };
        if !is_exif_group(group) {
            continue;
        }
        let Some(value) = render_json_value(value) else {
            continue;
        };
        if value.contains("(Binary data") {
            continue;
        }
        entries.push(ExifEntry {
            group: group.to_string(),
            tag: tag.to_string(),
            value: truncate_value(value),
        });
    }
    Some(entries)
}

fn render_json_value(value: &JsonValue) -> Option<String> {
    let rendered = match value {
        JsonValue::Null => return None,
        JsonValue::String(text) => text.clone(),
        JsonValue::Bool(flag) => flag.to_string(),
        JsonValue::Number(number) => number.to_string(),
        JsonValue::Array(items) => items
            .iter()
            .filter_map(render_json_value)
            .collect::<Vec<_>>()
            .join(", "),
        JsonValue::Object(_) => value.to_string(),
    };
    let rendered = rendered.trim();
    (!rendered.is_empty()).then(|| rendered.to_string())
}

fn truncate_value(value: String) -> String {
    const LIMIT: usize = 4000;
    if value.chars().count() <= LIMIT {
        return value;
    }
    let mut truncated: String = value.chars().take(LIMIT).collect();
    truncated.push('…');
    truncated
}

fn read_embedded_entries(path: &Path) -> Option<Vec<ExifEntry>> {
    let file = File::open(path).ok()?;
    let mut reader = BufReader::new(file);
    let exif = exif::Reader::new().read_from_container(&mut reader).ok()?;
    let mut entries = Vec::new();
    for field in exif.fields() {
        if matches!(&field.value, Value::Undefined(..) | Value::Unknown(..)) {
            continue;
        }
        let value = field.display_value().with_unit(&exif).to_string();
        if value.trim().is_empty() {
            continue;
        }
        entries.push(ExifEntry {
            group: match field.ifd_num.index() {
                0 => "EXIF".to_string(),
                1 => "Thumbnail".to_string(),
                index => format!("IFD{index}"),
            },
            tag: field.tag.to_string(),
            value: truncate_value(value),
        });
    }
    Some(entries)
}

/// Reads camera details from the original and, when needed, its companion
/// JPEG. RAW files use rawler first so formats such as RAF still provide
/// camera and lens data; everything falls back to the native EXIF reader.
pub fn read_camera_info(primary: &Path, companion: Option<&Path>) -> CameraInfo {
    let mut info = read_native(primary).unwrap_or_default();
    if !info.is_complete() || info.latitude.is_none() || info.longitude.is_none() {
        if let Some(companion) = companion.and_then(read_native) {
            info.fill_missing(companion);
        }
    }
    info
}

fn read_native(path: &Path) -> Option<CameraInfo> {
    let is_raw = extension_of(path).is_some_and(|extension| is_raw_extension(&extension));
    let mut info = if is_raw {
        read_raw(path).unwrap_or_default()
    } else {
        CameraInfo::default()
    };
    if !info.is_complete() {
        if let Some(exif) = read_exif(path) {
            info.fill_missing(exif);
        }
    }
    (!info.is_empty()).then_some(info)
}

fn read_raw(path: &Path) -> Option<CameraInfo> {
    let source = rawler::rawsource::RawSource::new(path).ok()?;
    let decoder = rawler::get_decoder(&source).ok()?;
    let metadata = decoder
        .raw_metadata(&source, &rawler::decoders::RawDecodeParams::default())
        .ok()?;
    let rawler::decoders::RawMetadata {
        exif,
        make,
        model,
        lens,
        ..
    } = metadata;
    let lens = lens
        .as_ref()
        .and_then(|lens| clean(lens.lens_name.clone()))
        .or_else(|| clean(exif.lens_model.clone().unwrap_or_default()));
    let gps = exif.gps.as_ref();
    let latitude = gps
        .and_then(|gps| raw_coordinate(gps.gps_latitude.as_ref(), gps.gps_latitude_ref.as_deref()))
        .filter(|value| (-90.0..=90.0).contains(value));
    let longitude = gps
        .and_then(|gps| {
            raw_coordinate(gps.gps_longitude.as_ref(), gps.gps_longitude_ref.as_deref())
        })
        .filter(|value| (-180.0..=180.0).contains(value));
    let altitude = gps.and_then(|gps| {
        gps.gps_altitude
            .map(|value| {
                let metres = f64::from(value.as_f32());
                if gps.gps_altitude_ref == Some(1) {
                    -metres
                } else {
                    metres
                }
            })
            .filter(|value| value.is_finite())
    });
    let info = CameraInfo {
        make: clean(make),
        model: clean(model),
        lens,
        shutter_speed: exif
            .exposure_time
            .map(|value| f64::from(value.as_f32()))
            .or_else(|| {
                exif.shutter_speed_value
                    .map(|value| 2f64.powf(-(f64::from(value.n) / f64::from(value.d))))
            })
            .and_then(positive),
        aperture: exif
            .fnumber
            .map(|value| f64::from(value.as_f32()))
            .or_else(|| {
                exif.aperture_value
                    .map(|value| 2f64.powf(f64::from(value.as_f32()) / 2.0))
            })
            .and_then(positive),
        iso: exif
            .iso_speed_ratings
            .map(u32::from)
            .or(exif.iso_speed)
            .or(exif.recommended_exposure_index)
            .filter(|iso| *iso > 0),
        focal_length: exif
            .focal_length
            .map(|value| f64::from(value.as_f32()))
            .and_then(positive),
        latitude,
        longitude,
        altitude,
    };
    (!info.is_empty()).then_some(info)
}

fn read_exif(path: &Path) -> Option<CameraInfo> {
    let file = File::open(path).ok()?;
    let mut reader = BufReader::new(file);
    let exif = exif::Reader::new().read_from_container(&mut reader).ok()?;
    let info = CameraInfo {
        make: text_field(&exif, Tag::Make),
        model: text_field(&exif, Tag::Model),
        lens: text_field(&exif, Tag::LensModel).or_else(|| text_field(&exif, Tag::LensMake)),
        shutter_speed: positive_field(&exif, Tag::ExposureTime).or_else(|| {
            signed_field(&exif, Tag::ShutterSpeedValue)
                .map(|apex| 2f64.powf(-apex))
                .and_then(positive)
        }),
        aperture: positive_field(&exif, Tag::FNumber).or_else(|| {
            signed_field(&exif, Tag::ApertureValue)
                .map(|apex| 2f64.powf(apex / 2.0))
                .and_then(positive)
        }),
        iso: uint_field(&exif, Tag::PhotographicSensitivity)
            .or_else(|| uint_field(&exif, Tag::ISOSpeed))
            .filter(|iso| *iso > 0),
        focal_length: positive_field(&exif, Tag::FocalLength),
        latitude: gps_coordinate(&exif, Tag::GPSLatitude, Tag::GPSLatitudeRef),
        longitude: gps_coordinate(&exif, Tag::GPSLongitude, Tag::GPSLongitudeRef),
        altitude: gps_altitude(&exif),
    };
    (!info.is_empty()).then_some(info)
}

fn raw_coordinate(
    values: Option<&[rawler::formats::tiff::Rational; 3]>,
    reference: Option<&str>,
) -> Option<f64> {
    let values = values?;
    let mut value = f64::from(values[0].as_f32())
        + f64::from(values[1].as_f32()) / 60.0
        + f64::from(values[2].as_f32()) / 3600.0;
    if !value.is_finite() {
        return None;
    }
    let negative = reference.is_some_and(|reference| {
        matches!(reference.trim().to_ascii_uppercase().as_str(), "S" | "W")
    });
    if negative {
        value = -value;
    }
    Some(value)
}

fn gps_coordinate(exif: &exif::Exif, tag: Tag, reference_tag: Tag) -> Option<f64> {
    let field = exif.get_field(tag, In::PRIMARY)?;
    let Value::Rational(values) = &field.value else {
        return None;
    };
    let mut value = values.first()?.to_f64()
        + values.get(1).map_or(0.0, |value| value.to_f64() / 60.0)
        + values.get(2).map_or(0.0, |value| value.to_f64() / 3600.0);
    if !value.is_finite() {
        return None;
    }
    if let Some(reference) = ascii_field(exif, reference_tag) {
        if matches!(reference.trim().to_ascii_uppercase().as_str(), "S" | "W") {
            value = -value;
        }
    }
    Some(value)
}

fn gps_altitude(exif: &exif::Exif) -> Option<f64> {
    let metres = number_field(exif, Tag::GPSAltitude)?;
    let below_sea_level = exif
        .get_field(Tag::GPSAltitudeRef, In::PRIMARY)
        .and_then(|field| field.value.get_uint(0))
        == Some(1);
    Some(if below_sea_level { -metres } else { metres })
}

fn ascii_field(exif: &exif::Exif, tag: Tag) -> Option<String> {
    let field = exif.get_field(tag, In::PRIMARY)?;
    match &field.value {
        Value::Ascii(parts) => parts
            .iter()
            .filter_map(|part| String::from_utf8(part.clone()).ok())
            .map(|part| part.trim_matches('\0').trim().to_string())
            .find(|part| !part.is_empty()),
        _ => Some(field.value.display_as(tag).to_string()),
    }
}

fn text_field(exif: &exif::Exif, tag: Tag) -> Option<String> {
    let field = exif.get_field(tag, In::PRIMARY)?;
    let value = match &field.value {
        Value::Ascii(parts) => parts
            .iter()
            .filter_map(|part| String::from_utf8(part.clone()).ok())
            .map(|part| part.trim_matches('\0').trim().to_string())
            .find(|part| !part.is_empty())?,
        _ => field.value.display_as(tag).to_string(),
    };
    clean(value)
}

fn number_field(exif: &exif::Exif, tag: Tag) -> Option<f64> {
    let field = exif.get_field(tag, In::PRIMARY)?;
    match &field.value {
        Value::Rational(values) => values.first().map(|value| value.to_f64()),
        Value::SRational(values) => values.first().map(|value| value.to_f64()),
        Value::Short(values) => values.first().map(|value| f64::from(*value)),
        Value::Long(values) => values.first().map(|value| f64::from(*value)),
        Value::Float(values) => values.first().map(|value| f64::from(*value)),
        Value::Double(values) => values.first().copied(),
        _ => field.value.get_uint(0).map(f64::from),
    }
}

fn positive_field(exif: &exif::Exif, tag: Tag) -> Option<f64> {
    number_field(exif, tag).and_then(positive)
}

fn signed_field(exif: &exif::Exif, tag: Tag) -> Option<f64> {
    number_field(exif, tag).filter(|value| value.is_finite())
}

fn uint_field(exif: &exif::Exif, tag: Tag) -> Option<u32> {
    let field = exif.get_field(tag, In::PRIMARY)?;
    field
        .value
        .get_uint(0)
        .or_else(|| number_field(exif, tag).map(|value| value.round() as u32))
}

fn positive(value: f64) -> Option<f64> {
    (value.is_finite() && value > 0.0).then_some(value)
}

fn clean(value: String) -> Option<String> {
    let value = value.trim_matches('\0').trim();
    (!value.is_empty()).then(|| value.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn exif_groups_exclude_non_exif_metadata() {
        for group in [
            "System",
            "File",
            "ExifTool",
            "IPTC",
            "Photoshop",
            "ICC_Profile",
            "XML",
            "JFIF",
            "XMP",
            "XMP-dc",
            "XMP-photoshop",
        ] {
            assert!(!is_exif_group(group), "{group} should be excluded");
        }
        for group in ["IFD0", "ExifIFD", "GPS", "InteropIFD", "Canon", "Composite"] {
            assert!(is_exif_group(group), "{group} should be included");
        }
    }

    #[test]
    fn json_values_are_rendered_flat() {
        assert_eq!(render_json_value(&json!("1/50")), Some("1/50".into()));
        assert_eq!(render_json_value(&json!(160)), Some("160".into()));
        assert_eq!(render_json_value(&json!(true)), Some("true".into()));
        assert_eq!(render_json_value(&json!(["a", "b"])), Some("a, b".into()));
        assert_eq!(render_json_value(&json!(null)), None);
        assert_eq!(render_json_value(&json!("  ")), None);
    }

    #[test]
    fn long_values_are_truncated() {
        let value = "x".repeat(5000);
        let truncated = truncate_value(value);
        assert_eq!(truncated.chars().count(), 4001);
        assert!(truncated.ends_with('…'));
    }
}
