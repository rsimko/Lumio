use std::path::PathBuf;
use std::sync::Arc;

use lumio_core::LumioError;
use lumio_core::types::{
    AddFolderOutcome, FolderImageCounts, FolderNode, PhotoAsset, RootInfo, RootRemovalMode,
    ScanResult, ThumbnailResult, UiSettings,
};
use serde::Serialize;
use tauri::State;

use crate::state::{AppState, refresh_asset_metadata, validate_under_roots};

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CommandError {
    pub code: &'static str,
    pub message: String,
}

impl CommandError {
    fn new(code: &'static str, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }
}

impl From<LumioError> for CommandError {
    fn from(error: LumioError) -> Self {
        Self {
            code: error.code(),
            message: error.to_string(),
        }
    }
}

impl From<tauri::Error> for CommandError {
    fn from(error: tauri::Error) -> Self {
        Self::new("internal", error.to_string())
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Bootstrap {
    pub roots: Vec<RootInfo>,
    pub ui: UiSettings,
    pub cache_dir: String,
    pub recovery_warning: Option<String>,
    pub app_version: String,
}

#[tauri::command]
pub fn get_bootstrap(state: State<'_, AppState>) -> Bootstrap {
    let config = state.config();
    Bootstrap {
        roots: config.root_infos(),
        ui: config.config().ui.clone(),
        cache_dir: state.thumbs.cache_dir().to_string_lossy().into_owned(),
        recovery_warning: state.recovery_warning.clone(),
        app_version: env!("CARGO_PKG_VERSION").to_string(),
    }
}

#[tauri::command]
pub fn add_root(
    state: State<'_, AppState>,
    path: String,
) -> Result<AddFolderOutcome, CommandError> {
    let mut config = state.config();
    Ok(config.add_root(&PathBuf::from(path))?)
}

#[tauri::command]
pub fn create_folder(
    state: State<'_, AppState>,
    parent: String,
    name: String,
) -> Result<String, CommandError> {
    let parent = validate_under_roots(&state, &PathBuf::from(parent))?;
    let path = lumio_core::create_folder(&parent, &name)?;
    Ok(path.to_string_lossy().into_owned())
}

#[tauri::command]
pub fn remove_root(
    state: State<'_, AppState>,
    id: String,
    mode: RootRemovalMode,
) -> Result<Vec<RootInfo>, CommandError> {
    let mut config = state.config();
    config.remove_root(&id, mode)?;
    Ok(config.root_infos())
}

#[tauri::command]
pub fn remove_subfolder(
    state: State<'_, AppState>,
    path: String,
    mode: RootRemovalMode,
) -> Result<(), CommandError> {
    let mut config = state.config();
    config.remove_subfolder(&PathBuf::from(path), mode)?;
    Ok(())
}

#[tauri::command]
pub fn update_ui_settings(
    state: State<'_, AppState>,
    ui: UiSettings,
) -> Result<UiSettings, CommandError> {
    let mut ui = ui;
    ui.thumbnail_size = ui.thumbnail_size.clamp(120, 480);
    ui.library_width = ui.library_width.clamp(200, 480);
    ui.info_width = ui.info_width.clamp(240, 520);
    ui.ui_font_size = ui.ui_font_size.clamp(11, 17);
    if !matches!(ui.theme.as_str(), "system" | "light" | "dark") {
        ui.theme = "system".into();
    }
    if !matches!(ui.ui_font.as_str(), "avenir" | "system") {
        ui.ui_font = "avenir".into();
    }
    if !matches!(ui.mono_font.as_str(), "jetbrains" | "system") {
        ui.mono_font = "jetbrains".into();
    }
    if !matches!(
        ui.view_mode.as_str(),
        "grid" | "masonry" | "justified" | "list" | "filmstrip"
    ) {
        ui.view_mode = "grid".into();
    }
    let mut config = state.config();
    config.update_ui(ui.clone())?;
    Ok(ui)
}

#[tauri::command]
pub async fn list_subfolders(
    state: State<'_, AppState>,
    folder: String,
) -> Result<Vec<FolderNode>, CommandError> {
    let folder = validate_under_roots(&state, &PathBuf::from(folder))?;
    let excluded = state.config().excluded_folders().to_vec();
    match tokio::task::spawn_blocking(move || lumio_core::list_subfolders(&folder, &excluded)).await
    {
        Ok(result) => Ok(result?),
        Err(error) => Err(CommandError::new(
            "internal",
            format!("Folder listing task failed: {error}"),
        )),
    }
}

#[tauri::command]
pub async fn count_folder_images(
    state: State<'_, AppState>,
    folder: String,
) -> Result<FolderImageCounts, CommandError> {
    let folder = validate_under_roots(&state, &PathBuf::from(folder))?;
    let excluded = state.config().excluded_folders().to_vec();
    match tokio::task::spawn_blocking(move || lumio_core::count_folder_images(&folder, &excluded))
        .await
    {
        Ok(result) => Ok(result?),
        Err(error) => Err(CommandError::new(
            "internal",
            format!("Folder count task failed: {error}"),
        )),
    }
}

#[tauri::command]
pub async fn scan_folder(
    state: State<'_, AppState>,
    folder: String,
    recursive: bool,
) -> Result<ScanResult, CommandError> {
    let folder = validate_under_roots(&state, &PathBuf::from(folder))?;
    let excluded = state.config().excluded_folders().to_vec();
    match tokio::task::spawn_blocking(move || {
        lumio_core::scan_photos(&folder, recursive, &excluded)
    })
    .await
    {
        Ok(result) => Ok(result?),
        Err(error) => Err(CommandError::new(
            "internal",
            format!("Folder scan task failed: {error}"),
        )),
    }
}

#[tauri::command]
pub async fn get_thumbnail(
    state: State<'_, AppState>,
    mut asset: PhotoAsset,
    edge: u32,
    prefer_thumbnail: bool,
) -> Result<ThumbnailResult, CommandError> {
    // Never trust a path sent from the webview: re-canonicalize it and check
    // that it still lives inside one of the registered library roots.
    asset.primary_path = validate_under_roots(&state, &asset.primary_path)?;
    if let Some(companion) = asset.companion_jpeg.clone() {
        let primary_parent = asset.primary_path.parent().map(PathBuf::from);
        match validate_under_roots(&state, &companion) {
            Ok(canonical) if canonical.parent().map(PathBuf::from) == primary_parent => {
                asset.companion_jpeg = Some(canonical);
            }
            _ => asset.companion_jpeg = None,
        }
    }
    refresh_asset_metadata(&mut asset)?;

    let edge = edge.clamp(64, 4096);
    let thumbs = state.thumbs.clone();
    let gate = state.thumb_gate.clone();
    let inflight = state.inflight.clone();

    // Coalesce duplicate requests (grid re-renders, filmstrip and grid asking
    // for the same asset at the same edge) so a file is decoded only once.
    let key = format!("{}:{edge}", asset.id);
    let lock = {
        let mut map = inflight
            .lock()
            .map_err(|_| CommandError::new("internal", "Thumbnail queue poisoned"))?;
        map.entry(key)
            .or_insert_with(|| Arc::new(tokio::sync::Mutex::new(())))
            .clone()
    };

    let _guard = lock.lock().await;
    let permit = gate
        .acquire()
        .await
        .map_err(|_| CommandError::new("internal", "Thumbnail queue closed"))?;
    match tokio::task::spawn_blocking(move || thumbs.get(&asset, edge, prefer_thumbnail)).await {
        Ok(result) => {
            drop(permit);
            Ok(result?)
        }
        Err(error) => Err(CommandError::new(
            "internal",
            format!("Thumbnail task failed: {error}"),
        )),
    }
}

#[tauri::command]
pub fn clear_thumbnail_cache(state: State<'_, AppState>) -> Result<(), CommandError> {
    state.thumbs.clear()?;
    Ok(())
}

#[tauri::command]
pub async fn open_asset(state: State<'_, AppState>, path: String) -> Result<(), CommandError> {
    let path = validate_under_roots(&state, &PathBuf::from(path))?;
    tauri::async_runtime::spawn_blocking(move || {
        tauri_plugin_opener::open_path(&path, None::<&str>)
    })
    .await
    .map_err(|error| CommandError::new("internal", format!("Open task failed: {error}")))?
    .map_err(|error| CommandError::new("openFailed", error.to_string()))
}

#[tauri::command]
pub async fn reveal_asset(state: State<'_, AppState>, path: String) -> Result<(), CommandError> {
    let path = validate_under_roots(&state, &PathBuf::from(path))?;
    tauri::async_runtime::spawn_blocking(move || tauri_plugin_opener::reveal_item_in_dir(&path))
        .await
        .map_err(|error| CommandError::new("internal", format!("Reveal task failed: {error}")))?
        .map_err(|error| CommandError::new("revealFailed", error.to_string()))
}
