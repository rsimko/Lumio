use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, MutexGuard};

use lumio_core::path_utils::{canonical_existing, modified_ms};
use lumio_core::{ConfigStore, LumioError, ThumbnailService};
use tauri::Manager;

pub struct AppPaths {
    pub config_file: PathBuf,
    pub cache_dir: PathBuf,
}

impl AppPaths {
    pub fn resolve(app: &tauri::AppHandle) -> std::io::Result<Self> {
        let config_dir = app.path().app_config_dir().map_err(std::io::Error::other)?;
        let cache_dir = app
            .path()
            .app_cache_dir()
            .map_err(std::io::Error::other)?
            .join("thumbnails");
        std::fs::create_dir_all(&config_dir)?;
        std::fs::create_dir_all(&cache_dir)?;
        Ok(Self {
            config_file: config_dir.join("settings.json"),
            cache_dir,
        })
    }
}

pub struct AppState {
    pub config: Mutex<ConfigStore>,
    pub thumbs: Arc<ThumbnailService>,
    pub thumb_gate: Arc<tokio::sync::Semaphore>,
    pub inflight: Arc<Mutex<HashMap<String, Arc<tokio::sync::Mutex<()>>>>>,
    pub recovery_warning: Option<String>,
}

impl AppState {
    pub fn new(paths: AppPaths) -> Result<Self, String> {
        let config = ConfigStore::open(&paths.config_file).map_err(|error| error.to_string())?;
        let recovery_warning = config.recovery_warning().map(str::to_string);
        let thumbs = ThumbnailService::new(&paths.cache_dir).map_err(|error| error.to_string())?;
        Ok(Self {
            config: Mutex::new(config),
            thumbs: Arc::new(thumbs),
            thumb_gate: Arc::new(tokio::sync::Semaphore::new(4)),
            inflight: Arc::new(Mutex::new(HashMap::new())),
            recovery_warning,
        })
    }

    pub fn config(&self) -> MutexGuard<'_, ConfigStore> {
        self.config
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    pub fn root_paths(&self) -> Vec<PathBuf> {
        self.config().root_paths()
    }
}

pub fn validate_under_roots(state: &AppState, path: &Path) -> Result<PathBuf, LumioError> {
    let canonical = canonical_existing(path)?;
    let roots = state.root_paths();
    if roots.iter().any(|root| canonical.starts_with(root)) {
        Ok(canonical)
    } else {
        Err(LumioError::OutsideRoots(canonical))
    }
}

pub fn refresh_asset_metadata(asset: &mut lumio_core::PhotoAsset) -> Result<(), LumioError> {
    let metadata = std::fs::metadata(&asset.primary_path)
        .map_err(|error| LumioError::io(&asset.primary_path, error))?;
    asset.file_size = metadata.len();
    asset.modified_ms = modified_ms(&metadata);

    if let Some(companion) = asset.companion_jpeg.clone() {
        match std::fs::metadata(&companion) {
            Ok(metadata) if metadata.is_file() => {}
            _ => asset.companion_jpeg = None,
        }
    }
    Ok(())
}
