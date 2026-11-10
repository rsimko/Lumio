export type AssetKind = 'raw' | 'jpeg' | 'png' | 'tiff' | 'webp' | 'other';

export type PreviewSource =
  | 'original'
  | 'companionJpeg'
  | 'embeddedRaw'
  | 'placeholder';

export type ViewMode = 'grid' | 'masonry' | 'justified' | 'list' | 'filmstrip';

export type ThemeSetting = 'system' | 'light' | 'dark';

export type UiFont = 'avenir' | 'system';

export type MonoFont = 'jetbrains' | 'system';

export type RootRemovalMode = 'libraryOnly' | 'moveToTrash';

export type AssetRemovalMode = 'libraryOnly' | 'moveToTrash';

export interface ExcludedAsset {
  id: string;
  primaryPath: string;
  name: string;
  hiddenAt: number;
  paths: string[];
}

export interface AssetRemovalOutcome {
  removedAssets: number;
  hiddenAssets: ExcludedAsset[];
}

export interface MoveAssetsOutcome {
  destination: string;
  movedAssets: number;
  movedFiles: number;
  skippedAssets: number;
}

export interface FolderRemovalTarget {
  kind: 'root' | 'folder';
  rootId: string;
  name: string;
  path: string;
}

export interface FolderMenuTarget {
  path: string;
  rootId: string;
  name: string;
  isRoot: boolean;
  x: number;
  y: number;
}

export interface PhotoMenuTarget {
  id: string;
  name: string;
  count: number;
  x: number;
  y: number;
}

export interface NewFolderTarget {
  path: string;
  rootId: string;
  name: string;
}

export interface AddFolderOutcome {
  root: RootInfo;
  folder: string;
  restored: boolean;
}

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

export interface HistogramData {
  /** 256 counts, one per 8-bit red value. */
  red: number[];
  green: number[];
  blue: number[];
  /** Rec. 709 luminance counts. */
  luma: number[];
  pixels: number;
}

export interface UiSettings {
  theme: ThemeSetting;
  uiFont: UiFont;
  monoFont: MonoFont;
  uiFontSize: number;
  viewMode: ViewMode;
  recursive: boolean;
  thumbnailSize: number;
  libraryWidth: number;
  infoWidth: number;
  lastRootId: string | null;
  lastFolder: string | null;
  filmstripVisible: boolean;
  listShowAssociations: boolean;
  /** Only actions that differ from the defaults are stored. */
  shortcuts: Record<string, string>;
}

export interface Bootstrap {
  roots: RootInfo[];
  ui: UiSettings;
  cacheDir: string;
  recoveryWarning: string | null;
  appVersion: string;
  hiddenAssets: ExcludedAsset[];
}

export interface CommandError {
  code: string;
  message: string;
}
