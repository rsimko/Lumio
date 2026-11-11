use std::collections::{BTreeMap, HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

use natord::compare_ignore_case;
use walkdir::WalkDir;

use crate::error::{LumioError, Result};
use crate::ingest::types::{IngestFileRole, IngestSourceFile, IngestWarning};
use crate::path_utils::{canonical_existing, display_name, is_hidden_or_package, modified_ms};
use crate::types::{extension_of, is_jpeg_extension, is_raw_extension, raw_priority};

const INTERNAL_TEMP_DIR: &str = ".lumio-ingest-tmp";

#[derive(Debug, Clone)]
pub struct SourceEntity {
    pub id: String,
    pub source_dir: PathBuf,
    pub stem: String,
    pub primary_path: PathBuf,
    pub primary_role: IngestFileRole,
    pub files: Vec<IngestSourceFile>,
}

#[derive(Debug, Default)]
struct DirBucket {
    raws: Vec<IngestSourceFile>,
    jpegs: Vec<IngestSourceFile>,
    xmps: Vec<IngestSourceFile>,
}

pub fn scan_sources(sources: &[PathBuf]) -> Result<(Vec<SourceEntity>, Vec<IngestWarning>)> {
    if sources.is_empty() {
        return Err(LumioError::InvalidPath(
            "Choose at least one source folder or file.".into(),
        ));
    }

    let mut warnings = Vec::new();
    let mut seen = HashSet::new();
    let mut buckets: BTreeMap<PathBuf, DirBucket> = BTreeMap::new();

    for source in sources {
        let canonical = canonical_existing(source)?;
        let metadata =
            fs::metadata(&canonical).map_err(|error| LumioError::io(&canonical, error))?;
        if metadata.is_dir() {
            scan_directory(&canonical, &mut buckets, &mut seen, &mut warnings)?;
        } else if metadata.is_file() {
            add_individual_file(&canonical, &mut buckets, &mut seen, &mut warnings)?;
        } else {
            warnings.push(IngestWarning {
                path: Some(canonical),
                entity_id: None,
                message: "The selected source is neither a file nor a folder.".into(),
            });
        }
    }

    let entities = build_entities(buckets, &mut warnings);
    if entities.is_empty() {
        return Err(LumioError::InvalidPath(
            "No supported RAW, JPEG or XMP files were found in the selected sources.".into(),
        ));
    }
    Ok((entities, warnings))
}

fn scan_directory(
    root: &Path,
    buckets: &mut BTreeMap<PathBuf, DirBucket>,
    seen: &mut HashSet<PathBuf>,
    warnings: &mut Vec<IngestWarning>,
) -> Result<()> {
    // Surface an unreadable root instead of pretending it was empty.
    fs::read_dir(root).map_err(|error| LumioError::io(root, error))?;
    let walker = WalkDir::new(root)
        .follow_links(false)
        .into_iter()
        .filter_entry(|entry| {
            if entry.depth() == 0 {
                return true;
            }
            if entry.file_type().is_symlink() {
                return false;
            }
            if entry.file_type().is_dir() {
                return !is_hidden_or_package(&entry.file_name())
                    && entry.file_name() != INTERNAL_TEMP_DIR;
            }
            true
        });

    for entry in walker {
        let entry = match entry {
            Ok(entry) => entry,
            Err(error) => {
                warnings.push(IngestWarning {
                    path: error.path().map(Path::to_path_buf),
                    entity_id: None,
                    message: error.to_string(),
                });
                continue;
            }
        };
        if !entry.file_type().is_file() || is_hidden_or_package(entry.file_name()) {
            continue;
        }
        add_record(entry.path(), buckets, seen, warnings);
    }
    Ok(())
}

fn add_individual_file(
    path: &Path,
    buckets: &mut BTreeMap<PathBuf, DirBucket>,
    seen: &mut HashSet<PathBuf>,
    warnings: &mut Vec<IngestWarning>,
) -> Result<()> {
    let Some(family) = family_stem(path) else {
        warnings.push(IngestWarning {
            path: Some(path.to_path_buf()),
            entity_id: None,
            message: "This file type is not supported by ingest and was ignored.".into(),
        });
        return Ok(());
    };

    // A selected RAW/JPEG/XMP file implies its same-stem companions. This keeps
    // one logical photo together even when it is selected file by file.
    let Some(parent) = path.parent() else {
        return Ok(());
    };
    let read = fs::read_dir(parent).map_err(|error| LumioError::io(parent, error))?;
    for entry in read.flatten() {
        let candidate = entry.path();
        let Ok(file_type) = entry.file_type() else {
            continue;
        };
        if !file_type.is_file()
            || file_type.is_symlink()
            || is_hidden_or_package(&entry.file_name())
            || family_stem(&candidate).as_deref() != Some(family.as_str())
        {
            continue;
        }
        add_record(&candidate, buckets, seen, warnings);
    }
    // Always include the selected file, even if the directory listing was odd.
    add_record(path, buckets, seen, warnings);
    Ok(())
}

fn family_stem(path: &Path) -> Option<String> {
    let extension = extension_of(path)?;
    if is_raw_extension(&extension) || is_jpeg_extension(&extension) {
        return path
            .file_stem()
            .map(|value| value.to_string_lossy().to_ascii_lowercase());
    }
    if extension == "xmp" {
        let base = path
            .file_stem()
            .map(|value| value.to_string_lossy().to_ascii_lowercase())?;
        if let Some((stem, ext)) = base.rsplit_once('.') {
            if is_raw_extension(ext) || is_jpeg_extension(ext) {
                return Some(stem.to_string());
            }
        }
        return Some(base);
    }
    None
}

fn add_record(
    path: &Path,
    buckets: &mut BTreeMap<PathBuf, DirBucket>,
    seen: &mut HashSet<PathBuf>,
    warnings: &mut Vec<IngestWarning>,
) {
    let Some(extension) = extension_of(path) else {
        return;
    };
    let role = if is_raw_extension(&extension) {
        IngestFileRole::Raw
    } else if is_jpeg_extension(&extension) {
        IngestFileRole::Jpeg
    } else if extension == "xmp" {
        IngestFileRole::Xmp
    } else {
        return;
    };

    let key = path.to_path_buf();
    if !seen.insert(key.clone()) {
        return;
    }
    let metadata = match fs::metadata(path) {
        Ok(metadata) if metadata.is_file() => metadata,
        Ok(_) => return,
        Err(error) => {
            warnings.push(IngestWarning {
                path: Some(path.to_path_buf()),
                entity_id: None,
                message: error.to_string(),
            });
            return;
        }
    };
    let record = IngestSourceFile {
        path: path.to_path_buf(),
        role,
        size: metadata.len(),
        modified_ms: modified_ms(&metadata),
    };
    let parent = path
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| path.to_path_buf());
    let bucket = buckets.entry(parent).or_default();
    match role {
        IngestFileRole::Raw => bucket.raws.push(record),
        IngestFileRole::Jpeg => bucket.jpegs.push(record),
        IngestFileRole::Xmp => bucket.xmps.push(record),
    }
}

fn jpeg_priority(extension: &str) -> usize {
    match extension {
        "jpg" => 0,
        "jpeg" => 1,
        _ => 2,
    }
}

fn extension_of_record(record: &IngestSourceFile) -> String {
    extension_of(&record.path).unwrap_or_default()
}

fn sort_photos(records: &mut [IngestSourceFile]) {
    records.sort_by(|left, right| {
        let left_ext = extension_of_record(left);
        let right_ext = extension_of_record(right);
        let priority = if is_raw_extension(&left_ext) {
            raw_priority(&left_ext).cmp(&raw_priority(&right_ext))
        } else {
            jpeg_priority(&left_ext).cmp(&jpeg_priority(&right_ext))
        };
        priority.then_with(|| {
            compare_ignore_case(&display_name(&left.path), &display_name(&right.path))
        })
    });
}

fn build_entities(
    buckets: BTreeMap<PathBuf, DirBucket>,
    warnings: &mut Vec<IngestWarning>,
) -> Vec<SourceEntity> {
    let mut entities = Vec::new();
    for (directory, mut bucket) in buckets {
        sort_photos(&mut bucket.raws);
        sort_photos(&mut bucket.jpegs);
        bucket.xmps.sort_by(|left, right| {
            compare_ignore_case(&display_name(&left.path), &display_name(&right.path))
        });

        let mut groups: BTreeMap<String, Vec<IngestSourceFile>> = BTreeMap::new();
        for record in bucket.raws.into_iter().chain(bucket.jpegs) {
            let stem = record
                .path
                .file_stem()
                .map(|value| value.to_string_lossy().to_ascii_lowercase())
                .unwrap_or_default();
            groups.entry(stem).or_default().push(record);
        }

        let stems: Vec<String> = groups.keys().cloned().collect();
        let mut xmp_by_group: HashMap<String, Vec<IngestSourceFile>> = HashMap::new();
        for xmp in bucket.xmps {
            let base = xmp
                .path
                .file_stem()
                .map(|value| value.to_string_lossy().to_ascii_lowercase())
                .unwrap_or_default();
            let mut matching: Option<&String> = None;
            for stem in &stems {
                let is_match = base == *stem
                    || (base.starts_with(stem)
                        && base.as_bytes().get(stem.len()) == Some(&b'.')
                        && base[stem.len() + 1..]
                            .split('.')
                            .all(|part| is_raw_extension(part) || is_jpeg_extension(part)));
                if is_match
                    && matching
                        .map(|current| stem.len() > current.len())
                        .unwrap_or(true)
                {
                    matching = Some(stem);
                }
            }
            match matching {
                Some(stem) => xmp_by_group.entry(stem.clone()).or_default().push(xmp),
                None => warnings.push(IngestWarning {
                    path: Some(xmp.path.clone()),
                    entity_id: None,
                    message: "XMP sidecar has no matching RAW or JPEG photo and was ignored."
                        .into(),
                }),
            }
        }

        for (stem, mut photos) in groups {
            photos.sort_by(|left, right| {
                let left_raw = left.role == IngestFileRole::Raw;
                let right_raw = right.role == IngestFileRole::Raw;
                right_raw.cmp(&left_raw).then_with(|| {
                    let left_ext = extension_of_record(left);
                    let right_ext = extension_of_record(right);
                    if left_raw {
                        raw_priority(&left_ext).cmp(&raw_priority(&right_ext))
                    } else {
                        jpeg_priority(&left_ext).cmp(&jpeg_priority(&right_ext))
                    }
                })
            });
            let Some(primary) = photos.first().cloned() else {
                continue;
            };
            let mut files = photos;
            if let Some(xmps) = xmp_by_group.remove(&stem) {
                files.extend(xmps);
            }
            let display_stem = primary
                .path
                .file_stem()
                .map(|value| value.to_string_lossy().into_owned())
                .unwrap_or_else(|| stem.clone());
            let id = blake3::hash(
                format!(
                    "{}\u{0}{}",
                    directory.to_string_lossy().to_ascii_lowercase(),
                    stem
                )
                .as_bytes(),
            )
            .to_hex()
            .to_string();
            entities.push(SourceEntity {
                id,
                source_dir: directory.clone(),
                stem: display_stem,
                primary_path: primary.path.clone(),
                primary_role: primary.role,
                files,
            });
        }
    }

    entities.sort_by(|left, right| {
        compare_ignore_case(
            &left.source_dir.to_string_lossy(),
            &right.source_dir.to_string_lossy(),
        )
        .then_with(|| compare_ignore_case(&left.stem, &right.stem))
        .then_with(|| left.id.cmp(&right.id))
    });
    entities
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn groups_raw_jpeg_and_xmp_into_one_entity() {
        let temp = tempfile::tempdir().unwrap();
        let dir = temp.path();
        fs::write(dir.join("IMG_0001.CR3"), b"raw").unwrap();
        fs::write(dir.join("IMG_0001.jpg"), b"jpeg").unwrap();
        fs::write(dir.join("IMG_0001.CR3.xmp"), b"<xmp/>").unwrap();
        fs::write(dir.join("notes.txt"), b"ignored").unwrap();

        let (entities, warnings) = scan_sources(&[dir.to_path_buf()]).unwrap();
        assert!(warnings.is_empty());
        assert_eq!(entities.len(), 1);
        let entity = &entities[0];
        assert_eq!(entity.primary_role, IngestFileRole::Raw);
        assert_eq!(entity.files.len(), 3);
        assert!(
            entity
                .files
                .iter()
                .any(|file| file.role == IngestFileRole::Jpeg)
        );
        assert!(
            entity
                .files
                .iter()
                .any(|file| file.role == IngestFileRole::Xmp)
        );
    }

    #[test]
    fn standalone_jpeg_is_an_entity_only_without_raw() {
        let temp = tempfile::tempdir().unwrap();
        let dir = temp.path();
        fs::write(dir.join("IMG_0002.jpg"), b"jpeg").unwrap();
        fs::write(dir.join("IMG_0002.xmp"), b"<xmp/>").unwrap();
        fs::write(dir.join("IMG_0003.CR2"), b"raw").unwrap();
        fs::write(dir.join("IMG_0003.JPG"), b"jpeg").unwrap();

        let (entities, _) = scan_sources(&[dir.to_path_buf()]).unwrap();
        assert_eq!(entities.len(), 2);
        let standalone = entities
            .iter()
            .find(|entity| entity.stem == "IMG_0002")
            .unwrap();
        assert_eq!(standalone.primary_role, IngestFileRole::Jpeg);
        assert_eq!(standalone.files.len(), 2);
        let raw = entities
            .iter()
            .find(|entity| entity.stem == "IMG_0003")
            .unwrap();
        assert_eq!(raw.files.len(), 2);
    }

    #[test]
    fn multiple_raws_share_one_entity_and_select_raw_first() {
        let temp = tempfile::tempdir().unwrap();
        let dir = temp.path();
        fs::write(dir.join("shot.dng"), b"raw").unwrap();
        fs::write(dir.join("shot.cr3"), b"raw").unwrap();
        fs::write(dir.join("shot.nef"), b"raw").unwrap();

        let (entities, _) = scan_sources(&[dir.to_path_buf()]).unwrap();
        assert_eq!(entities.len(), 1);
        assert_eq!(entities[0].files.len(), 3);
        assert!(entities[0].primary_path.ends_with("shot.cr3"));
    }

    #[test]
    fn selecting_an_individual_raw_brings_companions() {
        let temp = tempfile::tempdir().unwrap();
        let dir = temp.path();
        let raw = dir.join("palawan-112.RAF");
        fs::write(&raw, b"raw").unwrap();
        fs::write(dir.join("palawan-112.jpg"), b"jpeg").unwrap();
        fs::write(dir.join("palawan-112.RAF.xmp"), b"<xmp/>").unwrap();
        fs::write(dir.join("unrelated.jpg"), b"jpeg").unwrap();

        let (entities, _) = scan_sources(&[raw]).unwrap();
        assert_eq!(entities.len(), 1);
        assert_eq!(entities[0].files.len(), 3);
    }

    #[test]
    fn orphan_xmp_produces_a_warning() {
        let temp = tempfile::tempdir().unwrap();
        let dir = temp.path();
        fs::write(dir.join("photo.jpg"), b"jpeg").unwrap();
        fs::write(dir.join("orphan.xmp"), b"<xmp/>").unwrap();

        let (entities, warnings) = scan_sources(&[dir.to_path_buf()]).unwrap();
        assert_eq!(entities.len(), 1);
        assert_eq!(warnings.len(), 1);
        assert!(warnings[0].message.contains("no matching RAW or JPEG"));
    }

    #[test]
    fn xmp_naming_variants_associate() {
        let temp = tempfile::tempdir().unwrap();
        let dir = temp.path();
        fs::write(dir.join("A.CR3"), b"raw").unwrap();
        fs::write(dir.join("A.CR3.xmp"), b"<xmp/>").unwrap();
        fs::write(dir.join("B.jpg"), b"jpeg").unwrap();
        fs::write(dir.join("B.xmp"), b"<xmp/>").unwrap();

        let (entities, _) = scan_sources(&[dir.to_path_buf()]).unwrap();
        assert_eq!(entities.len(), 2);
        assert!(entities.iter().all(|entity| entity.files.len() == 2));
    }
}
