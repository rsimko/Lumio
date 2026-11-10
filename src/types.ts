export type AssetKind = 'raw' | 'jpeg' | 'png' | 'tiff' | 'webp' | 'other';

export type PreviewSource =
  | 'original'
  | 'companionJpeg'
  | 'embeddedRaw'
  | 'placeholder';

export type ViewMode = 'grid' | 'masonry' | 'justified' | 'list' | 'filmstrip';

export type ThemeSetting = 'system' | 'light' | 'dark';

export interface RootInfo {
  id: string;
  path: string;
  name: string;
  available: boolean;
  error: string | null;
  addedAt: number;
}

export interface FolderNode {
  name: string;
  path: string;
  hasChildren: boolean;
  accessible: boolean;
  error: string | null;
  imageCount: number;
  recursiveImageCount: number;
}

export interface FolderImageCounts {
  direct: number;
  recursive: number;
}

export interface ScanWarning {
  path: string;
  message: string;
}

export interface PhotoAsset {
  id: string;
  primaryPath: string;
  fileName: string;
  stem: string;
  extension: string;
  kind: AssetKind;
  companionJpeg: string | null;
  xmpSidecar: string | null;
  extraSidecars: string[];
  extraCompanions: string[];
  previewHint: PreviewSource;
  fileSize: number;
  modifiedMs: number;
  width: number | null;
  height: number | null;
  relativeDir: string;
}

export interface ScanResult {
  folder: string;
  recursive: boolean;
  assets: PhotoAsset[];
  warnings: ScanWarning[];
  elapsedMs: number;
}

export interface ThumbnailResult {
  cachePath: string | null;
  width: number | null;
  height: number | null;
  source: PreviewSource;
  edge: number;
  error: string | null;
}

export interface UiSettings {
  theme: ThemeSetting;
  viewMode: ViewMode;
  recursive: boolean;
  thumbnailSize: number;
  libraryWidth: number;
  infoWidth: number;
  lastRootId: string | null;
  lastFolder: string | null;
  filmstripVisible: boolean;
}

export interface Bootstrap {
  roots: RootInfo[];
  ui: UiSettings;
  cacheDir: string;
  recoveryWarning: string | null;
  appVersion: string;
}

export interface CommandError {
  code: string;
  message: string;
}
