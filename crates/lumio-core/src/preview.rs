use std::path::Path;

use image::DynamicImage;
use rawler::decoders::RawDecodeParams;
use rawler::get_decoder;
use rawler::rawsource::RawSource;

use crate::error::{LumioError, Result};
use crate::scan::read_orientation;
use crate::types::{AssetKind, PhotoAsset, PreviewSource};

pub struct PreviewImage {
    pub image: DynamicImage,
    pub source: PreviewSource,
}

pub fn load_preview(asset: &PhotoAsset, prefer_thumbnail: bool) -> Result<PreviewImage> {
    if let Some(companion) = &asset.companion_jpeg {
        match load_standard_image(companion) {
            Ok(image) => {
                return Ok(PreviewImage {
                    image,
                    source: PreviewSource::CompanionJpeg,
                });
            }
            Err(error) if asset.kind == AssetKind::Raw => {
                // A corrupt companion should not hide a perfectly good embedded
                // RAW preview. Fall through and make the source badge honest.
                let _ = error;
            }
            Err(error) => return Err(error),
        }
    }

    match asset.kind {
        AssetKind::Raw => {
            let image = load_raw_preview(&asset.primary_path, prefer_thumbnail)?;
            Ok(PreviewImage {
                image,
                source: PreviewSource::EmbeddedRaw,
            })
        }
        _ => Ok(PreviewImage {
            image: load_standard_image(&asset.primary_path)?,
            source: PreviewSource::Original,
        }),
    }
}

fn load_standard_image(path: &Path) -> Result<DynamicImage> {
    let mut image = image::open(path)
        .map_err(|error| LumioError::Preview(format!("{}: {error}", path.display())))?;
    if let Some(orientation) = read_orientation(path) {
        image = apply_orientation(image, orientation);
    }
    Ok(image)
}

fn load_raw_preview(path: &Path, prefer_thumbnail: bool) -> Result<DynamicImage> {
    let source = RawSource::new(path)
        .map_err(|error| LumioError::Preview(format!("{}: {error}", path.display())))?;
    let decoder = get_decoder(&source)
        .map_err(|error| LumioError::Preview(format!("{}: {error}", path.display())))?;
    let params = RawDecodeParams::default();

    let attempt = |thumbnail_first: bool| -> std::result::Result<Option<DynamicImage>, String> {
        let ordered: [bool; 2] = if thumbnail_first {
            [true, false]
        } else {
            [false, true]
        };
        let mut last_error = None;
        for thumbnail in ordered {
            let result = if thumbnail {
                decoder.thumbnail_image(&source, &params)
            } else {
                decoder.preview_image(&source, &params)
            };
            match result {
                Ok(Some(image)) => return Ok(Some(image)),
                Ok(None) => {}
                Err(error) => last_error = Some(error.to_string()),
            }
        }
        if let Some(error) = last_error {
            Err(error)
        } else {
            Ok(None)
        }
    };

    let image = match attempt(prefer_thumbnail) {
        Ok(Some(image)) => image,
        Ok(None) => {
            return Err(LumioError::Preview(format!(
                "No embedded preview found in {}",
                path.display()
            )));
        }
        Err(error) => return Err(LumioError::Preview(format!("{}: {error}", path.display()))),
    };

    let orientation = decoder
        .raw_metadata(&source, &params)
        .ok()
        .and_then(|metadata| metadata.exif.orientation)
        .or_else(|| read_orientation(path));
    Ok(match orientation {
        Some(value) => apply_orientation(image, value),
        None => image,
    })
}

pub fn apply_orientation(image: DynamicImage, orientation: u16) -> DynamicImage {
    match orientation {
        2 => image.fliph(),
        3 => image.rotate180(),
        4 => image.flipv(),
        5 => image.rotate90().fliph(),
        6 => image.rotate90(),
        7 => image.rotate270().fliph(),
        8 => image.rotate270(),
        _ => image,
    }
}
