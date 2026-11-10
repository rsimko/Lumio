use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use uuid::Uuid;

use crate::error::{LumioError, Result};
use crate::path_utils::{canonical_dir, display_name, is_hidden_or_package, now_ms};
use crate::types::{
    AddFolderOutcome, AppConfig, RootFolder, RootInfo, RootRemovalMode, UiSettings,
};

const FORBIDDEN_DELETE_PATHS: &[&str] = &[
    "/",
    "/Applications",
    "/Library",
    "/Network",
    "/System",
    "/Users",
    "/Volumes",
    "/bin",
    "/boot",
    "/dev",
    "/etc",
    "/home",
    "/lib",
    "/lib64",
    "/media",
    "/mnt",
    "/opt",
    "/private",
    "/private/etc",
    "/private/tmp",
    "/private/var",
    "/proc",
    "/root",
    "/run",
    "/sbin",
    "/srv",
    "/sys",
    "/tmp",
    "/usr",
    "/var",
];

const FORBIDDEN_MOUNT_ROOTS: &[&str] = &["/Volumes", "/media", "/mnt", "/run/media"];

pub struct ConfigStore {
    path: PathBuf,
    config: AppConfig,
    recovery_warning: Option<String>,
}

impl ConfigStore {
    pub fn open(path: impl Into<PathBuf>) -> Result<Self> {
        let path = path.into();
        if !path.exists() {
            return Ok(Self {
                path,
                config: AppConfig::default(),
                recovery_warning: None,
            });
        }

        let raw = fs::read_to_string(&path).map_err(|error| LumioError::io(&path, error))?;
        match serde_json::from_str::<AppConfig>(&raw) {
            Ok(config) => Ok(Self {
                path,
                config,
                recovery_warning: None,
            }),
            Err(error) => {
                // Never silently destroy a config the user may still need.
                let backup = path.with_file_name(format!("settings.json.corrupt-{}", now_ms()));
                let _ = fs::rename(&path, &backup);
                Ok(Self {
                    path,
                    config: AppConfig::default(),
                    recovery_warning: Some(format!(
                        "Settings could not be read ({error}). Previous file kept at {}",
                        backup.display()
                    )),
                })
            }
        }
    }

    pub fn config(&self) -> &AppConfig {
        &self.config
    }

    pub fn recovery_warning(&self) -> Option<&str> {
        self.recovery_warning.as_deref()
    }

    pub fn roots(&self) -> &[RootFolder] {
        &self.config.roots
    }

    pub fn excluded_folders(&self) -> &[PathBuf] {
        &self.config.excluded_folders
    }

    pub fn root_paths(&self) -> Vec<PathBuf> {
        self.config
            .roots
            .iter()
            .map(|root| fs::canonicalize(&root.path).unwrap_or_else(|_| root.path.clone()))
            .collect()
    }

    pub fn root_infos(&self) -> Vec<RootInfo> {
        self.config
            .roots
            .iter()
            .map(|root| {
                let metadata = fs::metadata(&root.path);
                let (available, error) = match metadata {
                    Ok(metadata) if metadata.is_dir() => (true, None),
                    Ok(_) => (false, Some("Not a directory".to_string())),
                    Err(error) => (false, Some(error.to_string())),
                };
                RootInfo {
                    id: root.id.clone(),
                    path: root.path.clone(),
                    name: display_name(&root.path),
                    available,
                    error,
                    added_at: root.added_at,
                }
            })
            .collect()
    }

    pub fn add_root(&mut self, path: &Path) -> Result<AddFolderOutcome> {
        let canonical = canonical_dir(path)?;
        if let Some(existing) = self.config.roots.iter().find(|root| {
            fs::canonicalize(&root.path).unwrap_or_else(|_| root.path.clone()) == canonical
        }) {
            return Ok(AddFolderOutcome {
                root: self.root_info_by_id(&existing.id),
                folder: canonical,
                restored: false,
            });
        }

        // Re-adding a hidden subfolder restores it without creating a second
        // root; the registered parent already covers it.
        if self
            .config
            .excluded_folders
            .iter()
            .any(|hidden| hidden == &canonical)
        {
            let containing = self
                .config
                .roots
                .iter()
                .find(|root| {
                    let root_path =
                        fs::canonicalize(&root.path).unwrap_or_else(|_| root.path.clone());
                    canonical.starts_with(&root_path)
                })
                .map(|root| root.id.clone());
            if let Some(root_id) = containing {
                self.config
                    .excluded_folders
                    .retain(|hidden| hidden != &canonical);
                self.save()?;
                return Ok(AddFolderOutcome {
                    root: self.root_info_by_id(&root_id),
                    folder: canonical,
                    restored: true,
                });
            }
        }

        // Refuse to add a folder that is already covered by a registered root.
        // The existing root already lets the user browse it.
        if let Some(existing) = self.config.roots.iter().find(|root| {
            let root_path = fs::canonicalize(&root.path).unwrap_or_else(|_| root.path.clone());
            canonical.starts_with(&root_path)
        }) {
            return Err(LumioError::InvalidPath(format!(
                "{} is already inside the added folder {}",
                canonical.display(),
                existing.path.display()
            )));
        }

        let root = RootFolder {
            id: Uuid::new_v4().to_string(),
            path: canonical.clone(),
            added_at: now_ms(),
        };
        self.config.roots.push(root.clone());
        self.save()?;
        Ok(AddFolderOutcome {
            root: RootInfo {
                id: root.id,
                path: root.path,
                name: display_name(&canonical),
                available: true,
                error: None,
                added_at: root.added_at,
            },
            folder: canonical,
            restored: false,
        })
    }

    fn root_info_by_id(&self, id: &str) -> RootInfo {
        self.root_infos()
            .into_iter()
            .find(|root| root.id == id)
            .expect("root must exist")
    }

    pub fn remove_root(&mut self, id: &str, mode: RootRemovalMode) -> Result<bool> {
        let Some(root) = self.config.roots.iter().find(|root| root.id == id).cloned() else {
            return Ok(false);
        };
        let root_path = fs::canonicalize(&root.path).unwrap_or_else(|_| root.path.clone());

        if mode == RootRemovalMode::MoveToTrash {
            let path = canonical_dir(&root.path)?;
            ensure_safe_to_trash(&path, &root.id, &self.config.roots)?;
            // Move the folder to the system Trash before forgetting it so a
            // failure leaves the library entry intact and nothing is lost.
            move_to_trash(&path).map_err(|error| LumioError::Trash(error.to_string()))?;
        }

        self.config.roots.retain(|root| root.id != id);
        // Hidden subfolders only make sense while their root is registered.
        self.config
            .excluded_folders
            .retain(|hidden| !hidden.starts_with(&root_path));
        if self.config.ui.last_root_id.as_deref() == Some(id) {
            self.config.ui.last_root_id = None;
            self.config.ui.last_folder = None;
        }
        self.save()?;
        Ok(true)
    }

    /// Remove a subfolder from Lumio without unregistering its library root.
    /// The folder can either be hidden (files untouched) or moved to Trash.
    pub fn remove_subfolder(&mut self, path: &Path, mode: RootRemovalMode) -> Result<bool> {
        let (_, canonical) = resolve_trashable_subfolder(path, &self.config.roots)?;

        match mode {
            RootRemovalMode::LibraryOnly => {
                if !self.config.excluded_folders.contains(&canonical) {
                    self.config.excluded_folders.push(canonical.clone());
                }
            }
            RootRemovalMode::MoveToTrash => {
                // Move the path as given rather than its canonical target so a
                // symlinked entry is removed as a link, leaving its target intact.
                move_to_trash(path).map_err(|error| LumioError::Trash(error.to_string()))?;
                self.config
                    .excluded_folders
                    .retain(|hidden| !hidden.starts_with(&canonical));
            }
        }

        if let Some(last) = &self.config.ui.last_folder {
            let last = fs::canonicalize(last).unwrap_or_else(|_| last.clone());
            if last == canonical || last.starts_with(&canonical) {
                self.config.ui.last_root_id = None;
                self.config.ui.last_folder = None;
            }
        }

        self.save()?;
        Ok(true)
    }

    pub fn update_ui(&mut self, ui: UiSettings) -> Result<()> {
        self.config.ui = ui;
        self.save()
    }

    pub fn save(&self) -> Result<()> {
        save_atomic(&self.path, &self.config)
    }
}

/// Resolve a folder against the registered roots. The folder must live strictly
/// inside one root (not be the root itself) and pass the destructive-action
/// guards before it may be moved to Trash.
fn resolve_trashable_subfolder<'a>(
    path: &Path,
    roots: &'a [RootFolder],
) -> Result<(&'a RootFolder, PathBuf)> {
    let canonical = canonical_dir(path)?;
    for root in roots {
        let root_path = fs::canonicalize(&root.path).unwrap_or_else(|_| root.path.clone());
        if canonical == root_path {
            return Err(LumioError::InvalidPath(format!(
                "{} is a registered library folder; remove it from the library instead",
                path.display()
            )));
        }
        if canonical.starts_with(&root_path) {
            ensure_safe_to_trash(&canonical, &root.id, roots)?;
            return Ok((root, canonical));
        }
    }
    Err(LumioError::OutsideRoots(path.to_path_buf()))
}

/// Reject destructive operations that are more likely to be catastrophic than
/// intentional. The frontend asks for an explicit confirmation, but the backend
/// must enforce these guards as well.
fn ensure_safe_to_trash(path: &Path, root_id: &str, roots: &[RootFolder]) -> Result<()> {
    if path.parent().is_none() {
        return Err(refuse_delete(path, "the filesystem root cannot be deleted"));
    }

    if let Some(home) = home_directory() {
        if path == home {
            return Err(refuse_delete(
                path,
                "the user's home folder cannot be deleted",
            ));
        }
    }
    if path.parent() == Some(Path::new("/Users")) || path.parent() == Some(Path::new("/home")) {
        return Err(refuse_delete(path, "user home folders cannot be deleted"));
    }

    if let Some(parent) = path.parent() {
        if FORBIDDEN_MOUNT_ROOTS
            .iter()
            .any(|mount| parent == Path::new(mount))
        {
            return Err(refuse_delete(path, "mounted volumes cannot be deleted"));
        }
    }

    if FORBIDDEN_DELETE_PATHS
        .iter()
        .any(|forbidden| path == Path::new(forbidden))
    {
        return Err(refuse_delete(path, "this is a system folder"));
    }

    match path.file_name() {
        Some(name) if is_hidden_or_package(name) => {
            return Err(refuse_delete(
                path,
                "hidden folders and application/photo packages are protected",
            ));
        }
        Some(_) => {}
        None => return Err(refuse_delete(path, "the path has no folder name")),
    }

    for other in roots.iter().filter(|other| other.id != root_id) {
        let other_path = fs::canonicalize(&other.path).unwrap_or_else(|_| other.path.clone());
        if other_path.starts_with(path) {
            return Err(refuse_delete(
                path,
                "it contains another folder registered with Lumio",
            ));
        }
        if path.starts_with(&other_path) {
            return Err(refuse_delete(
                path,
                "it is inside another folder registered with Lumio",
            ));
        }
    }

    Ok(())
}

fn refuse_delete(path: &Path, reason: &str) -> LumioError {
    LumioError::InvalidPath(format!("Refusing to delete {}: {reason}.", path.display()))
}

fn home_directory() -> Option<PathBuf> {
    let home = std::env::var_os("HOME")
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)?;
    Some(fs::canonicalize(&home).unwrap_or(home))
}

fn move_to_trash(path: &Path) -> std::result::Result<(), trash::Error> {
    let mut context = trash::TrashContext::new();
    // Finder automation needs extra permissions and can be blocked by macOS.
    // NSFileManager moves the folder to Trash without prompting for automation
    // access and still remains recoverable through the Trash.
    #[cfg(target_os = "macos")]
    {
        use trash::macos::{DeleteMethod, TrashContextExtMacos};
        context.set_delete_method(DeleteMethod::NsFileManager);
    }
    context.delete(path)
}

fn save_atomic(path: &Path, config: &AppConfig) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|error| LumioError::io(parent, error))?;
    }
    let tmp = path.with_file_name(format!(
        ".{}.tmp-{}",
        path.file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_else(|| "settings.json".into()),
        Uuid::new_v4()
    ));
    let payload =
        serde_json::to_vec_pretty(config).map_err(|error| LumioError::Config(error.to_string()))?;
    {
        let mut file = fs::File::create(&tmp).map_err(|error| LumioError::io(&tmp, error))?;
        file.write_all(&payload)
            .map_err(|error| LumioError::io(&tmp, error))?;
        file.sync_all()
            .map_err(|error| LumioError::io(&tmp, error))?;
    }
    fs::rename(&tmp, path).map_err(|error| {
        let _ = fs::remove_file(&tmp);
        LumioError::io(path, error)
    })?;
    // Make the rename durable on the containing directory where possible.
    #[cfg(unix)]
    if let Some(parent) = path.parent() {
        if let Ok(dir) = fs::File::open(parent) {
            let _ = dir.sync_all();
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn config_round_trip_and_root_management() {
        let temp = tempfile::tempdir().unwrap();
        let config_path = temp.path().join("settings.json");
        let photos = temp.path().join("photos");
        fs::create_dir_all(photos.join("child")).unwrap();

        let mut store = ConfigStore::open(&config_path).unwrap();
        let root = store.add_root(&photos).unwrap().root;
        assert!(root.available);
        assert_eq!(store.roots().len(), 1);

        // Re-adding is a no-op.
        store.add_root(&photos).unwrap();
        assert_eq!(store.roots().len(), 1);

        // Nested roots are rejected.
        assert!(store.add_root(&photos.join("child")).is_err());

        let reopened = ConfigStore::open(&config_path).unwrap();
        assert_eq!(reopened.roots().len(), 1);
        assert_eq!(reopened.roots()[0].path, fs::canonicalize(&photos).unwrap());

        let mut reopened = reopened;
        assert!(
            reopened
                .remove_root(&root.id, RootRemovalMode::LibraryOnly)
                .unwrap()
        );
        assert!(reopened.roots().is_empty());
        // Library-only removal must not touch the folder on disk.
        assert!(photos.exists());
    }

    #[test]
    fn hidden_subfolder_round_trip_and_restore() {
        let temp = tempfile::tempdir().unwrap();
        let config_path = temp.path().join("settings.json");
        let photos = temp.path().join("photos");
        let child = photos.join("child");
        let outside = temp.path().join("outside");
        fs::create_dir_all(&child).unwrap();
        fs::create_dir_all(&outside).unwrap();

        let mut store = ConfigStore::open(&config_path).unwrap();
        let root = store.add_root(&photos).unwrap().root;
        assert!(store.excluded_folders().is_empty());

        // A folder outside every registered root cannot be hidden.
        assert!(
            store
                .remove_subfolder(&outside, RootRemovalMode::LibraryOnly)
                .is_err()
        );

        assert!(
            store
                .remove_subfolder(&child, RootRemovalMode::LibraryOnly)
                .unwrap()
        );
        let canonical_child = fs::canonicalize(&child).unwrap();
        assert_eq!(store.excluded_folders().len(), 1);
        assert_eq!(store.excluded_folders()[0], canonical_child);
        // Hiding must not touch the folder on disk.
        assert!(child.exists());

        let mut reopened = ConfigStore::open(&config_path).unwrap();
        assert_eq!(reopened.excluded_folders().len(), 1);
        assert_eq!(reopened.excluded_folders()[0], canonical_child);

        // Re-adding a hidden subfolder restores it without creating a new root.
        let outcome = reopened.add_root(&child).unwrap();
        assert!(outcome.restored);
        assert_eq!(outcome.root.id, root.id);
        assert_eq!(outcome.folder, canonical_child);
        assert!(reopened.excluded_folders().is_empty());
        assert_eq!(reopened.roots().len(), 1);
    }

    #[test]
    fn safe_delete_guards_reject_unsafe_paths() {
        assert!(ensure_safe_to_trash(Path::new("/"), "root", &[]).is_err());
        assert!(ensure_safe_to_trash(Path::new("/Users"), "root", &[]).is_err());
        assert!(ensure_safe_to_trash(Path::new("/Volumes/Camera"), "root", &[]).is_err());
        assert!(ensure_safe_to_trash(Path::new("/media/usb"), "root", &[]).is_err());
        assert!(ensure_safe_to_trash(Path::new("/mnt/disk"), "root", &[]).is_err());
        if let Some(home) = home_directory() {
            assert!(ensure_safe_to_trash(&home, "root", &[]).is_err());
        }

        let temp = tempfile::tempdir().unwrap();
        let normal = temp.path().join("photos");
        let hidden = temp.path().join(".hidden");
        let package = temp.path().join("Library.photoslibrary");
        fs::create_dir(&normal).unwrap();
        fs::create_dir(&hidden).unwrap();
        fs::create_dir(&package).unwrap();

        assert!(ensure_safe_to_trash(&normal, "root", &[]).is_ok());
        assert!(ensure_safe_to_trash(&hidden, "root", &[]).is_err());
        assert!(ensure_safe_to_trash(&package, "root", &[]).is_err());
    }

    #[test]
    fn safe_delete_guards_reject_overlapping_roots() {
        let temp = tempfile::tempdir().unwrap();
        let parent = temp.path().join("parent");
        let nested = parent.join("nested");
        fs::create_dir_all(&nested).unwrap();
        let parent = fs::canonicalize(parent).unwrap();
        let nested = fs::canonicalize(nested).unwrap();

        let other_parent = vec![RootFolder {
            id: "other".into(),
            path: parent.clone(),
            added_at: 0,
        }];
        assert!(ensure_safe_to_trash(&nested, "root", &other_parent).is_err());

        let other_nested = vec![RootFolder {
            id: "other".into(),
            path: nested.clone(),
            added_at: 0,
        }];
        assert!(ensure_safe_to_trash(&parent, "root", &other_nested).is_err());
    }

    #[test]
    fn subfolder_trash_resolution_respects_roots() {
        let temp = tempfile::tempdir().unwrap();
        let root_dir = temp.path().join("photos");
        let child = root_dir.join("child");
        let hidden = root_dir.join(".hidden");
        let outside = temp.path().join("outside");
        fs::create_dir_all(&child).unwrap();
        fs::create_dir_all(&hidden).unwrap();
        fs::create_dir_all(&outside).unwrap();

        let root_path = fs::canonicalize(&root_dir).unwrap();
        let roots = vec![RootFolder {
            id: "root".into(),
            path: root_path.clone(),
            added_at: 0,
        }];

        assert!(resolve_trashable_subfolder(&child, &roots).is_ok());
        assert!(resolve_trashable_subfolder(&hidden, &roots).is_err());
        assert!(resolve_trashable_subfolder(&root_path, &roots).is_err());
        assert!(resolve_trashable_subfolder(&outside, &roots).is_err());
    }

    #[test]
    fn typography_settings_round_trip() {
        let temp = tempfile::tempdir().unwrap();
        let config_path = temp.path().join("settings.json");

        let mut store = ConfigStore::open(&config_path).unwrap();
        let mut ui = store.config().ui.clone();
        ui.ui_font = "system".into();
        ui.mono_font = "system".into();
        ui.ui_font_size = 16;
        store.update_ui(ui.clone()).unwrap();

        let reopened = ConfigStore::open(&config_path).unwrap();
        assert_eq!(reopened.config().ui, ui);
    }

    #[test]
    fn legacy_ui_settings_receive_typography_defaults() {
        let temp = tempfile::tempdir().unwrap();
        let config_path = temp.path().join("settings.json");
        fs::write(
            &config_path,
            r#"{
                "version": 1,
                "roots": [],
                "ui": {
                    "theme": "dark",
                    "viewMode": "list",
                    "recursive": true,
                    "thumbnailSize": 240,
                    "libraryWidth": 300,
                    "infoWidth": 360,
                    "lastRootId": null,
                    "lastFolder": null,
                    "filmstripVisible": false
                }
            }"#,
        )
        .unwrap();

        let store = ConfigStore::open(&config_path).unwrap();
        assert!(store.recovery_warning().is_none());
        assert!(store.excluded_folders().is_empty());
        let ui = &store.config().ui;
        assert_eq!(ui.theme, "dark");
        assert_eq!(ui.view_mode, "list");
        assert_eq!(ui.ui_font, "avenir");
        assert_eq!(ui.mono_font, "jetbrains");
        assert_eq!(ui.ui_font_size, 13);
    }

    #[test]
    fn corrupt_config_is_preserved() {
        let temp = tempfile::tempdir().unwrap();
        let config_path = temp.path().join("settings.json");
        fs::write(&config_path, "{not json").unwrap();
        let store = ConfigStore::open(&config_path).unwrap();
        assert!(store.recovery_warning().is_some());
        assert!(store.roots().is_empty());
        let backups: Vec<_> = fs::read_dir(temp.path())
            .unwrap()
            .filter_map(|entry| entry.ok())
            .filter(|entry| {
                entry
                    .file_name()
                    .to_string_lossy()
                    .starts_with("settings.json.corrupt-")
            })
            .collect();
        assert_eq!(backups.len(), 1);
    }
}
