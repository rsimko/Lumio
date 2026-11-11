import { Channel, invoke } from '@tauri-apps/api/core';

import type {
  AddFolderOutcome,
  AssetRemovalMode,
  AssetRemovalOutcome,
  Bootstrap,
  CameraInfo,
  CommandError,
  ExcludedAsset,
  FolderImageCounts,
  FolderNode,
  HistogramData,
  IngestConflictResolution,
  IngestEvent,
  IngestOutcome,
  IngestPlan,
  IngestRequest,
  IngestSettings,
  IngestTools,
  MetadataBatchOutcome,
  MetadataUpdateRequest,
  MoveAssetsOutcome,
  PhotoAsset,
  RootInfo,
  RootRemovalMode,
  ScanResult,
  ThumbnailResult,
  UiSettings,
} from './types';

export const api = {
  bootstrap: () => invoke<Bootstrap>('get_bootstrap'),
  addRoot: (path: string) => invoke<AddFolderOutcome>('add_root', { path }),
  createFolder: (parent: string, name: string) =>
    invoke<string>('create_folder', { parent, name }),
  removeRoot: (id: string, mode: RootRemovalMode) =>
    invoke<RootInfo[]>('remove_root', { id, mode }),
  removeSubfolder: (path: string, mode: RootRemovalMode) =>
    invoke<void>('remove_subfolder', { path, mode }),
  updateUiSettings: (ui: UiSettings) => invoke<UiSettings>('update_ui_settings', { ui }),
  listSubfolders: (folder: string) => invoke<FolderNode[]>('list_subfolders', { folder }),
  countFolderImages: (folder: string) =>
    invoke<FolderImageCounts>('count_folder_images', { folder }),
  scanFolder: (folder: string, recursive: boolean) =>
    invoke<ScanResult>('scan_folder', { folder, recursive }),
  updateMetadata: (requests: MetadataUpdateRequest[]) =>
    invoke<MetadataBatchOutcome>('update_metadata', { requests }),
  moveAssets: (assets: PhotoAsset[], destination: string) =>
    invoke<MoveAssetsOutcome>('move_assets', { assets, destination }),
  removeAssets: (assets: PhotoAsset[], mode: AssetRemovalMode) =>
    invoke<AssetRemovalOutcome>('remove_assets', { assets, mode }),
  restoreAssets: (ids: string[]) => invoke<ExcludedAsset[]>('restore_assets', { ids }),
  getHiddenAssets: () => invoke<ExcludedAsset[]>('get_hidden_assets'),
  thumbnail: (asset: PhotoAsset, edge: number, preferThumbnail = false) =>
    invoke<ThumbnailResult>('get_thumbnail', { asset, edge, preferThumbnail }),
  histogram: (asset: PhotoAsset, edge: number) =>
    invoke<HistogramData>('get_histogram', { asset, edge }),
  cameraInfo: (asset: PhotoAsset) => invoke<CameraInfo>('get_camera_info', { asset }),
  clearThumbnailCache: () => invoke<void>('clear_thumbnail_cache'),
  openAsset: (path: string) => invoke<void>('open_asset', { path }),
  editAssets: (paths: string[], application: string) =>
    invoke<void>('edit_assets', { paths, application }),
  revealAsset: (path: string) => invoke<void>('reveal_asset', { path }),
  updateIngestSettings: (ingest: IngestSettings) =>
    invoke<IngestSettings>('update_ingest_settings', { ingest }),
  detectIngestTools: (exiftoolPath: string | null) =>
    invoke<IngestTools>('detect_ingest_tools', { exiftoolPath }),
  createIngestPlan: (request: IngestRequest) =>
    invoke<IngestPlan>('create_ingest_plan', { request }),
  setIngestResolution: (
    planId: string,
    entityId: string,
    resolution: IngestConflictResolution,
  ) => invoke<IngestPlan>('set_ingest_resolution', { planId, entityId, resolution }),
  async runIngest(
    planId: string,
    onEvent: (event: IngestEvent) => void,
  ): Promise<IngestOutcome> {
    const channel = new Channel<IngestEvent>();
    channel.onmessage = onEvent;
    return invoke<IngestOutcome>('run_ingest', { planId, channel });
  },
  cancelIngest: (planId: string) => invoke<boolean>('cancel_ingest', { planId }),
  writeTextFile: (path: string, contents: string) =>
    invoke<void>('write_text_file', { path, contents }),
};

export function errorMessage(error: unknown): string {
  if (typeof error === 'string') return error;
  if (error && typeof error === 'object') {
    const candidate = error as Partial<CommandError> & { message?: unknown };
    if (typeof candidate.message === 'string' && candidate.message.trim()) {
      return candidate.message;
    }
  }
  if (error instanceof Error) return error.message;
  return String(error);
}

export function errorCode(error: unknown): string {
  if (error && typeof error === 'object') {
    const candidate = error as Partial<CommandError>;
    if (typeof candidate.code === 'string') return candidate.code;
  }
  return 'unknown';
}
