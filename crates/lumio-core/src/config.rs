use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use uuid::Uuid;

use crate::error::{LumioError, Result};
use crate::path_utils::{canonical_dir, display_name, now_ms};
use crate::types::{AppConfig, RootFolder, RootInfo, UiSettings};

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

    pub fn add_root(&mut self, path: &Path) -> Result<RootInfo> {
        let canonical = canonical_dir(path)?;
        if let Some(existing) = self.config.roots.iter().find(|root| root.path == canonical) {
            return Ok(self
                .root_infos()
                .into_iter()
                .find(|root| root.id == existing.id)
                .expect("root must exist"));
        }

        // Refuse to add a folder that is already covered by a registered root.
        // The existing root already lets the user browse it.
        if let Some(existing) = self
            .config
            .roots
            .iter()
            .find(|root| canonical.starts_with(&root.path))
        {
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
        Ok(RootInfo {
            id: root.id,
            path: root.path,
            name: display_name(&canonical),
            available: true,
            error: None,
            added_at: root.added_at,
        })
    }

    pub fn remove_root(&mut self, id: &str) -> Result<bool> {
        let before = self.config.roots.len();
        self.config.roots.retain(|root| root.id != id);
        if self.config.roots.len() == before {
            return Ok(false);
        }
        if self.config.ui.last_root_id.as_deref() == Some(id) {
            self.config.ui.last_root_id = None;
            self.config.ui.last_folder = None;
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
        let root = store.add_root(&photos).unwrap();
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
        assert!(reopened.remove_root(&root.id).unwrap());
        assert!(reopened.roots().is_empty());
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
