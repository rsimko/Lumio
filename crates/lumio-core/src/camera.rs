use std::fs::File;
use std::io::BufReader;
use std::path::Path;

use exif::{In, Tag, Value};
use serde::{Deserialize, Serialize};

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
    }
}

/// Reads camera details from the original and, when needed, its companion
/// JPEG. RAW files use rawler first so formats such as RAF still provide
/// camera and lens data; everything falls back to the native EXIF reader.
pub fn read_camera_info(primary: &Path, companion: Option<&Path>) -> CameraInfo {
    let mut info = read_native(primary).unwrap_or_default();
    if !info.is_complete() {
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
    };
    (!info.is_empty()).then_some(info)
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
