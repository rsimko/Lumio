export type AssetKind = 'raw' | 'jpeg' | 'png' | 'tiff' | 'webp' | 'other';

export type PreviewSource =
  | 'original'
  | 'companionJpeg'
  | 'embeddedRaw'
  | 'placeholder';

export type ViewMode = 'grid' | 'masonry' | 'justified' | 'list' | 'filmstrip';

export type ThemeSetting = 'system' | 'light' | 'dark';

/**
 * Preset token (`avenir` or `system`) or any installed font family.
 * The Rust settings layer applies the same validation and fallback defaults.
 */
export type UiFont = string;

/** Preset token (`jetbrains` or `system`) or any installed font family. */
export type MonoFont = string;

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
  alreadyInLibrary: boolean;
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

export type ColorLabel = 'red' | 'yellow' | 'green' | 'blue' | 'purple';

export type CullFlag = 'pick' | 'reject';

export type MetadataSource = 'sidecar' | 'embedded' | 'none';

export interface MetadataIssue {
  code: string;
  message: string;
}

export interface PhotoMetadata {
  rating: number;
  label: ColorLabel | null;
  cull: CullFlag | null;
  favorite: boolean;
  keywords: string[];
  hierarchicalKeywords: string[];
  source: MetadataSource;
  revision: string;
  issues: MetadataIssue[];
}

export interface MetadataUpdate {
  rating?: number;
  label?: ColorLabel;
  clearLabel?: boolean;
  cull?: CullFlag;
  clearCull?: boolean;
  favorite?: boolean;
  keywords?: string[];
  hierarchicalKeywords?: string[];
}

export interface MetadataUpdateRequest {
  asset: PhotoAsset;
  expectedRevision: string;
  update: MetadataUpdate;
}

export interface MetadataUpdateSuccess {
  assetId: string;
  metadata: PhotoMetadata;
  target: string;
  xmpSidecar: string | null;
  createdSidecar: boolean;
  fileSize: number;
  modifiedMs: number;
}

export interface MetadataUpdateFailure {
  assetId: string;
  fileName: string;
  code: string;
  message: string;
}

export interface MetadataBatchOutcome {
  updated: MetadataUpdateSuccess[];
  failed: MetadataUpdateFailure[];
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
  metadata: PhotoMetadata;
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

export interface CameraInfo {
  make: string | null;
  model: string | null;
  lens: string | null;
  /** Exposure time in seconds. */
  shutterSpeed: number | null;
  /** Aperture f-number. */
  aperture: number | null;
  iso: number | null;
  /** Focal length in millimetres. */
  focalLength: number | null;
}

export interface HistogramData {
  /** 256 counts, one per 8-bit red value. */
  red: number[];
  green: number[];
  blue: number[];
  /** Rec. 709 luminance counts. */
  luma: number[];
  pixels: number;
  /** True when the counts come from RAW sensor samples. */
  raw: boolean;
}

export interface ExternalEditor {
  /** Display name taken from the application bundle. */
  name: string;
  /** Absolute path to the application bundle. */
  path: string;
}

export interface UiSettings {
  theme: ThemeSetting;
  uiFont: UiFont;
  monoFont: MonoFont;
  uiFontSize: number;
  viewMode: ViewMode;
  recursive: boolean;
  thumbnailSize: number;
  /** Thumbnail corner radius in pixels; zero gives sharp corners. */
  thumbnailRadius: number;
  libraryWidth: number;
  infoWidth: number;
  lastRootId: string | null;
  lastFolder: string | null;
  filmstripVisible: boolean;
  /** Height of the bottom filmstrip in pixels. */
  filmstripHeight: number;
  listShowAssociations: boolean;
  /** Applications offered by “Edit with” in the photo context menu. */
  externalEditors: ExternalEditor[];
  /** Path of the editor used by the Edit shortcut. */
  defaultEditorPath: string | null;
  /** Only actions that differ from the defaults are stored. */
  shortcuts: Record<string, string>;
}

export interface Bootstrap {
  roots: RootInfo[];
  ui: UiSettings;
  ingest: IngestSettings;
  cacheDir: string;
  recoveryWarning: string | null;
  appVersion: string;
  hiddenAssets: ExcludedAsset[];
}

export interface CommandError {
  code: string;
  message: string;
}

export type XmpNaming = 'filenameExt' | 'filename';

export interface IngestTemplate {
  id: string;
  name: string;
  folderTemplate: string;
  filenameTemplate: string;
  createdAt: number;
  updatedAt: number;
}

export interface IptcTemplates {
  title: string;
  description: string;
  creator: string;
  copyright: string;
  keywords: string[];
}

export interface IngestSettings {
  defaultSource: string | null;
  folderTemplate: string;
  filenameTemplate: string;
  jobCode: string;
  customText: string;
  counterStart: number;
  counterPadding: number;
  xmpNaming: XmpNaming;
  renamePostfix: string;
  exiftoolPath: string | null;
  addToLibrary: boolean;
  templates: IngestTemplate[];
  iptc: IptcTemplates;
}

export interface IngestRules {
  folderTemplate: string;
  filenameTemplate: string;
  jobCode: string;
  customText: string;
  counterStart: number;
  counterPadding: number;
  xmpNaming: XmpNaming;
  renamePostfix: string;
  iptc: IptcTemplates;
}

export interface IngestRequest {
  sources: string[];
  destination: string;
  rules: IngestRules;
  exiftoolPath: string | null;
}

export interface ExifToolInfo {
  path: string;
  version: string;
}

export interface IngestTools {
  exiftool: ExifToolInfo | null;
}

export type IngestFileRole = 'raw' | 'jpeg' | 'xmp';

export type CaptureSource = 'rawExif' | 'jpegExif' | 'xmp' | 'fileModified';

export interface CaptureInfo {
  display: string;
  timestampMs: number;
  source: CaptureSource;
  warning: string | null;
}

export interface IngestSourceFile {
  path: string;
  role: IngestFileRole;
  size: number;
  modifiedMs: number;
}

export type IngestConflictResolution = 'skip' | 'keepBoth' | 'replace';

export type IngestEntityStatus = 'ready' | 'conflict' | 'error';

export type IngestMetadataTargetKind = 'xmpSidecar' | 'jpeg';

export interface IngestMetadataTarget {
  path: string;
  kind: IngestMetadataTargetKind;
}

export interface IngestPlannedFile {
  source: string | null;
  destination: string;
  role: IngestFileRole;
  size: number;
  primary: boolean;
  generated: boolean;
}

export interface IngestConflict {
  existing: string[];
  message: string;
}

export interface IngestIptcValues {
  title: string | null;
  description: string | null;
  creator: string | null;
  copyright: string | null;
  keywords: string[];
}

export interface IngestEntity {
  id: string;
  sourceDir: string;
  stem: string;
  displayName: string;
  primaryPath: string;
  primaryRole: IngestFileRole;
  files: IngestSourceFile[];
  capture: CaptureInfo;
  baseDestinationDir: string;
  baseDestinationStem: string;
  destinationDir: string;
  destinationStem: string;
  plannedFiles: IngestPlannedFile[];
  metadataTarget: IngestMetadataTarget | null;
  status: IngestEntityStatus;
  resolution: IngestConflictResolution;
  conflict: IngestConflict | null;
  renamed: boolean;
  renameReason: string | null;
  warnings: string[];
  iptc: IngestIptcValues;
}

export interface IngestPlanSummary {
  entities: number;
  files: number;
  bytes: number;
  conflicts: number;
  renamed: number;
  warnings: number;
  errors: number;
}

export interface IngestWarning {
  path: string | null;
  entityId: string | null;
  message: string;
}

export type IngestLogLevel = 'info' | 'warning' | 'error';

export interface IngestLogEntry {
  at: string;
  level: IngestLogLevel;
  message: string;
  path: string | null;
  entityId: string | null;
}

export interface IngestPlan {
  id: string;
  createdAt: number;
  sources: string[];
  destination: string;
  rules: IngestRules;
  /** Resolved ExifTool executable used by this plan. */
  exiftoolPath: string | null;
  /** ExifTool setting at Dry Run time; null means auto-detect. */
  requestedExiftoolPath: string | null;
  tools: IngestTools;
  entities: IngestEntity[];
  warnings: IngestWarning[];
  log: IngestLogEntry[];
  summary: IngestPlanSummary;
}

export type IngestResultStatus = 'copied' | 'skipped' | 'failed' | 'cancelled';

export interface IngestEntityResult {
  entityId: string;
  displayName: string;
  status: IngestResultStatus;
  files: number;
  bytes: number;
  destination: string | null;
  message: string | null;
}

export interface IngestOutcome {
  planId: string;
  destination: string;
  copiedEntities: number;
  skippedEntities: number;
  failedEntities: number;
  copiedFiles: number;
  copiedBytes: number;
  cancelled: boolean;
  elapsedMs: number;
  results: IngestEntityResult[];
  log: IngestLogEntry[];
}

export type IngestEvent =
  | {
      type: 'started';
      totalEntities: number;
      totalFiles: number;
      totalBytes: number;
    }
  | {
      type: 'entityStarted';
      entityId: string;
      displayName: string;
      index: number;
      total: number;
    }
  | {
      type: 'fileCopied';
      entityId: string;
      source: string;
      destination: string;
      bytes: number;
      index: number;
      total: number;
    }
  | {
      type: 'entityCompleted';
      entityId: string;
      displayName: string;
      destination: string;
      files: number;
      bytes: number;
    }
  | {
      type: 'entitySkipped';
      entityId: string;
      displayName: string;
      reason: string;
    }
  | {
      type: 'entityFailed';
      entityId: string;
      displayName: string;
      message: string;
    }
  | { type: 'warning'; entityId: string | null; message: string }
  | { type: 'log'; entry: IngestLogEntry };
