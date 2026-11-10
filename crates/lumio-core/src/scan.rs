use std::collections::{BTreeMap, HashMap};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Instant;

use natord::compare_ignore_case;
use walkdir::{DirEntry, WalkDir};

use crate::error::{LumioError, Result};
use crate::path_utils::{display_name, is_hidden_or_package, modified_ms};
use crate::types::{
    AssetKind, FolderNode, PhotoAsset, PreviewSource, ScanResult, ScanWarning, asset_kind_for,
    extension_of, is_image_extension, is_jpeg_extension, is_raw_extension, raw_priority,
};

#[derive(Debug, Clone)]
struct FileRecord {
    path: PathBuf,
    file_name: String,
    stem: String,
    extension: String,
    file_size: u64,
    modified_ms: u64,
    width: Option<u32>,
    height: Option<u32>,
}

#[derive(Debug, Default)]
struct DirBucket {
    records: Vec<FileRecord>,
    xmps: Vec<PathBuf>,
}

#[derive(Debug, Default)]
struct StemGroup {
    raws: Vec<FileRecord>,
    jpegs: Vec<FileRecord>,
    images: Vec<FileRecord>,
}

pub fn list_subfolders(path: &Path) -> Result<Vec<FolderNode>> {
    let read = fs::read_dir(path).map_err(|error| LumioError::io(path, error))?;
    let mut nodes = Vec::new();
    for entry in read {
        let entry = match entry {
            Ok(entry) => entry,
            Err(_) => continue,
        };
        let file_type = match entry.file_type() {
            Ok(file_type) => file_type,
            Err(_) => continue,
        };
        if !file_type.is_dir() || file_type.is_symlink() {
            continue;
        }
        if is_hidden_or_package(&entry.file_name()) {
            continue;
        }
        let child_path = entry.path();
        let has_children = has_visible_child_dir(&child_path).unwrap_or(false);
        nodes.push(FolderNode {
            name: display_name(&child_path),
            path: child_path,
            has_children,
            accessible: true,
            error: None,
        });
    }
    nodes.sort_by(|left, right| compare_ignore_case(&left.name, &right.name));
    Ok(nodes)
}

fn has_visible_child_dir(path: &Path) -> Result<bool> {
    let read = fs::read_dir(path).map_err(|error| LumioError::io(path, error))?;
    for entry in read.flatten() {
        let Ok(file_type) = entry.file_type() else {
            continue;
        };
        if file_type.is_dir()
            && !file_type.is_symlink()
            && !is_hidden_or_package(&entry.file_name())
        {
            return Ok(true);
        }
    }
    Ok(false)
}

pub fn scan_photos(root: &Path, recursive: bool) -> Result<ScanResult> {
    let started = Instant::now();
    let mut buckets: BTreeMap<PathBuf, DirBucket> = BTreeMap::new();
    let mut warnings = Vec::new();

    if recursive {
        let walker = WalkDir::new(root)
            .follow_links(false)
            .into_iter()
            .filter_entry(keep_entry);
        for entry in walker {
            let entry = match entry {
                Ok(entry) => entry,
                Err(error) => {
                    if let Some(path) = error.path() {
                        warnings.push(ScanWarning {
                            path: path.to_path_buf(),
                            message: error.to_string(),
                        });
                    }
                    continue;
                }
            };
            if !entry.file_type().is_file() {
                continue;
            }
            if is_hidden_or_package(entry.file_name()) {
                continue;
            }
            add_entry(&mut buckets, entry.path(), &mut warnings);
        }
    } else {
        let read = fs::read_dir(root).map_err(|error| LumioError::io(root, error))?;
        for entry in read {
            let entry = match entry {
                Ok(entry) => entry,
                Err(error) => {
                    warnings.push(ScanWarning {
                        path: root.to_path_buf(),
                        message: error.to_string(),
                    });
                    continue;
                }
            };
            let file_type = match entry.file_type() {
                Ok(file_type) => file_type,
                Err(error) => {
                    warnings.push(ScanWarning {
                        path: entry.path(),
                        message: error.to_string(),
                    });
                    continue;
                }
            };
            if !file_type.is_file() || file_type.is_symlink() {
                continue;
            }
            if is_hidden_or_package(&entry.file_name()) {
                continue;
            }
            add_entry(&mut buckets, &entry.path(), &mut warnings);
        }
    }

    let mut assets = Vec::new();
    for (directory, bucket) in buckets {
        assets.extend(build_assets_for_dir(root, &directory, bucket));
    }

    assets.sort_by(|left, right| {
        compare_ignore_case(&left.relative_dir, &right.relative_dir)
            .then_with(|| compare_ignore_case(&left.file_name, &right.file_name))
            .then_with(|| left.extension.cmp(&right.extension))
    });

    Ok(ScanResult {
        folder: root.to_path_buf(),
        recursive,
        assets,
        warnings,
        elapsed_ms: started.elapsed().as_millis() as u64,
    })
}

fn keep_entry(entry: &DirEntry) -> bool {
    if entry.depth() == 0 {
        return true;
    }
    if entry.file_type().is_symlink() {
        return false;
    }
    if entry.file_type().is_dir() {
        return !is_hidden_or_package(entry.file_name());
    }
    true
}

fn add_entry(
    buckets: &mut BTreeMap<PathBuf, DirBucket>,
    path: &Path,
    warnings: &mut Vec<ScanWarning>,
) {
    let Some(extension) = extension_of(path) else {
        return;
    };
    let is_xmp = extension == "xmp";
    let is_photo = is_raw_extension(&extension) || is_image_extension(&extension);
    if !is_xmp && !is_photo {
        return;
    }

    let parent = path
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| path.to_path_buf());
    let bucket = buckets.entry(parent).or_default();

    if is_xmp {
        bucket.xmps.push(path.to_path_buf());
        return;
    }

    let metadata = match fs::metadata(path) {
        Ok(metadata) => metadata,
        Err(error) => {
            warnings.push(ScanWarning {
                path: path.to_path_buf(),
                message: error.to_string(),
            });
            return;
        }
    };
    let file_name = display_name(path);
    let stem = path
        .file_stem()
        .map(|value| value.to_string_lossy().into_owned())
        .unwrap_or_else(|| file_name.clone());
    let (width, height) = read_dimensions(path, &extension);

    bucket.records.push(FileRecord {
        path: path.to_path_buf(),
        file_name,
        stem,
        extension,
        file_size: metadata.len(),
        modified_ms: modified_ms(&metadata),
        width,
        height,
    });
}

fn build_assets_for_dir(root: &Path, directory: &Path, bucket: DirBucket) -> Vec<PhotoAsset> {
    let mut groups: BTreeMap<String, StemGroup> = BTreeMap::new();
    for record in bucket.records {
        // Group case-insensitively so `IMG_0001.CR2` associates with a
        // companion saved as `img_0001.jpg` on case-insensitive volumes.
        let group = groups.entry(record.stem.to_ascii_lowercase()).or_default();
        if is_raw_extension(&record.extension) {
            group.raws.push(record);
        } else if is_jpeg_extension(&record.extension) {
            group.jpegs.push(record);
        } else {
            group.images.push(record);
        }
    }

    let xmp_by_name = index_xmps(&bucket.xmps);
    let relative_dir = directory
        .strip_prefix(root)
        .ok()
        .map(|path| path.to_string_lossy().into_owned())
        .unwrap_or_default();

    let mut assets = Vec::new();
    for (_, mut group) in groups {
        group.raws.sort_by(|left, right| {
            raw_priority(&left.extension)
                .cmp(&raw_priority(&right.extension))
                .then_with(|| compare_ignore_case(&left.file_name, &right.file_name))
        });
        group.jpegs.sort_by(|left, right| {
            jpeg_priority(&left.extension)
                .cmp(&jpeg_priority(&right.extension))
                .then_with(|| compare_ignore_case(&left.file_name, &right.file_name))
        });
        group
            .images
            .sort_by(|left, right| compare_ignore_case(&left.file_name, &right.file_name));

        if !group.raws.is_empty() {
            let companion = group.jpegs.first().cloned();
            let extra_companions: Vec<PathBuf> = group
                .jpegs
                .iter()
                .skip(1)
                .map(|record| record.path.clone())
                .collect();

            for (index, raw) in group.raws.into_iter().enumerate() {
                let is_primary = index == 0;
                let (width, height) = if is_primary {
                    companion
                        .as_ref()
                        .map(|jpeg| (jpeg.width, jpeg.height))
                        .unwrap_or((raw.width, raw.height))
                } else {
                    (raw.width, raw.height)
                };
                let (sidecar, extras) =
                    find_sidecars(&raw.file_name, &raw.stem, is_primary, &xmp_by_name);
                assets.push(make_asset(
                    &relative_dir,
                    raw,
                    AssetKind::Raw,
                    if is_primary {
                        companion.as_ref().map(|jpeg| jpeg.path.clone())
                    } else {
                        None
                    },
                    if is_primary {
                        extra_companions.clone()
                    } else {
                        Vec::new()
                    },
                    sidecar,
                    extras,
                    if is_primary && companion.is_some() {
                        PreviewSource::CompanionJpeg
                    } else {
                        PreviewSource::EmbeddedRaw
                    },
                    width,
                    height,
                ));
            }

            // Non-JPEG images are independent photographs, even when they share
            // a stem with a RAW file (for example an exported TIFF).
            for image in group.images {
                let (sidecar, extras) =
                    find_sidecars(&image.file_name, &image.stem, true, &xmp_by_name);
                let kind = asset_kind_for(&image.extension);
                let width = image.width;
                let height = image.height;
                assets.push(make_asset(
                    &relative_dir,
                    image,
                    kind,
                    None,
                    Vec::new(),
                    sidecar,
                    extras,
                    PreviewSource::Original,
                    width,
                    height,
                ));
            }
        } else {
            for jpeg in group.jpegs {
                let (sidecar, extras) =
                    find_sidecars(&jpeg.file_name, &jpeg.stem, true, &xmp_by_name);
                let width = jpeg.width;
                let height = jpeg.height;
                assets.push(make_asset(
                    &relative_dir,
                    jpeg,
                    AssetKind::Jpeg,
                    None,
                    Vec::new(),
                    sidecar,
                    extras,
                    PreviewSource::Original,
                    width,
                    height,
                ));
            }
            for image in group.images {
                let (sidecar, extras) =
                    find_sidecars(&image.file_name, &image.stem, true, &xmp_by_name);
                let kind = asset_kind_for(&image.extension);
                let width = image.width;
                let height = image.height;
                assets.push(make_asset(
                    &relative_dir,
                    image,
                    kind,
                    None,
                    Vec::new(),
                    sidecar,
                    extras,
                    PreviewSource::Original,
                    width,
                    height,
                ));
            }
        }
    }
    assets
}

fn jpeg_priority(extension: &str) -> usize {
    match extension {
        "jpg" => 0,
        "jpeg" => 1,
        _ => 2,
    }
}

fn index_xmps(xmps: &[PathBuf]) -> HashMap<String, Vec<PathBuf>> {
    let mut map: HashMap<String, Vec<PathBuf>> = HashMap::new();
    for path in xmps {
        let name = display_name(path).to_ascii_lowercase();
        map.entry(name).or_default().push(path.clone());
    }
    map
}

fn find_sidecars(
    file_name: &str,
    stem: &str,
    allow_stem: bool,
    xmps: &HashMap<String, Vec<PathBuf>>,
) -> (Option<PathBuf>, Vec<PathBuf>) {
    let full_name = format!("{file_name}.xmp").to_ascii_lowercase();
    let stem_name = format!("{stem}.xmp").to_ascii_lowercase();
    let mut found = Vec::new();
    let mut seen = std::collections::HashSet::new();

    if let Some(paths) = xmps.get(&full_name) {
        for path in paths {
            if seen.insert(path.clone()) {
                found.push(path.clone());
            }
        }
    }
    if allow_stem {
        if let Some(paths) = xmps.get(&stem_name) {
            for path in paths {
                if seen.insert(path.clone()) {
                    found.push(path.clone());
                }
            }
        }
    }

    let primary = found.first().cloned();
    let extras = found.into_iter().skip(1).collect();
    (primary, extras)
}

#[allow(clippy::too_many_arguments)]
fn make_asset(
    relative_dir: &str,
    record: FileRecord,
    kind: AssetKind,
    companion_jpeg: Option<PathBuf>,
    extra_companions: Vec<PathBuf>,
    xmp_sidecar: Option<PathBuf>,
    extra_sidecars: Vec<PathBuf>,
    preview_hint: PreviewSource,
    width: Option<u32>,
    height: Option<u32>,
) -> PhotoAsset {
    let id = blake3::hash(record.path.as_os_str().as_encoded_bytes())
        .to_hex()
        .to_string();
    PhotoAsset {
        id,
        primary_path: record.path,
        file_name: record.file_name,
        stem: record.stem,
        extension: record.extension,
        kind,
        companion_jpeg,
        xmp_sidecar,
        extra_sidecars,
        extra_companions,
        preview_hint,
        file_size: record.file_size,
        modified_ms: record.modified_ms,
        width,
        height,
        relative_dir: relative_dir.to_string(),
    }
}

fn read_dimensions(path: &Path, extension: &str) -> (Option<u32>, Option<u32>) {
    if !matches!(extension, "jpg" | "jpeg" | "png" | "tif" | "tiff" | "webp") {
        return (None, None);
    }
    let Ok(size) = imagesize::size(path) else {
        return (None, None);
    };
    let mut width = size.width as u32;
    let mut height = size.height as u32;
    if matches!(extension, "jpg" | "jpeg" | "tif" | "tiff") {
        if let Some(orientation) = read_orientation(path) {
            if (5..=8).contains(&orientation) {
                std::mem::swap(&mut width, &mut height);
            }
        }
    }
    (Some(width), Some(height))
}

pub fn read_orientation(path: &Path) -> Option<u16> {
    let file = fs::File::open(path).ok()?;
    let mut reader = std::io::BufReader::new(file);
    let exif = exif::Reader::new().read_from_container(&mut reader).ok()?;
    exif.get_field(exif::Tag::Orientation, exif::In::PRIMARY)
        .and_then(|field| field.value.get_uint(0))
        .map(|value| value as u16)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn write_jpeg(path: &Path) {
        let image = image::RgbImage::from_fn(16, 10, |x, y| {
            image::Rgb([(x * 16) as u8, (y * 24) as u8, 120])
        });
        image.save(path).unwrap();
    }

    #[test]
    fn associates_raw_jpeg_and_both_sidecar_styles() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path();
        fs::write(root.join("IMG_0001.CR2"), b"raw").unwrap();
        write_jpeg(&root.join("IMG_0001.jpg"));
        write_jpeg(&root.join("IMG_0001.jpeg"));
        fs::write(root.join("IMG_0001.CR2.xmp"), b"<xmp/>").unwrap();
        fs::write(root.join("IMG_0001.xmp"), b"<xmp/>").unwrap();
        write_jpeg(&root.join("standalone.jpg"));
        fs::write(root.join("standalone.jpg.xmp"), b"<xmp/>").unwrap();
        fs::write(root.join("orphan.xmp"), b"<xmp/>").unwrap();

        let result = scan_photos(root, false).unwrap();
        assert_eq!(result.assets.len(), 2);

        let raw = result
            .assets
            .iter()
            .find(|asset| asset.extension == "cr2")
            .unwrap();
        assert_eq!(raw.kind, AssetKind::Raw);
        assert_eq!(
            raw.companion_jpeg.as_deref(),
            Some(root.join("IMG_0001.jpg").as_path())
        );
        assert_eq!(raw.preview_hint, PreviewSource::CompanionJpeg);
        assert_eq!(
            raw.xmp_sidecar.as_deref(),
            Some(root.join("IMG_0001.CR2.xmp").as_path())
        );
        assert_eq!(raw.extra_sidecars.len(), 1);
        assert_eq!(raw.extra_companions.len(), 1);

        let standalone = result
            .assets
            .iter()
            .find(|asset| asset.file_name == "standalone.jpg")
            .unwrap();
        assert_eq!(standalone.kind, AssetKind::Jpeg);
        assert!(standalone.xmp_sidecar.is_some());
    }

    #[test]
    fn recursive_scan_does_not_cross_directories_for_association() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path();
        let one = root.join("one");
        let two = root.join("two");
        fs::create_dir_all(&one).unwrap();
        fs::create_dir_all(&two).unwrap();
        fs::write(one.join("IMG_1.CR2"), b"raw").unwrap();
        write_jpeg(&two.join("IMG_1.jpg"));

        let direct = scan_photos(root, false).unwrap();
        assert!(direct.assets.is_empty());

        let recursive = scan_photos(root, true).unwrap();
        assert_eq!(recursive.assets.len(), 2);
        assert!(
            recursive
                .assets
                .iter()
                .all(|asset| asset.companion_jpeg.is_none())
        );
    }

    #[test]
    fn hidden_and_package_directories_are_skipped() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path();
        fs::create_dir_all(root.join(".hidden")).unwrap();
        fs::create_dir_all(root.join("Library.photoslibrary")).unwrap();
        fs::create_dir_all(root.join("visible")).unwrap();
        write_jpeg(&root.join("visible/photo.jpg"));
        write_jpeg(&root.join(".hidden/photo.jpg"));
        write_jpeg(&root.join("Library.photoslibrary/photo.jpg"));

        let result = scan_photos(root, true).unwrap();
        assert_eq!(result.assets.len(), 1);
        assert_eq!(result.assets[0].relative_dir, "visible");
    }
}
