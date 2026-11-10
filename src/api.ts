import { invoke } from '@tauri-apps/api/core';

import type {
  AddFolderOutcome,
  AssetRemovalMode,
  AssetRemovalOutcome,
  Bootstrap,
  CommandError,
  ExcludedAsset,
  FolderImageCounts,
  FolderNode,
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
  moveAssets: (assets: PhotoAsset[], destination: string) =>
    invoke<MoveAssetsOutcome>('move_assets', { assets, destination }),
  removeAssets: (assets: PhotoAsset[], mode: AssetRemovalMode) =>
    invoke<AssetRemovalOutcome>('remove_assets', { assets, mode }),
  restoreAssets: (ids: string[]) => invoke<ExcludedAsset[]>('restore_assets', { ids }),
  getHiddenAssets: () => invoke<ExcludedAsset[]>('get_hidden_assets'),
  thumbnail: (asset: PhotoAsset, edge: number, preferThumbnail = false) =>
    invoke<ThumbnailResult>('get_thumbnail', { asset, edge, preferThumbnail }),
  clearThumbnailCache: () => invoke<void>('clear_thumbnail_cache'),
  openAsset: (path: string) => invoke<void>('open_asset', { path }),
  revealAsset: (path: string) => invoke<void>('reveal_asset', { path }),
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
