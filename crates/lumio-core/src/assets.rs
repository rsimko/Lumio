use std::collections::HashSet;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use crate::error::{LumioError, Result};
use crate::path_utils::{canonical_dir, is_hidden_or_package, is_within_roots};
use crate::types::{MoveAssetsOutcome, PhotoAsset, RootFolder, TrashAssetsOutcome};

/// Every file that belongs to one visible photo. The primary file always comes
/// first; companion JPEGs and XMP sidecars follow in scan association order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AssetFileGroup {
    pub primary_path: PathBuf,
    pub name: String,
    pub paths: Vec<PathBuf>,
}

/// Stable identifier for a photo group. It matches `PhotoAsset::id` for assets
/// discovered by a scan (BLAKE3 of the primary path bytes).
pub fn asset_group_id(primary_path: &Path) -> String {
    blake3::hash(primary_path.as_os_str().as_encoded_bytes())
        .to_hex()
        .to_string()
}

/// Expand a photo into its primary file and every associated file, preserving
/// order and removing exact duplicates.
pub fn asset_file_paths(asset: &PhotoAsset) -> Vec<PathBuf> {
    let mut paths =
        Vec::with_capacity(1 + asset.extra_companions.len() + 1 + asset.extra_sidecars.len());
    let mut seen = HashSet::new();
    let mut push = |path: &PathBuf| {
        if seen.insert(path.clone()) {
            paths.push(path.clone());
        }
    };
    push(&asset.primary_path);
    if let Some(companion) = &asset.companion_jpeg {
        push(companion);
    }
    for companion in &asset.extra_companions {
        push(companion);
    }
    if let Some(sidecar) = &asset.xmp_sidecar {
        push(sidecar);
    }
    for sidecar in &asset.extra_sidecars {
        push(sidecar);
    }
    paths
}

/// Expand and validate a batch of photos into deduplicated file groups.
///
/// Every file must be a regular file (not a symlink) under one of the
/// registered roots, and every associated file must live in the primary
/// file's directory. Repeated assets are collapsed by canonical primary path.
pub fn group_assets(assets: &[PhotoAsset], roots: &[RootFolder]) -> Result<Vec<AssetFileGroup>> {
    let mut groups = Vec::new();
    let mut seen_primary = HashSet::new();

    for asset in assets {
        let raw_paths = asset_file_paths(asset);
        let mut paths = Vec::with_capacity(raw_paths.len());
        for raw in raw_paths {
            paths.push(canonical_asset_file(&raw, roots)?);
        }
        let Some(primary_path) = paths.first().cloned() else {
            continue;
        };
        if !seen_primary.insert(primary_path.clone()) {
            continue;
        }
        let parent = primary_path.parent().map(Path::to_path_buf);
        for path in paths.iter().skip(1) {
            if path.parent().map(Path::to_path_buf) != parent {
                return Err(LumioError::InvalidPath(format!(
                    "{} is not in the same folder as {}",
                    path.display(),
                    primary_path.display()
                )));
            }
        }
        groups.push(AssetFileGroup {
            primary_path,
            name: asset.file_name.clone(),
            paths,
        });
    }

    Ok(groups)
}

/// Deduplicated file paths for a batch of photos, used by scans to hide every
/// associated file, not only the primary one.
pub fn asset_group_paths(groups: &[AssetFileGroup]) -> Vec<PathBuf> {
    let mut seen = HashSet::new();
    let mut paths = Vec::new();
    for group in groups {
        for path in &group.paths {
            if seen.insert(path.clone()) {
                paths.push(path.clone());
            }
        }
    }
    paths
}

fn canonical_asset_file(path: &Path, roots: &[RootFolder]) -> Result<PathBuf> {
    let metadata =
        fs::symlink_metadata(path).map_err(|error| LumioError::io(path.to_path_buf(), error))?;
    if metadata.file_type().is_symlink() {
        return Err(LumioError::InvalidPath(format!(
            "Refusing to operate on the symlink {}.",
            path.display()
        )));
    }
    if !metadata.is_file() {
        return Err(LumioError::InvalidPath(format!(
            "{} is not a regular file.",
            path.display()
        )));
    }
    if let Some(name) = path.file_name() {
        if is_hidden_or_package(name) {
            return Err(LumioError::InvalidPath(format!(
                "Refusing to operate on the protected file {}.",
                path.display()
            )));
        }
    }
    let canonical =
        fs::canonicalize(path).map_err(|error| LumioError::io(path.to_path_buf(), error))?;
    if !within_roots(&canonical, roots) {
        return Err(LumioError::OutsideRoots(canonical));
    }
    Ok(canonical)
}

fn within_roots(path: &Path, roots: &[RootFolder]) -> bool {
    let paths: Vec<PathBuf> = roots
        .iter()
        .map(|root| fs::canonicalize(&root.path).unwrap_or_else(|_| root.path.clone()))
        .collect();
    is_within_roots(path, &paths)
}

/// Move a batch of photos, including every associated file, into `destination`.
///
/// The whole batch is preflighted before anything is moved: destination name
/// conflicts and duplicate names inside the batch abort the operation without
/// touching the disk. If a later move fails, already-moved files are moved back
/// where possible.
pub fn move_assets(
    assets: &[PhotoAsset],
    destination: &Path,
    roots: &[RootFolder],
) -> Result<MoveAssetsOutcome> {
    let destination = resolve_destination(destination, roots)?;
    let groups = group_assets(assets, roots)?;
    if groups.is_empty() {
        return Ok(MoveAssetsOutcome {
            destination,
            moved_assets: 0,
            moved_files: 0,
            skipped_assets: 0,
        });
    }

    let mut target_names: HashSet<PathBuf> = HashSet::new();
    let mut plans: Vec<Vec<(PathBuf, PathBuf)>> = Vec::new();
    let mut skipped_assets = 0;

    for group in groups {
        let mut moves = Vec::new();
        for source in &group.paths {
            // A photo that already lives in the destination is a no-op.
            if source.parent() == Some(destination.as_path()) {
                continue;
            }
            let Some(file_name) = source.file_name() else {
                return Err(LumioError::InvalidPath(format!(
                    "{} has no file name.",
                    source.display()
                )));
            };
            let target = destination.join(file_name);
            // Use symlink_metadata so even a dangling symlink counts as an
            // existing entry that must not be overwritten.
            if fs::symlink_metadata(&target).is_ok() {
                return Err(LumioError::InvalidPath(format!(
                    "Refusing to overwrite {} in {}.",
                    file_name.to_string_lossy(),
                    destination.display()
                )));
            }
            if !target_names.insert(target.clone()) {
                return Err(LumioError::InvalidPath(format!(
                    "Two selected photos would both be written to {}.",
                    target.display()
                )));
            }
            moves.push((source.clone(), target));
        }

        if moves.is_empty() {
            skipped_assets += 1;
        } else {
            plans.push(moves);
        }
    }

    let mut moved: Vec<(PathBuf, PathBuf)> = Vec::new();
    for plan in &plans {
        for (source, target) in plan {
            if let Err(error) = move_one(source, target) {
                let rollback = rollback(&moved);
                let suffix = if rollback.is_empty() {
                    String::new()
                } else {
                    format!(" Rollback problems: {}", rollback.join("; "))
                };
                return Err(LumioError::Move(format!(
                    "Could not move {} to {}: {error}.{suffix}",
                    source.display(),
                    target.display()
                )));
            }
            moved.push((source.clone(), target.clone()));
        }
    }

    Ok(MoveAssetsOutcome {
        destination,
        moved_assets: plans.len() as u32,
        moved_files: moved.len() as u32,
        skipped_assets,
    })
}

/// Move every associated file to the operating system Trash.
pub fn trash_assets(assets: &[PhotoAsset], roots: &[RootFolder]) -> Result<TrashAssetsOutcome> {
    let groups = group_assets(assets, roots)?;
    let trashed_assets = groups.len() as u32;
    let paths = asset_group_paths(&groups);

    let mut trashed_files = 0u32;
    for path in paths {
        move_file_to_trash(&path).map_err(|error| LumioError::TrashFiles(error.to_string()))?;
        trashed_files += 1;
    }
    Ok(TrashAssetsOutcome {
        trashed_assets,
        trashed_files,
    })
}

fn resolve_destination(destination: &Path, roots: &[RootFolder]) -> Result<PathBuf> {
    let canonical = canonical_dir(destination)?;
    if !within_roots(&canonical, roots) {
        return Err(LumioError::OutsideRoots(canonical));
    }
    if let Some(name) = canonical.file_name() {
        if is_hidden_or_package(name) {
            return Err(LumioError::InvalidPath(format!(
                "Refusing to move photos into the protected folder {}.",
                canonical.display()
            )));
        }
    }
    Ok(canonical)
}

fn move_one(source: &Path, target: &Path) -> io::Result<()> {
    match fs::rename(source, target) {
        Ok(()) => Ok(()),
        Err(error) if is_cross_device(&error) => {
            fs::copy(source, target)?;
            if let Err(remove_error) = fs::remove_file(source) {
                // Do not leave a duplicate behind if the source could not be
                // removed after a successful cross-volume copy.
                let _ = fs::remove_file(target);
                return Err(remove_error);
            }
            Ok(())
        }
        Err(error) => Err(error),
    }
}

fn is_cross_device(error: &io::Error) -> bool {
    // EXDEV is 18 on macOS and Linux. `ErrorKind::CrossesDevices` is the
    // portable spelling on current Rust toolchains.
    error.kind() == io::ErrorKind::CrossesDevices || error.raw_os_error() == Some(18)
}

fn rollback(moved: &[(PathBuf, PathBuf)]) -> Vec<String> {
    let mut problems = Vec::new();
    for (source, target) in moved.iter().rev() {
        if let Err(error) = move_one(target, source) {
            problems.push(format!("could not restore {} ({error})", source.display()));
        }
    }
    problems
}

fn move_file_to_trash(path: &Path) -> std::result::Result<(), trash::Error> {
    let mut context = trash::TrashContext::new();
    // Finder automation needs extra permissions and can be blocked by macOS.
    // NSFileManager moves the file to Trash without prompting for automation
    // access and still remains recoverable through the Trash.
    #[cfg(target_os = "macos")]
    {
        use trash::macos::{DeleteMethod, TrashContextExtMacos};
        context.set_delete_method(DeleteMethod::NsFileManager);
    }
    context.delete(path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{AssetKind, PreviewSource};

    fn root(path: &Path) -> RootFolder {
        RootFolder {
            id: "root".into(),
            path: fs::canonicalize(path).unwrap(),
            added_at: 0,
        }
    }

    fn asset(primary: &Path, companion: Option<&Path>, sidecar: Option<&Path>) -> PhotoAsset {
        PhotoAsset {
            id: asset_group_id(primary),
            primary_path: primary.to_path_buf(),
            file_name: primary.file_name().unwrap().to_string_lossy().into_owned(),
            stem: primary.file_stem().unwrap().to_string_lossy().into_owned(),
            extension: primary
                .extension()
                .unwrap()
                .to_string_lossy()
                .to_ascii_lowercase(),
            kind: AssetKind::Raw,
            companion_jpeg: companion.map(Path::to_path_buf),
            xmp_sidecar: sidecar.map(Path::to_path_buf),
            extra_sidecars: Vec::new(),
            extra_companions: Vec::new(),
            preview_hint: PreviewSource::CompanionJpeg,
            file_size: 3,
            modified_ms: 0,
            width: None,
            height: None,
            relative_dir: String::new(),
            metadata: Default::default(),
        }
    }

    #[test]
    fn groups_include_every_associated_file_once() {
        let temp = tempfile::tempdir().unwrap();
        let root_dir = temp.path().join("photos");
        fs::create_dir_all(&root_dir).unwrap();
        let raw = root_dir.join("IMG_0001.CR2");
        let jpeg = root_dir.join("IMG_0001.jpg");
        let xmp = root_dir.join("IMG_0001.CR2.xmp");
        let extra = root_dir.join("IMG_0001.xmp");
        fs::write(&raw, b"raw").unwrap();
        fs::write(&jpeg, b"jpeg").unwrap();
        fs::write(&xmp, b"xmp").unwrap();
        fs::write(&extra, b"xmp").unwrap();

        let mut photo = asset(&raw, Some(&jpeg), Some(&xmp));
        photo.extra_sidecars = vec![xmp.clone(), extra.clone()];

        let roots = vec![root(&root_dir)];
        let groups = group_assets(&[photo], &roots).unwrap();
        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].paths.len(), 4);
        assert_eq!(groups[0].primary_path, fs::canonicalize(&raw).unwrap());
    }

    #[test]
    fn move_moves_the_whole_group_into_the_destination() {
        let temp = tempfile::tempdir().unwrap();
        let root_dir = temp.path().join("photos");
        let source = root_dir.join("source");
        let destination = root_dir.join("destination");
        fs::create_dir_all(&source).unwrap();
        fs::create_dir_all(&destination).unwrap();
        let raw = source.join("IMG_0001.CR2");
        let jpeg = source.join("IMG_0001.jpg");
        let xmp = source.join("IMG_0001.CR2.xmp");
        fs::write(&raw, b"raw").unwrap();
        fs::write(&jpeg, b"jpeg").unwrap();
        fs::write(&xmp, b"xmp").unwrap();

        let roots = vec![root(&root_dir)];
        let outcome = move_assets(
            &[asset(&raw, Some(&jpeg), Some(&xmp))],
            &destination,
            &roots,
        )
        .unwrap();

        assert_eq!(outcome.moved_assets, 1);
        assert_eq!(outcome.moved_files, 3);
        assert_eq!(outcome.skipped_assets, 0);
        assert!(!raw.exists());
        assert!(!jpeg.exists());
        assert!(!xmp.exists());
        assert!(destination.join("IMG_0001.CR2").exists());
        assert!(destination.join("IMG_0001.jpg").exists());
        assert!(destination.join("IMG_0001.CR2.xmp").exists());
    }

    #[test]
    fn move_skips_assets_already_in_the_destination() {
        let temp = tempfile::tempdir().unwrap();
        let root_dir = temp.path().join("photos");
        fs::create_dir_all(&root_dir).unwrap();
        let raw = root_dir.join("IMG_0001.CR2");
        fs::write(&raw, b"raw").unwrap();

        let roots = vec![root(&root_dir)];
        let outcome = move_assets(&[asset(&raw, None, None)], &root_dir, &roots).unwrap();
        assert_eq!(outcome.moved_assets, 0);
        assert_eq!(outcome.skipped_assets, 1);
        assert!(raw.exists());
    }

    #[test]
    fn move_refuses_to_overwrite_existing_files() {
        let temp = tempfile::tempdir().unwrap();
        let root_dir = temp.path().join("photos");
        let source = root_dir.join("source");
        let destination = root_dir.join("destination");
        fs::create_dir_all(&source).unwrap();
        fs::create_dir_all(&destination).unwrap();
        let raw = source.join("IMG_0001.CR2");
        fs::write(&raw, b"raw").unwrap();
        fs::write(destination.join("IMG_0001.CR2"), b"existing").unwrap();

        let roots = vec![root(&root_dir)];
        let error = move_assets(&[asset(&raw, None, None)], &destination, &roots).unwrap_err();
        assert!(matches!(error, LumioError::InvalidPath(_)));
        assert!(raw.exists());
        assert_eq!(
            fs::read(destination.join("IMG_0001.CR2")).unwrap(),
            b"existing"
        );
    }

    #[test]
    fn move_rejects_duplicate_destination_names_in_one_batch() {
        let temp = tempfile::tempdir().unwrap();
        let root_dir = temp.path().join("photos");
        let one = root_dir.join("one");
        let two = root_dir.join("two");
        let destination = root_dir.join("destination");
        fs::create_dir_all(&one).unwrap();
        fs::create_dir_all(&two).unwrap();
        fs::create_dir_all(&destination).unwrap();
        let first = one.join("IMG_0001.CR2");
        let second = two.join("IMG_0001.CR2");
        fs::write(&first, b"one").unwrap();
        fs::write(&second, b"two").unwrap();

        let roots = vec![root(&root_dir)];
        let error = move_assets(
            &[asset(&first, None, None), asset(&second, None, None)],
            &destination,
            &roots,
        )
        .unwrap_err();
        assert!(matches!(error, LumioError::InvalidPath(_)));
        assert!(first.exists());
        assert!(second.exists());
    }

    #[test]
    fn move_allows_cross_root_destinations() {
        let temp = tempfile::tempdir().unwrap();
        let first_root = temp.path().join("first");
        let second_root = temp.path().join("second");
        let destination = second_root.join("target");
        fs::create_dir_all(&first_root).unwrap();
        fs::create_dir_all(&destination).unwrap();
        let raw = first_root.join("IMG_0001.CR2");
        fs::write(&raw, b"raw").unwrap();

        let roots = vec![root(&first_root), root(&second_root)];
        let outcome = move_assets(&[asset(&raw, None, None)], &destination, &roots).unwrap();
        assert_eq!(outcome.moved_assets, 1);
        assert!(destination.join("IMG_0001.CR2").exists());
    }

    #[test]
    fn move_rejects_destinations_outside_the_registered_roots() {
        let temp = tempfile::tempdir().unwrap();
        let root_dir = temp.path().join("photos");
        let outside = temp.path().join("outside");
        fs::create_dir_all(&root_dir).unwrap();
        fs::create_dir_all(&outside).unwrap();
        let raw = root_dir.join("IMG_0001.CR2");
        fs::write(&raw, b"raw").unwrap();

        let roots = vec![root(&root_dir)];
        let error = move_assets(&[asset(&raw, None, None)], &outside, &roots).unwrap_err();
        assert!(matches!(error, LumioError::OutsideRoots(_)));
        assert!(raw.exists());
    }

    #[cfg(unix)]
    #[test]
    fn grouping_rejects_symlinked_files() {
        use std::os::unix::fs::symlink;

        let temp = tempfile::tempdir().unwrap();
        let root_dir = temp.path().join("photos");
        fs::create_dir_all(&root_dir).unwrap();
        let target = root_dir.join("target.CR2");
        let link = root_dir.join("link.CR2");
        fs::write(&target, b"raw").unwrap();
        symlink(&target, &link).unwrap();

        let roots = vec![root(&root_dir)];
        let error = group_assets(&[asset(&link, None, None)], &roots).unwrap_err();
        assert!(matches!(error, LumioError::InvalidPath(_)));
    }
}
