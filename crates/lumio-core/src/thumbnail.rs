use std::fs;
use std::io::Cursor;
use std::path::{Path, PathBuf};

use image::codecs::jpeg::JpegEncoder;
use image::{DynamicImage, RgbImage};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error::{LumioError, Result};
use crate::histogram::{Histogram, compute_histogram};
use crate::path_utils::modified_ms;
use crate::preview::load_preview;
use crate::types::{CACHE_VERSION, PhotoAsset, PreviewSource, ThumbnailResult};

const EDGE_BUCKETS: &[u32] = &[256, 512, 1024, 2048, 2560];

pub struct ThumbnailService {
    cache_dir: PathBuf,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CacheMeta {
    width: u32,
    height: u32,
    source: PreviewSource,
    edge: u32,
}

impl ThumbnailService {
    pub fn new(cache_dir: impl Into<PathBuf>) -> Result<Self> {
        let cache_dir = cache_dir.into();
        fs::create_dir_all(&cache_dir).map_err(|error| LumioError::io(&cache_dir, error))?;
        Ok(Self { cache_dir })
    }

    pub fn cache_dir(&self) -> &Path {
        &self.cache_dir
    }

    pub fn get(
        &self,
        asset: &PhotoAsset,
        requested_edge: u32,
        prefer_thumbnail: bool,
    ) -> Result<ThumbnailResult> {
        let edge = bucket_edge(requested_edge);
        let (jpg_path, meta_path) = self.cache_paths(asset, edge);

        if let Some(meta) = read_meta(&meta_path) {
            if jpg_path.is_file() && meta.edge == edge {
                return Ok(ThumbnailResult {
                    cache_path: Some(jpg_path),
                    width: Some(meta.width),
                    height: Some(meta.height),
                    source: meta.source,
                    edge,
                    error: None,
                });
            }
        }

        let preview = match load_preview(asset, prefer_thumbnail) {
            Ok(preview) => preview,
            Err(error) => {
                return Ok(ThumbnailResult {
                    cache_path: None,
                    width: None,
                    height: None,
                    source: PreviewSource::Placeholder,
                    edge,
                    error: Some(error.to_string()),
                });
            }
        };

        let source_width = preview.image.width();
        let source_height = preview.image.height();
        let resized = resize_for_edge(&preview.image, edge);
        let rgb = to_rgb8(resized);
        let quality = if edge >= 1024 { 90 } else { 84 };
        let mut bytes = Cursor::new(Vec::new());
        JpegEncoder::new_with_quality(&mut bytes, quality)
            .encode_image(&rgb)
            .map_err(|error| LumioError::Cache(error.to_string()))?;

        write_atomic(&jpg_path, bytes.get_ref())?;
        let meta = CacheMeta {
            width: source_width,
            height: source_height,
            source: preview.source,
            edge,
        };
        let meta_bytes =
            serde_json::to_vec(&meta).map_err(|error| LumioError::Cache(error.to_string()))?;
        write_atomic(&meta_path, &meta_bytes)?;

        Ok(ThumbnailResult {
            cache_path: Some(jpg_path),
            width: Some(source_width),
            height: Some(source_height),
            source: preview.source,
            edge,
            error: None,
        })
    }

    /// Builds a histogram from the cached preview. The preview is generated
    /// through the regular thumbnail pipeline when it is not cached yet, so
    /// the info panel only pays for one decode.
    pub fn histogram(&self, asset: &PhotoAsset, requested_edge: u32) -> Result<Histogram> {
        let preview = self.get(asset, requested_edge, false)?;
        let path = preview.cache_path.ok_or_else(|| {
            LumioError::Cache(
                preview
                    .error
                    .unwrap_or_else(|| "No preview is available for this file.".into()),
            )
        })?;
        let image = image::open(&path)
            .map_err(|error| LumioError::Cache(format!("{}: {error}", path.display())))?;
        Ok(compute_histogram(&image))
    }

    pub fn clear(&self) -> Result<()> {
        if self.cache_dir.exists() {
            fs::remove_dir_all(&self.cache_dir)
                .map_err(|error| LumioError::io(&self.cache_dir, error))?;
        }
        fs::create_dir_all(&self.cache_dir)
            .map_err(|error| LumioError::io(&self.cache_dir, error))?;
        Ok(())
    }

    fn cache_paths(&self, asset: &PhotoAsset, edge: u32) -> (PathBuf, PathBuf) {
        let mut hasher = blake3::Hasher::new();
        hasher.update(asset.primary_path.as_os_str().as_encoded_bytes());
        hasher.update(&asset.file_size.to_le_bytes());
        hasher.update(&asset.modified_ms.to_le_bytes());
        // A replaced companion JPEG must invalidate the cached thumbnail even
        // when the RAW itself did not change.
        if let Some(companion) = &asset.companion_jpeg {
            hasher.update(companion.as_os_str().as_encoded_bytes());
            if let Ok(metadata) = fs::metadata(companion) {
                hasher.update(&metadata.len().to_le_bytes());
                hasher.update(&modified_ms(&metadata).to_le_bytes());
            }
        }
        hasher.update(&edge.to_le_bytes());
        hasher.update(&CACHE_VERSION.to_le_bytes());
        let hash = hasher.finalize().to_hex().to_string();
        let shard = &hash[..2];
        let base = self.cache_dir.join(shard).join(&hash);
        let jpg = base.with_extension("jpg");
        let meta = base.with_extension("json");
        (jpg, meta)
    }
}

pub fn bucket_edge(requested: u32) -> u32 {
    EDGE_BUCKETS
        .iter()
        .copied()
        .find(|edge| *edge >= requested)
        .unwrap_or(*EDGE_BUCKETS.last().expect("buckets must not be empty"))
}

fn resize_for_edge(image: &DynamicImage, edge: u32) -> DynamicImage {
    let (width, height) = (image.width(), image.height());
    if width <= edge && height <= edge {
        return image.clone();
    }
    // Fit the long edge into the bucket without forcing a square: previews must
    // keep their original aspect ratio or the viewer shows stretched photos.
    let scale = edge as f64 / width.max(height) as f64;
    let target_width = ((width as f64 * scale).round() as u32).max(1);
    let target_height = ((height as f64 * scale).round() as u32).max(1);
    // Box-filtered downscale is dramatically faster than a convolution kernel
    // at these ratios (roughly 8x on a 45 MP embedded preview) and avoids the
    // ringing that harms busy photographic detail.
    DynamicImage::ImageRgba8(image::imageops::thumbnail(
        image,
        target_width,
        target_height,
    ))
}

fn to_rgb8(image: DynamicImage) -> RgbImage {
    if image.color().has_alpha() {
        let rgba = image.to_rgba8();
        let mut rgb = RgbImage::new(rgba.width(), rgba.height());
        for (x, y, pixel) in rgba.enumerate_pixels() {
            let alpha = pixel[3] as u16;
            let inverse = 255 - alpha;
            rgb.put_pixel(
                x,
                y,
                image::Rgb([
                    ((pixel[0] as u16 * alpha + 128 * inverse) / 255) as u8,
                    ((pixel[1] as u16 * alpha + 128 * inverse) / 255) as u8,
                    ((pixel[2] as u16 * alpha + 128 * inverse) / 255) as u8,
                ]),
            );
        }
        rgb
    } else {
        image.to_rgb8()
    }
}

fn read_meta(path: &Path) -> Option<CacheMeta> {
    let bytes = fs::read(path).ok()?;
    serde_json::from_slice(&bytes).ok()
}

fn write_atomic(path: &Path, bytes: &[u8]) -> Result<()> {
    let parent = path
        .parent()
        .ok_or_else(|| LumioError::Cache(format!("Invalid cache path: {}", path.display())))?;
    fs::create_dir_all(parent).map_err(|error| LumioError::io(parent, error))?;
    let tmp = parent.join(format!(".lumio-{}.tmp", Uuid::new_v4()));
    fs::write(&tmp, bytes).map_err(|error| LumioError::io(&tmp, error))?;
    match fs::rename(&tmp, path) {
        Ok(()) => Ok(()),
        Err(_error) if path.exists() => {
            let _ = fs::remove_file(&tmp);
            Ok(())
        }
        Err(error) => {
            let _ = fs::remove_file(&tmp);
            Err(LumioError::io(path, error))
        }
    }
}

/// Ensure we only ever clear the cache directory we own.
pub fn is_safe_cache_dir(cache_dir: &Path) -> bool {
    cache_dir
        .file_name()
        .map(|name| name.to_string_lossy() == "thumbnails")
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn buckets_round_up_and_cap() {
        assert_eq!(bucket_edge(1), 256);
        assert_eq!(bucket_edge(256), 256);
        assert_eq!(bucket_edge(300), 512);
        assert_eq!(bucket_edge(1800), 2048);
        assert_eq!(bucket_edge(9000), 2560);
    }

    #[test]
    fn resize_preserves_aspect_ratio() {
        let landscape = DynamicImage::ImageRgb8(RgbImage::new(400, 200));
        let resized = resize_for_edge(&landscape, 256);
        assert_eq!((resized.width(), resized.height()), (256, 128));

        let portrait = DynamicImage::ImageRgb8(RgbImage::new(200, 400));
        let resized = resize_for_edge(&portrait, 256);
        assert_eq!((resized.width(), resized.height()), (128, 256));

        let small = DynamicImage::ImageRgb8(RgbImage::new(100, 50));
        let resized = resize_for_edge(&small, 256);
        assert_eq!((resized.width(), resized.height()), (100, 50));
    }

    #[test]
    fn histogram_uses_cached_preview() {
        let temp = tempfile::tempdir().unwrap();
        let service = ThumbnailService::new(temp.path()).unwrap();
        let path = temp.path().join("photo.png");
        let mut image = RgbImage::new(2, 1);
        image.put_pixel(0, 0, image::Rgb([0, 0, 0]));
        image.put_pixel(1, 0, image::Rgb([255, 255, 255]));
        image.save(&path).unwrap();

        let metadata = fs::metadata(&path).unwrap();
        let asset = PhotoAsset {
            id: "test".into(),
            primary_path: path,
            file_name: "photo.png".into(),
            stem: "photo".into(),
            extension: "png".into(),
            kind: crate::types::AssetKind::Png,
            companion_jpeg: None,
            xmp_sidecar: None,
            extra_sidecars: Vec::new(),
            extra_companions: Vec::new(),
            preview_hint: PreviewSource::Original,
            file_size: metadata.len(),
            modified_ms: modified_ms(&metadata),
            width: Some(2),
            height: Some(1),
            relative_dir: String::new(),
            metadata: Default::default(),
        };

        let histogram = service.histogram(&asset, 256).unwrap();
        assert_eq!(histogram.pixels, 2);
        assert_eq!(histogram.red[0], 1);
        assert_eq!(histogram.red[255], 1);
        // The second call must reuse the cache and stay consistent.
        let again = service.histogram(&asset, 256).unwrap();
        assert_eq!(again, histogram);
    }

    #[test]
    fn cache_dir_guard() {
        assert!(is_safe_cache_dir(Path::new("/tmp/lumio/thumbnails")));
        assert!(!is_safe_cache_dir(Path::new("/tmp/lumio")));
    }
}
