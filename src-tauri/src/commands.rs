use std::path::PathBuf;
use std::sync::Arc;

use lumio_core::Histogram;
use lumio_core::LumioError;
use lumio_core::ingest::{
    IngestConflictResolution, IngestEvent, IngestOutcome, IngestPlan, IngestRequest,
    IngestSettings, IngestTools,
};
use lumio_core::metadata::{MetadataSource, MetadataUpdate, MetadataWriteResult, PhotoMetadata};
use lumio_core::types::{
    AddFolderOutcome, AssetRemovalMode, AssetRemovalOutcome, ExcludedAsset, FolderImageCounts,
    FolderNode, MoveAssetsOutcome, PhotoAsset, RootInfo, RootRemovalMode, ScanResult,
    ThumbnailResult, UiSettings,
};
use serde::{Deserialize, Serialize};
use tauri::State;
use tauri::ipc::Channel;

use crate::ingest::IngestRuntime;
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
    pub ingest: IngestSettings,
    pub cache_dir: String,
    pub recovery_warning: Option<String>,
    pub app_version: String,
    pub hidden_assets: Vec<ExcludedAsset>,
}

#[tauri::command]
pub fn get_bootstrap(state: State<'_, AppState>) -> Bootstrap {
    let config = state.config();
    Bootstrap {
        roots: config.root_infos(),
        ui: config.config().ui.clone(),
        ingest: config.ingest_settings().clone(),
        cache_dir: state.thumbs.cache_dir().to_string_lossy().into_owned(),
        recovery_warning: state.recovery_warning.clone(),
        app_version: env!("CARGO_PKG_VERSION").to_string(),
        hidden_assets: config.excluded_assets().to_vec(),
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
    let excluded_files = state.config().excluded_asset_paths();
    match tokio::task::spawn_blocking(move || {
        lumio_core::list_subfolders_excluding(&folder, &excluded, &excluded_files)
    })
    .await
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
    let excluded_files = state.config().excluded_asset_paths();
    match tokio::task::spawn_blocking(move || {
        lumio_core::count_folder_images_excluding(&folder, &excluded, &excluded_files)
    })
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
    let excluded_files = state.config().excluded_asset_paths();
    match tokio::task::spawn_blocking(move || {
        lumio_core::scan_photos_excluding(&folder, recursive, &excluded, &excluded_files)
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

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MetadataUpdateRequest {
    pub asset: PhotoAsset,
    pub expected_revision: String,
    #[serde(default)]
    pub update: MetadataUpdate,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MetadataUpdateSuccess {
    pub asset_id: String,
    pub metadata: PhotoMetadata,
    pub target: PathBuf,
    pub xmp_sidecar: Option<PathBuf>,
    pub created_sidecar: bool,
    pub file_size: u64,
    pub modified_ms: u64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MetadataUpdateFailure {
    pub asset_id: String,
    pub file_name: String,
    pub code: String,
    pub message: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MetadataBatchOutcome {
    pub updated: Vec<MetadataUpdateSuccess>,
    pub failed: Vec<MetadataUpdateFailure>,
}

fn validate_sidecar_path(
    state: &AppState,
    primary: &std::path::Path,
    sidecar: Option<&std::path::Path>,
) -> Result<Option<PathBuf>, CommandError> {
    let Some(sidecar) = sidecar else {
        return Ok(None);
    };
    let Some(file_name) = sidecar.file_name() else {
        return Err(CommandError::new(
            "invalidPath",
            "The XMP sidecar path has no file name.",
        ));
    };
    let parent = sidecar.parent().ok_or_else(|| {
        CommandError::new("invalidPath", "The XMP sidecar path has no parent folder.")
    })?;
    let parent = validate_under_roots(state, parent)?;
    let primary_parent = primary.parent().unwrap_or(primary);
    if parent != primary_parent {
        return Err(CommandError::new(
            "invalidPath",
            "The XMP sidecar must live beside its photo.",
        ));
    }
    let candidate = parent.join(file_name);
    if candidate.exists() {
        Ok(Some(validate_under_roots(state, &candidate)?))
    } else {
        Ok(Some(candidate))
    }
}

#[tauri::command]
pub async fn update_metadata(
    state: State<'_, AppState>,
    requests: Vec<MetadataUpdateRequest>,
) -> Result<MetadataBatchOutcome, CommandError> {
    if requests.is_empty() {
        return Ok(MetadataBatchOutcome {
            updated: Vec::new(),
            failed: Vec::new(),
        });
    }

    let ingest = state.config().ingest_settings().clone();
    let Some(tool) = lumio_core::ingest::discover_exiftool(ingest.exiftool_path.as_deref()) else {
        return Err(CommandError::new(
            "exiftoolMissing",
            "ExifTool is required to edit photo metadata. Install it or set its path in Settings → Ingest.",
        ));
    };

    // Validate every path before writing anything so a bad entry cannot leave
    // a batch partially applied for an avoidable reason.
    let mut prepared = Vec::new();
    for request in requests {
        let primary = validate_under_roots(&state, &request.asset.primary_path)?;
        let sidecar =
            validate_sidecar_path(&state, &primary, request.asset.xmp_sidecar.as_deref())?;
        prepared.push((request, primary, sidecar));
    }

    let config = state.exiftool_config_file.clone();
    let gate = &state.metadata_gate;
    let _guard = gate.lock().await;
    let outcome = tokio::task::spawn_blocking(move || {
        let result = |request: &MetadataUpdateRequest,
                      primary: &std::path::Path,
                      sidecar: Option<&std::path::Path>|
         -> std::result::Result<MetadataWriteResult, LumioError> {
            lumio_core::metadata::write_photo_metadata(
                primary,
                sidecar,
                &request.update,
                Some(request.expected_revision.as_str()),
                &tool.path,
                Some(&config),
            )
        };

        let mut updated = Vec::new();
        let mut failed = Vec::new();
        for (request, primary, sidecar) in prepared {
            match result(&request, &primary, sidecar.as_deref()) {
                Ok(write) => {
                    let xmp_sidecar = if write.metadata.source == MetadataSource::Sidecar {
                        Some(write.target.clone())
                    } else {
                        None
                    };
                    updated.push(MetadataUpdateSuccess {
                        asset_id: request.asset.id,
                        metadata: write.metadata,
                        target: write.target,
                        xmp_sidecar,
                        created_sidecar: write.created_sidecar,
                        file_size: write.file_size,
                        modified_ms: write.modified_ms,
                    });
                }
                Err(error) => failed.push(MetadataUpdateFailure {
                    asset_id: request.asset.id,
                    file_name: request.asset.file_name,
                    code: error.code().to_string(),
                    message: error.to_string(),
                }),
            }
        }
        MetadataBatchOutcome { updated, failed }
    })
    .await
    .map_err(|error| CommandError::new("internal", format!("Metadata task failed: {error}")))?;

    Ok(outcome)
}

#[tauri::command]
pub fn get_hidden_assets(state: State<'_, AppState>) -> Vec<ExcludedAsset> {
    state.config().excluded_assets().to_vec()
}

#[tauri::command]
pub async fn move_assets(
    state: State<'_, AppState>,
    assets: Vec<PhotoAsset>,
    destination: String,
) -> Result<MoveAssetsOutcome, CommandError> {
    let destination = validate_under_roots(&state, &PathBuf::from(destination))?;
    let roots = state.config().roots().to_vec();
    match tokio::task::spawn_blocking(move || {
        lumio_core::move_assets(&assets, &destination, &roots)
    })
    .await
    {
        Ok(result) => Ok(result?),
        Err(error) => Err(CommandError::new(
            "internal",
            format!("Photo move task failed: {error}"),
        )),
    }
}

#[tauri::command]
pub async fn remove_assets(
    state: State<'_, AppState>,
    assets: Vec<PhotoAsset>,
    mode: AssetRemovalMode,
) -> Result<AssetRemovalOutcome, CommandError> {
    if mode == AssetRemovalMode::MoveToTrash {
        let roots = state.config().roots().to_vec();
        let primary_paths: Vec<PathBuf> = assets
            .iter()
            .map(|asset| asset.primary_path.clone())
            .collect();
        let outcome =
            match tokio::task::spawn_blocking(move || lumio_core::trash_assets(&assets, &roots))
                .await
            {
                Ok(result) => result?,
                Err(error) => {
                    return Err(CommandError::new(
                        "internal",
                        format!("Photo trash task failed: {error}"),
                    ));
                }
            };
        let mut config = state.config();
        config.forget_asset_exclusions(&primary_paths)?;
        return Ok(AssetRemovalOutcome {
            removed_assets: outcome.trashed_assets,
            hidden_assets: config.excluded_assets().to_vec(),
        });
    }

    let mut config = state.config();
    let removed_assets = config.hide_assets(&assets)? as u32;
    Ok(AssetRemovalOutcome {
        removed_assets,
        hidden_assets: config.excluded_assets().to_vec(),
    })
}

#[tauri::command]
pub fn restore_assets(
    state: State<'_, AppState>,
    ids: Vec<String>,
) -> Result<Vec<ExcludedAsset>, CommandError> {
    let mut config = state.config();
    config.restore_assets(&ids)?;
    Ok(config.excluded_assets().to_vec())
}

/// Re-canonicalizes an asset sent by the webview, drops unsafe companion
/// paths, and refreshes the metadata used to derive thumbnail cache keys.
fn prepare_asset(state: &AppState, mut asset: PhotoAsset) -> Result<PhotoAsset, CommandError> {
    asset.primary_path = validate_under_roots(state, &asset.primary_path)?;
    if let Some(companion) = asset.companion_jpeg.clone() {
        let primary_parent = asset.primary_path.parent().map(PathBuf::from);
        match validate_under_roots(state, &companion) {
            Ok(canonical) if canonical.parent().map(PathBuf::from) == primary_parent => {
                asset.companion_jpeg = Some(canonical);
            }
            _ => asset.companion_jpeg = None,
        }
    }
    refresh_asset_metadata(&mut asset)?;
    Ok(asset)
}

#[tauri::command]
pub async fn get_thumbnail(
    state: State<'_, AppState>,
    asset: PhotoAsset,
    edge: u32,
    prefer_thumbnail: bool,
) -> Result<ThumbnailResult, CommandError> {
    // Never trust a path sent from the webview: re-canonicalize it and check
    // that it still lives inside one of the registered library roots.
    let asset = prepare_asset(&state, asset)?;

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
pub async fn get_histogram(
    state: State<'_, AppState>,
    asset: PhotoAsset,
    edge: u32,
) -> Result<Histogram, CommandError> {
    let asset = prepare_asset(&state, asset)?;
    let edge = edge.clamp(64, 4096);
    let thumbs = state.thumbs.clone();
    let gate = state.thumb_gate.clone();
    let inflight = state.inflight.clone();

    // Share the thumbnail queue so a histogram requested while the preview is
    // still decoding waits for that decode instead of repeating it.
    let key = format!("{}:{edge}", asset.id);
    let lock = {
        let mut map = inflight
            .lock()
            .map_err(|_| CommandError::new("internal", "Histogram queue poisoned"))?;
        map.entry(key)
            .or_insert_with(|| Arc::new(tokio::sync::Mutex::new(())))
            .clone()
    };

    let _guard = lock.lock().await;
    let permit = gate
        .acquire()
        .await
        .map_err(|_| CommandError::new("internal", "Thumbnail queue closed"))?;
    match tokio::task::spawn_blocking(move || thumbs.histogram(&asset, edge)).await {
        Ok(result) => {
            drop(permit);
            Ok(result?)
        }
        Err(error) => Err(CommandError::new(
            "internal",
            format!("Histogram task failed: {error}"),
        )),
    }
}

#[tauri::command]
pub async fn get_camera_info(
    state: State<'_, AppState>,
    asset: PhotoAsset,
) -> Result<lumio_core::CameraInfo, CommandError> {
    let asset = prepare_asset(&state, asset)?;
    match tokio::task::spawn_blocking(move || {
        lumio_core::read_camera_info(&asset.primary_path, asset.companion_jpeg.as_deref())
    })
    .await
    {
        Ok(info) => Ok(info),
        Err(error) => Err(CommandError::new(
            "internal",
            format!("Camera metadata task failed: {error}"),
        )),
    }
}

#[tauri::command]
pub fn clear_thumbnail_cache(state: State<'_, AppState>) -> Result<(), CommandError> {
    state.thumbs.clear()?;
    Ok(())
}

#[tauri::command]
pub async fn edit_assets(
    state: State<'_, AppState>,
    paths: Vec<String>,
    application: String,
) -> Result<(), CommandError> {
    if paths.is_empty() {
        return Err(CommandError::new("editFailed", "No photos were selected."));
    }
    let files = paths
        .iter()
        .map(|path| validate_under_roots(&state, &PathBuf::from(path)))
        .collect::<Result<Vec<_>, _>>()?;
    let application = PathBuf::from(application);
    if !application.exists() {
        return Err(CommandError::new(
            "editorMissing",
            "The selected editor is no longer installed.",
        ));
    }
    tauri::async_runtime::spawn_blocking(move || launch_editor(&application, &files))
        .await
        .map_err(|error| CommandError::new("internal", format!("Edit task failed: {error}")))?
        .map_err(|error| CommandError::new("editFailed", error.to_string()))
}

fn launch_editor(application: &std::path::Path, paths: &[PathBuf]) -> std::io::Result<()> {
    #[cfg(target_os = "macos")]
    {
        let status = std::process::Command::new("/usr/bin/open")
            .arg("-a")
            .arg(application)
            .args(paths)
            .status()?;
        if status.success() {
            return Ok(());
        }
        return Err(std::io::Error::other(format!(
            "The application could not open the selected photos ({status})"
        )));
    }
    #[cfg(not(target_os = "macos"))]
    {
        std::process::Command::new(application)
            .args(paths)
            .spawn()
            .map(|_| ())
    }
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

#[tauri::command]
pub fn update_ingest_settings(
    state: State<'_, AppState>,
    ingest: IngestSettings,
) -> Result<IngestSettings, CommandError> {
    let mut ingest = ingest;
    ingest.counter_start = ingest.counter_start.min(1_000_000);
    ingest.counter_padding = ingest.counter_padding.min(12);
    ingest.default_source = ingest.default_source.and_then(|path| {
        let trimmed = path.to_string_lossy().trim().to_string();
        (!trimmed.is_empty()).then(|| PathBuf::from(trimmed))
    });
    ingest.rename_postfix = lumio_core::ingest::normalize_rename_postfix(&ingest.rename_postfix);
    ingest.folder_template = ingest.folder_template.trim().to_string();
    ingest.filename_template = ingest.filename_template.trim().to_string();
    for template in &mut ingest.templates {
        template.name = template.name.trim().to_string();
        template.folder_template = template.folder_template.trim().to_string();
        template.filename_template = template.filename_template.trim().to_string();
    }
    ingest
        .templates
        .retain(|template| !template.name.is_empty());
    let mut config = state.config();
    config.update_ingest(ingest.clone())?;
    Ok(ingest)
}

#[tauri::command]
pub fn detect_ingest_tools(exiftool_path: Option<String>) -> IngestTools {
    lumio_core::ingest::detect_tools(exiftool_path.as_deref().map(PathBuf::from).as_deref())
}

#[tauri::command]
pub async fn create_ingest_plan(
    runtime: State<'_, IngestRuntime>,
    request: IngestRequest,
) -> Result<IngestPlan, CommandError> {
    match tokio::task::spawn_blocking(move || lumio_core::ingest::plan_ingest(&request)).await {
        Ok(result) => {
            let plan = result?;
            runtime.store(plan.clone());
            Ok(plan)
        }
        Err(error) => Err(CommandError::new(
            "internal",
            format!("Ingest planning task failed: {error}"),
        )),
    }
}

#[tauri::command]
pub fn set_ingest_resolution(
    runtime: State<'_, IngestRuntime>,
    plan_id: String,
    entity_id: String,
    resolution: IngestConflictResolution,
) -> Result<IngestPlan, CommandError> {
    let mut plan = runtime.get(&plan_id).ok_or_else(|| {
        CommandError::new(
            "ingestPlanMissing",
            "The ingest preview is no longer available. Run Dry Run again.",
        )
    })?;
    lumio_core::ingest::set_entity_resolution(&mut plan, &entity_id, resolution)?;
    runtime.update(plan.clone());
    Ok(plan)
}

#[tauri::command]
pub async fn run_ingest(
    runtime: State<'_, IngestRuntime>,
    plan_id: String,
    channel: Channel<IngestEvent>,
) -> Result<IngestOutcome, CommandError> {
    let Some(plan) = runtime.begin(&plan_id) else {
        return Err(CommandError::new(
            "ingestPlanMissing",
            "The ingest preview is no longer available, or it is already running.",
        ));
    };
    let Some(cancel) = runtime.cancel_token(&plan_id) else {
        runtime.finish(&plan_id);
        return Err(CommandError::new(
            "internal",
            "The ingest cancellation token was not created.",
        ));
    };
    let task_id = plan.id.clone();
    let result = tokio::task::spawn_blocking(move || {
        lumio_core::ingest::execute_plan(&plan, &cancel, |event| {
            let _ = channel.send(event);
        })
    })
    .await;
    runtime.finish(&task_id);
    match result {
        Ok(outcome) => Ok(outcome),
        Err(error) => Err(CommandError::new(
            "internal",
            format!("Ingest task failed: {error}"),
        )),
    }
}

#[tauri::command]
pub fn cancel_ingest(runtime: State<'_, IngestRuntime>, plan_id: String) -> bool {
    runtime.cancel(&plan_id)
}

#[tauri::command]
pub fn write_text_file(path: String, contents: String) -> Result<(), CommandError> {
    let path = PathBuf::from(path);
    if let Some(parent) = path.parent() {
        if !parent.exists() {
            return Err(CommandError::new(
                "invalidPath",
                format!("The report folder does not exist: {}", parent.display()),
            ));
        }
    }
    std::fs::write(&path, contents).map_err(|error| LumioError::io(&path, error))?;
    Ok(())
}
