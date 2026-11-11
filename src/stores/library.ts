import { open as openDialog } from '@tauri-apps/plugin-dialog';
import { defineStore } from 'pinia';
import { computed, reactive, ref, shallowRef, watch } from 'vue';

import { api, errorCode, errorMessage } from '@/api';
import { resetThumbnails } from '@/composables/useThumbnails';
import {
  editorNameFromPath,
  isApplicationPath,
  normalizeDefaultEditorPath,
  normalizeExternalEditors,
} from '@/lib/editors';
import { basename, isPathInside, parentPath } from '@/lib/format';
import { pruneFolderRemovalTargets } from '@/lib/folders';
import { DEFAULT_INGEST_SETTINGS, normalizeIngestSettings } from '@/lib/ingest';
import {
  commonMetadataValue,
  cullPatch,
  favoritePatch,
  labelPatch,
  mergeMetadataResult,
  ratingPatch,
} from '@/lib/metadata';
import {
  DEFAULT_SHORTCUTS,
  canonicalBinding,
  formatBindingLabel,
  matchesShortcut,
  normalizeShortcutOverrides,
  resolveShortcutBinding,
  type ShortcutId,
} from '@/lib/shortcuts';
import {
  clickSelection,
  emptySelection,
  navigateSelection,
  reconcileSelection,
  selectAllSelection,
  singleSelection,
  type SelectionState,
} from '@/lib/selection';
import type {
  AddFolderOutcome,
  AssetRemovalMode,
  ColorLabel,
  CullFlag,
  ExcludedAsset,
  ExternalEditor,
  FolderImageCounts,
  FolderMenuTarget,
  FolderNode,
  FolderRemovalTarget,
  IngestSettings,
  MetadataUpdate,
  MonoFont,
  NewFolderTarget,
  PhotoAsset,
  PhotoMenuTarget,
  RootInfo,
  RootRemovalMode,
  ScanWarning,
  ThemeSetting,
  UiFont,
  UiSettings,
  ViewMode,
} from '@/types';

export const DEFAULT_UI_SETTINGS: UiSettings = {
  theme: 'system',
  uiFont: 'avenir',
  monoFont: 'jetbrains',
  uiFontSize: 13,
  viewMode: 'grid',
  recursive: false,
  thumbnailSize: 184,
  libraryWidth: 276,
  infoWidth: 320,
  lastRootId: null,
  lastFolder: null,
  filmstripVisible: true,
  listShowAssociations: false,
  externalEditors: [],
  defaultEditorPath: null,
  shortcuts: {},
};

const BASE_FONT_SIZE = 13;

interface FolderRow {
  path: string;
  name: string;
  rootId: string;
  isRoot: boolean;
}

const UI_FONT_STACKS: Record<UiFont, string> = {
  avenir:
    "'Avenir Next', Avenir, -apple-system, BlinkMacSystemFont, 'SF Pro Text', 'Segoe UI', Inter, Roboto, 'Helvetica Neue', Arial, sans-serif",
  system:
    "-apple-system, BlinkMacSystemFont, 'SF Pro Text', 'Segoe UI', Inter, Roboto, 'Helvetica Neue', Arial, sans-serif",
};

const MONO_FONT_STACKS: Record<MonoFont, string> = {
  jetbrains:
    "'JetBrains Mono Variable', 'JetBrains Mono', 'SF Mono', ui-monospace, Menlo, Monaco, monospace",
  system: "'SF Mono', ui-monospace, Menlo, Monaco, Consolas, monospace",
};

export type ToastKind = 'info' | 'success' | 'error';

export interface Toast {
  id: number;
  message: string;
  kind: ToastKind;
}

export interface BreadcrumbItem {
  name: string;
  path: string;
}

export interface FolderStats {
  total: number;
  raw: number;
  jpeg: number;
  other: number;
  withXmp: number;
  withCompanion: number;
}

interface FolderTargetOptions {
  force?: boolean;
  expandAncestors?: boolean;
}

function readSystemDark(): boolean {
  if (typeof window === 'undefined' || !window.matchMedia) return false;
  return window.matchMedia('(prefers-color-scheme: dark)').matches;
}

function normalizeUiSettings(ui: Partial<UiSettings>): UiSettings {
  const merged = { ...DEFAULT_UI_SETTINGS, ...ui };
  const size = Number.isFinite(merged.uiFontSize) ? Math.round(merged.uiFontSize) : BASE_FONT_SIZE;
  const externalEditors = normalizeExternalEditors(merged.externalEditors);
  return {
    ...merged,
    uiFont: merged.uiFont === 'system' ? 'system' : 'avenir',
    monoFont: merged.monoFont === 'system' ? 'system' : 'jetbrains',
    uiFontSize: Math.min(17, Math.max(11, size)),
    externalEditors,
    defaultEditorPath: normalizeDefaultEditorPath(merged.defaultEditorPath, externalEditors),
    shortcuts: normalizeShortcutOverrides(merged.shortcuts),
  };
}

export const useLibraryStore = defineStore('library', () => {
  const initializing = ref(true);
  const bootstrapped = ref(false);
  const initializationError = ref<string | null>(null);

  const roots = ref<RootInfo[]>([]);
  const settings = ref<UiSettings>({ ...DEFAULT_UI_SETTINGS });
  const ingestSettings = ref<IngestSettings>({
    ...DEFAULT_INGEST_SETTINGS,
    templates: [],
    iptc: { ...DEFAULT_INGEST_SETTINGS.iptc, keywords: [] },
  });
  const cacheDir = ref('');
  const recoveryWarning = ref<string | null>(null);
  const recoveryWarningDismissed = ref(false);
  const appVersion = ref('');

  const tree = reactive(new Map<string, FolderNode[]>());
  const treeLoading = reactive(new Set<string>());
  const treeErrors = reactive(new Map<string, string>());
  const expanded = reactive(new Set<string>());

  // Asset counts are cached per folder, with direct and recursive values so the
  // tree can switch immediately when "Include subfolders" changes.
  const folderImageCounts = reactive(new Map<string, FolderImageCounts>());
  const folderImageCountsLoading = reactive(new Set<string>());
  const folderCountsEpoch = ref(0);

  const selectedRootId = ref<string | null>(null);
  const selectedFolder = ref<string | null>(null);
  /** Folder rows highlighted for bulk actions; independent from the folder being browsed. */
  const selectedFolderPaths = ref<ReadonlySet<string>>(new Set());
  const folderSelectionAnchor = ref<string | null>(null);
  const assets = shallowRef<PhotoAsset[]>([]);
  const warnings = ref<ScanWarning[]>([]);
  const scanState = ref<'idle' | 'loading' | 'ready' | 'error'>('idle');
  const scanError = ref<string | null>(null);
  const scanErrorCode = ref<string | null>(null);
  const scanElapsedMs = ref(0);
  const scanRecursive = ref(false);
  const navRowSize = ref(1);

  const selectedIds = ref<ReadonlySet<string>>(new Set());
  const primaryId = ref<string | null>(null);
  const anchorId = ref<string | null>(null);
  const selectionId = computed(() => primaryId.value);
  const hiddenAssets = ref<ExcludedAsset[]>([]);
  const fullscreenOpen = ref(false);
  const settingsOpen = ref(false);
  const warningsOpen = ref(false);
  const shortcutsOpen = ref(false);
  const infoVisible = ref(true);
  const addingRoot = ref(false);
  const addingEditor = ref(false);
  const removeFolderOpen = ref(false);
  const removeFolderTargets = ref<FolderRemovalTarget[]>([]);
  const removingFolder = ref(false);
  const removeFolderError = ref<string | null>(null);
  const deleteAssetsOpen = ref(false);
  const deleteAssetsTargets = ref<PhotoAsset[]>([]);
  const removingAssets = ref(false);
  const removeAssetsError = ref<string | null>(null);
  const assetDragIds = ref<string[]>([]);
  const dragOverFolder = ref<string | null>(null);
  const movingAssets = ref(false);
  const folderMenuOpen = ref(false);
  const folderMenuTarget = ref<FolderMenuTarget | null>(null);
  const photoMenuOpen = ref(false);
  const photoMenuTarget = ref<PhotoMenuTarget | null>(null);
  const newFolderOpen = ref(false);
  const newFolderTarget = ref<NewFolderTarget | null>(null);
  const creatingFolder = ref(false);
  const newFolderError = ref<string | null>(null);
  const clearingCache = ref(false);
  const metadataBusy = ref(false);
  const metadataError = ref<string | null>(null);
  const toasts = ref<Toast[]>([]);

  watch(primaryId, () => {
    metadataError.value = null;
  });

  const systemDark = ref(readSystemDark());
  const resolvedTheme = computed<'light' | 'dark'>(() =>
    settings.value.theme === 'system'
      ? systemDark.value
        ? 'dark'
        : 'light'
      : settings.value.theme === 'dark'
        ? 'dark'
        : 'light',
  );

  watch(
    resolvedTheme,
    (theme) => {
      if (typeof document === 'undefined') return;
      document.documentElement.classList.toggle('dark', theme === 'dark');
      document.documentElement.style.colorScheme = theme;
    },
    { immediate: true },
  );

  watch(
    () => [settings.value.uiFont, settings.value.monoFont, settings.value.uiFontSize] as const,
    ([uiFont, monoFont, uiFontSize]) => {
      if (typeof document === 'undefined') return;
      const root = document.documentElement;
      root.style.setProperty('--font-ui', UI_FONT_STACKS[uiFont] ?? UI_FONT_STACKS.avenir);
      root.style.setProperty('--font-mono', MONO_FONT_STACKS[monoFont] ?? MONO_FONT_STACKS.jetbrains);
      root.style.setProperty('--font-scale', (uiFontSize / BASE_FONT_SIZE).toFixed(4));
    },
    { immediate: true },
  );

  const selectedRoot = computed(
    () => roots.value.find((root) => root.id === selectedRootId.value) ?? null,
  );

  /** Folder rows currently visible in the tree, in display order. */
  const folderRows = computed<FolderRow[]>(() => {
    const rows: FolderRow[] = [];
    const seen = new Set<string>();
    function walk(row: FolderRow): void {
      if (seen.has(row.path)) return;
      seen.add(row.path);
      rows.push(row);
      if (!expanded.has(row.path)) return;
      for (const child of tree.get(row.path) ?? []) {
        walk({ path: child.path, name: child.name, rootId: row.rootId, isRoot: false });
      }
    }
    for (const root of roots.value) {
      walk({ path: root.path, name: root.name, rootId: root.id, isRoot: true });
    }
    return rows;
  });

  const folderOrder = computed(() => folderRows.value.map((row) => row.path));

  const selectedFolderTargets = computed<FolderRemovalTarget[]>(() => {
    const byPath = new Map(folderRows.value.map((row) => [row.path, row]));
    return [...selectedFolderPaths.value].flatMap((path) => {
      const row = byPath.get(path);
      if (!row) return [];
      return [
        {
          kind: row.isRoot ? ('root' as const) : ('folder' as const),
          rootId: row.rootId,
          name: row.name,
          path: row.path,
        },
      ];
    });
  });

  const selectedFolderCount = computed(() => selectedFolderTargets.value.length);

  function currentFolderSelection(): SelectionState {
    return {
      ids: [...selectedFolderPaths.value],
      primaryId: selectedFolder.value,
      anchorId: folderSelectionAnchor.value,
    };
  }

  function applyFolderSelection(state: SelectionState): void {
    selectedFolderPaths.value = new Set(state.ids);
    folderSelectionAnchor.value = state.anchorId;
  }

  function isFolderSelected(path: string): boolean {
    return selectedFolderPaths.value.has(path);
  }

  function selectFolderRow(
    path: string,
    rootId: string,
    modifiers: { metaKey?: boolean; ctrlKey?: boolean; shiftKey?: boolean } = {},
  ): void {
    if (modifiers.shiftKey || modifiers.metaKey || modifiers.ctrlKey) {
      applyFolderSelection(
        clickSelection(folderOrder.value, currentFolderSelection(), path, modifiers),
      );
      return;
    }
    applyFolderSelection(singleSelection(path));
    void selectFolder(path, rootId);
  }

  function clearFolderSelection(): void {
    applyFolderSelection(emptySelection());
  }

  // Keep the highlight in sync when a folder is opened from outside the tree
  // (startup restore, delete fallback, ingest, and so on).
  watch(selectedFolder, (path) => {
    if (!path) {
      if (selectedFolderPaths.value.size) applyFolderSelection(emptySelection());
      return;
    }
    if (!selectedFolderPaths.value.has(path)) applyFolderSelection(singleSelection(path));
  });

  const assetIndex = computed(() => {
    const index = new Map<string, PhotoAsset>();
    for (const asset of assets.value) index.set(asset.id, asset);
    return index;
  });

  const assetOrder = computed(() => assets.value.map((asset) => asset.id));
  let focusIndexAfterScan: number | null = null;

  const selectedAsset = computed(() => {
    if (!primaryId.value) return null;
    return assetIndex.value.get(primaryId.value) ?? null;
  });

  const selectedIndex = computed(() => {
    if (!primaryId.value) return -1;
    return assets.value.findIndex((asset) => asset.id === primaryId.value);
  });

  const selectedCount = computed(() => selectedIds.value.size);

  const selectedAssets = computed(() =>
    assets.value.filter((asset) => selectedIds.value.has(asset.id)),
  );

  const externalEditors = computed<ExternalEditor[]>(() => settings.value.externalEditors);

  const defaultEditor = computed<ExternalEditor | null>(() => {
    const path = settings.value.defaultEditorPath;
    return (
      settings.value.externalEditors.find((editor) => editor.path === path) ??
      settings.value.externalEditors[0] ??
      null
    );
  });

  function isSelected(id: string): boolean {
    return selectedIds.value.has(id);
  }

  function currentSelection(): SelectionState {
    const ids = assets.value.filter((asset) => selectedIds.value.has(asset.id)).map((a) => a.id);
    if (!ids.length) return emptySelection();
    return {
      ids,
      primaryId: primaryId.value && selectedIds.value.has(primaryId.value) ? primaryId.value : ids[0],
      anchorId: anchorId.value && selectedIds.value.has(anchorId.value) ? anchorId.value : ids[0],
    };
  }

  function applySelection(next: SelectionState): void {
    selectedIds.value = new Set(next.ids);
    primaryId.value = next.primaryId;
    anchorId.value = next.anchorId;
  }

  const stats = computed<FolderStats>(() => {
    let raw = 0;
    let jpeg = 0;
    let other = 0;
    let withXmp = 0;
    let withCompanion = 0;
    for (const asset of assets.value) {
      if (asset.kind === 'raw') raw += 1;
      else if (asset.kind === 'jpeg') jpeg += 1;
      else other += 1;
      if (asset.xmpSidecar) withXmp += 1;
      if (asset.companionJpeg) withCompanion += 1;
    }
    return { total: assets.value.length, raw, jpeg, other, withXmp, withCompanion };
  });

  const folderName = computed(() => {
    if (!selectedFolder.value) return 'Library';
    if (selectedRoot.value && selectedFolder.value === selectedRoot.value.path) {
      return selectedRoot.value.name;
    }
    return basename(selectedFolder.value);
  });

  const relativeFolderLabel = computed(() => {
    if (!selectedFolder.value || !selectedRoot.value) return '';
    const base = selectedRoot.value.path.replace(/[\\/]+$/, '');
    if (selectedFolder.value === base) return selectedRoot.value.name;
    return `${selectedRoot.value.name}${selectedFolder.value.slice(base.length)}`;
  });

  const breadcrumb = computed<BreadcrumbItem[]>(() => {
    const folder = selectedFolder.value;
    const root = selectedRoot.value;
    if (!folder || !root) return [];
    const base = root.path.replace(/[\\/]+$/, '');
    const relative = folder.slice(base.length).replace(/^[\\/]+/, '');
    const parts = relative ? relative.split(/[\\/]/) : [];
    const crumbs: BreadcrumbItem[] = [{ name: root.name, path: root.path }];
    let current = base;
    for (const part of parts) {
      current = `${current}/${part}`;
      crumbs.push({ name: part, path: current });
    }
    return crumbs;
  });

  const viewMode = computed<ViewMode>(() => settings.value.viewMode);
  const warningsCount = computed(() => warnings.value.length);
  const hasRoots = computed(() => roots.value.length > 0);

  let toastSerial = 0;
  let persistTimer: number | undefined;

  function addToast(message: string, kind: ToastKind = 'info', duration?: number): void {
    const id = ++toastSerial;
    toasts.value = [...toasts.value, { id, message, kind }];
    if (typeof window !== 'undefined') {
      const timeout = duration ?? (kind === 'error' ? 7000 : 4000);
      window.setTimeout(() => dismissToast(id), timeout);
    }
  }

  function dismissToast(id: number): void {
    toasts.value = toasts.value.filter((toast) => toast.id !== id);
  }

  function queuePersist(): void {
    if (typeof window === 'undefined') return;
    if (persistTimer) window.clearTimeout(persistTimer);
    persistTimer = window.setTimeout(() => {
      persistTimer = undefined;
      api.updateUiSettings(settings.value).catch((error: unknown) => {
        console.error('Could not save settings', error);
      });
    }, 350);
  }

  function updateSettings(patch: Partial<UiSettings>): void {
    settings.value = { ...settings.value, ...patch };
    queuePersist();
  }

  function setTheme(theme: ThemeSetting): void {
    if (!['system', 'light', 'dark'].includes(theme)) return;
    settings.value = { ...settings.value, theme };
    try {
      localStorage.setItem('lumio.theme', theme);
    } catch {
      /* localStorage may be unavailable; theme still applies for this session */
    }
    queuePersist();
  }

  function toggleTheme(): void {
    setTheme(resolvedTheme.value === 'dark' ? 'light' : 'dark');
  }

  function setUiFont(font: UiFont): void {
    if (!['avenir', 'system'].includes(font)) return;
    if (settings.value.uiFont === font) return;
    updateSettings({ uiFont: font });
  }

  function setMonoFont(font: MonoFont): void {
    if (!['jetbrains', 'system'].includes(font)) return;
    if (settings.value.monoFont === font) return;
    updateSettings({ monoFont: font });
  }

  function setUiFontSize(size: number): void {
    const clamped = Math.min(17, Math.max(11, Math.round(size)));
    if (clamped === settings.value.uiFontSize) return;
    updateSettings({ uiFontSize: clamped });
  }

  function setViewMode(mode: ViewMode): void {
    // The bottom strip belongs to the filmstrip view alone. Browse layouts must
    // never leave it reachable, so visibility follows the active view exactly.
    updateSettings({ viewMode: mode, filmstripVisible: mode === 'filmstrip' });
  }

  function setThumbnailSize(size: number): void {
    const clamped = Math.min(480, Math.max(120, Math.round(size)));
    if (clamped === settings.value.thumbnailSize) return;
    updateSettings({ thumbnailSize: clamped });
  }

  function setRecursive(value: boolean): void {
    if (settings.value.recursive === value) return;
    updateSettings({ recursive: value });
    if (selectedFolder.value) void scanFolder();
  }

  function setFilmstripVisible(value: boolean): void {
    // Ignore requests from browse layouts: the strip is only meaningful inside
    // the filmstrip view and must stay hidden everywhere else.
    if (settings.value.viewMode !== 'filmstrip') return;
    updateSettings({ filmstripVisible: value });
  }

  function toggleFilmstrip(): void {
    setFilmstripVisible(!settings.value.filmstripVisible);
  }

  function toggleListAssociations(): void {
    updateSettings({ listShowAssociations: !settings.value.listShowAssociations });
  }

  function shortcutBinding(id: ShortcutId): string {
    return resolveShortcutBinding(settings.value.shortcuts, id);
  }

  function shortcutLabel(id: ShortcutId): string {
    return formatBindingLabel(shortcutBinding(id));
  }

  function shortcutIsCustom(id: ShortcutId): boolean {
    return Boolean(settings.value.shortcuts[id]);
  }

  function shortcutMatches(event: KeyboardEvent, id: ShortcutId): boolean {
    return matchesShortcut(event, id, settings.value.shortcuts);
  }

  function setShortcut(id: ShortcutId, binding: string): void {
    const canonical = canonicalBinding(binding);
    if (!canonical || canonical === shortcutBinding(id)) return;
    const shortcuts = { ...settings.value.shortcuts };
    if (canonical === DEFAULT_SHORTCUTS[id]) delete shortcuts[id];
    else shortcuts[id] = canonical;
    updateSettings({ shortcuts });
  }

  function resetShortcut(id: ShortcutId): void {
    if (!settings.value.shortcuts[id]) return;
    const shortcuts = { ...settings.value.shortcuts };
    delete shortcuts[id];
    updateSettings({ shortcuts });
  }

  function resetShortcuts(): void {
    if (!Object.keys(settings.value.shortcuts).length) return;
    updateSettings({ shortcuts: {} });
  }

  async function addExternalEditor(): Promise<void> {
    if (addingEditor.value) return;
    addingEditor.value = true;
    try {
      const picked = await openDialog({
        directory: false,
        multiple: false,
        title: 'Choose an application',
        defaultPath: '/Applications',
        filters: [{ name: 'Applications', extensions: ['app'] }],
      });
      if (!picked || Array.isArray(picked)) return;
      const path = picked.replace(/[\\/]+$/, '');
      if (!isApplicationPath(path)) {
        addToast('Choose a macOS application bundle (.app).', 'error');
        return;
      }
      if (settings.value.externalEditors.some((editor) => editor.path === path)) {
        addToast(`${editorNameFromPath(path)} is already available for editing.`, 'info');
        return;
      }
      const externalEditors = [
        ...settings.value.externalEditors,
        { name: editorNameFromPath(path), path },
      ];
      updateSettings({
        externalEditors,
        defaultEditorPath: settings.value.defaultEditorPath ?? path,
      });
    } catch (error) {
      addToast(errorMessage(error), 'error');
    } finally {
      addingEditor.value = false;
    }
  }

  function setDefaultEditor(path: string): void {
    if (!settings.value.externalEditors.some((editor) => editor.path === path)) return;
    if (settings.value.defaultEditorPath === path) return;
    updateSettings({ defaultEditorPath: path });
  }

  function removeExternalEditor(path: string): void {
    const externalEditors = settings.value.externalEditors.filter(
      (editor) => editor.path !== path,
    );
    if (externalEditors.length === settings.value.externalEditors.length) return;
    const defaultEditorPath =
      settings.value.defaultEditorPath === path
        ? (externalEditors[0]?.path ?? null)
        : settings.value.defaultEditorPath;
    updateSettings({ externalEditors, defaultEditorPath });
  }

  async function editSelectionWithApplication(applicationPath: string): Promise<void> {
    const targets = selectedAssets.value;
    if (!targets.length) {
      addToast('Select a photo to edit.', 'info');
      return;
    }
    try {
      await api.editAssets(
        targets.map((asset) => asset.primaryPath),
        applicationPath,
      );
    } catch (error) {
      addToast(errorMessage(error), 'error');
    }
  }

  function editSelectionWithDefault(): void {
    if (!selectedAssets.value.length) {
      addToast('Select a photo to edit.', 'info');
      return;
    }
    const editor = defaultEditor.value;
    if (!editor) {
      addToast('Add an external editor in Settings to edit photos.', 'info');
      openSettings();
      return;
    }
    closePhotoMenu();
    void editSelectionWithApplication(editor.path);
  }

  async function applyMetadataPatch(
    update: MetadataUpdate,
    targets?: readonly PhotoAsset[],
  ): Promise<void> {
    const source = targets ?? selectedAssets.value;
    const assetsToUpdate = source.filter((asset) => asset.primaryPath);
    if (!assetsToUpdate.length) {
      addToast('Select a photo to edit metadata.', 'info');
      return;
    }
    if (metadataBusy.value) return;
    metadataBusy.value = true;
    metadataError.value = null;
    try {
      const outcome = await api.updateMetadata(
        assetsToUpdate.map((asset) => ({
          asset,
          expectedRevision: asset.metadata?.revision ?? '',
          update,
        })),
      );

      if (outcome.updated.length) {
        const byId = new Map(outcome.updated.map((result) => [result.assetId, result]));
        assets.value = assets.value.map((asset) => {
          const result = byId.get(asset.id);
          if (!result) return asset;
          return mergeMetadataResult(
            asset,
            result.metadata,
            result.xmpSidecar,
            result.fileSize,
            result.modifiedMs,
          );
        });
        addToast(
          `Updated metadata for ${outcome.updated.length} ${
            outcome.updated.length === 1 ? 'photo' : 'photos'
          }.`,
          'success',
        );
      }

      if (outcome.failed.length) {
        for (const failure of outcome.failed) {
          addToast(`${failure.fileName}: ${failure.message}`, 'error');
        }
        metadataError.value =
          outcome.failed.length === 1
            ? outcome.failed[0].message
            : `${outcome.failed.length} photos could not be updated.`;
      }
    } catch (error) {
      const message = errorMessage(error);
      metadataError.value = message;
      addToast(message, 'error');
    } finally {
      metadataBusy.value = false;
    }
  }

  function setRating(rating: number): void {
    const current = commonMetadataValue(selectedAssets.value, (value) => value.rating, 0);
    const next = Math.max(0, Math.min(5, Math.round(rating)));
    void applyMetadataPatch(ratingPatch(current.value, current.mixed, next));
  }

  function setColorLabel(label: ColorLabel | null): void {
    const current = commonMetadataValue(selectedAssets.value, (value) => value.label, null);
    void applyMetadataPatch(labelPatch(current.value, current.mixed, label));
  }

  function setCull(cull: CullFlag): void {
    const current = commonMetadataValue(selectedAssets.value, (value) => value.cull, null);
    void applyMetadataPatch(cullPatch(current.value, current.mixed, cull));
  }

  function unflagSelection(): void {
    if (!selectedAssets.value.length) {
      addToast('Select a photo to edit metadata.', 'info');
      return;
    }
    void applyMetadataPatch({ clearCull: true });
  }

  function toggleFavorite(): void {
    const current = commonMetadataValue(
      selectedAssets.value,
      (value) => value.favorite,
      false,
    );
    const allFavorite = !current.mixed && current.value === true;
    void applyMetadataPatch(favoritePatch(allFavorite, current.mixed));
  }

  function toggleInfo(): void {
    infoVisible.value = !infoVisible.value;
  }

  function openSettings(): void {
    settingsOpen.value = true;
  }

  function closeSettings(): void {
    settingsOpen.value = false;
  }

  function openShortcuts(): void {
    shortcutsOpen.value = true;
  }

  function closeShortcuts(): void {
    shortcutsOpen.value = false;
  }

  function openWarnings(): void {
    warningsOpen.value = true;
  }

  function closeWarnings(): void {
    warningsOpen.value = false;
  }

  function dismissRecoveryWarning(): void {
    recoveryWarningDismissed.value = true;
  }

  function select(id: string | null): void {
    applySelection(id ? singleSelection(id) : emptySelection());
  }

  function selectWithModifiers(
    id: string,
    modifiers: { metaKey?: boolean; ctrlKey?: boolean; shiftKey?: boolean } = {},
  ): void {
    applySelection(clickSelection(assetOrder.value, currentSelection(), id, modifiers));
  }

  function selectAll(): void {
    applySelection(selectAllSelection(assetOrder.value));
  }

  function clearSelection(): void {
    applySelection(emptySelection());
  }

  function navigateBy(offset: number, extend = false): void {
    applySelection(navigateSelection(assetOrder.value, currentSelection(), offset, extend));
  }

  function navigateHorizontal(delta: number, extend = false): void {
    navigateBy(delta, extend);
  }

  function navigateVertical(delta: number, extend = false): void {
    navigateBy(delta * Math.max(1, navRowSize.value), extend);
  }

  function openInFilmstrip(id?: string): void {
    if (id) select(id);
    if (!primaryId.value && assets.value.length) select(assets.value[0]?.id ?? null);
    if (!primaryId.value) return;
    // Opening from any browsing layout switches to the filmstrip view and makes
    // sure its strip is visible so the selected photo has context.
    setViewMode('filmstrip');
  }

  function openFullscreen(id?: string): void {
    if (id) select(id);
    if (!primaryId.value && assets.value.length) select(assets.value[0]?.id ?? null);
    if (!primaryId.value) return;
    fullscreenOpen.value = true;
  }

  function closeFullscreen(): void {
    fullscreenOpen.value = false;
  }

  function toggleFullscreen(): void {
    if (fullscreenOpen.value) closeFullscreen();
    else openFullscreen();
  }

  function imageCountFor(path: string): number | null {
    const counts = folderImageCounts.get(path);
    if (!counts) return null;
    return settings.value.recursive ? counts.recursive : counts.direct;
  }

  async function ensureFolderImageCount(path: string): Promise<void> {
    if (folderImageCounts.has(path) || folderImageCountsLoading.has(path)) return;
    const epoch = folderCountsEpoch.value;
    folderImageCountsLoading.add(path);
    try {
      const counts = await api.countFolderImages(path);
      if (epoch === folderCountsEpoch.value) folderImageCounts.set(path, counts);
    } catch (error) {
      // Counts are an enhancement; a folder row should still work if counting
      // fails or the folder disappears between renders.
      console.error('Could not count images in folder', path, error);
    } finally {
      folderImageCountsLoading.delete(path);
      if (epoch !== folderCountsEpoch.value && !folderImageCounts.has(path)) {
        void ensureFolderImageCount(path);
      }
    }
  }

  async function loadChildren(path: string, force = false): Promise<void> {
    if (treeLoading.has(path)) return;
    if (!force && tree.has(path)) return;
    treeLoading.add(path);
    treeErrors.delete(path);
    try {
      const children = await api.listSubfolders(path);
      tree.set(path, children);
      for (const child of children) {
        folderImageCounts.set(child.path, {
          direct: child.imageCount,
          recursive: child.recursiveImageCount,
        });
      }
    } catch (error) {
      treeErrors.set(path, errorMessage(error));
    } finally {
      treeLoading.delete(path);
    }
  }

  async function expandAncestors(path: string, rootPath: string): Promise<void> {
    const base = rootPath.replace(/[\\/]+$/, '');
    const relative = path.slice(base.length).replace(/^[\\/]+/, '');
    const parts = relative ? relative.split(/[\\/]/) : [];
    expanded.add(rootPath);
    await loadChildren(rootPath);
    let current = base;
    for (const part of parts.slice(0, -1)) {
      current = `${current}/${part}`;
      expanded.add(current);
      await loadChildren(current);
    }
  }

  async function toggleExpanded(path: string): Promise<void> {
    if (expanded.has(path)) {
      expanded.delete(path);
      return;
    }
    expanded.add(path);
    await loadChildren(path);
  }

  async function scanFolder(): Promise<void> {
    const folder = selectedFolder.value;
    if (!folder) return;
    const token = ++scanToken;
    scanState.value = 'loading';
    scanError.value = null;
    scanErrorCode.value = null;
    warnings.value = [];
    scanRecursive.value = settings.value.recursive;
    try {
      const result = await api.scanFolder(folder, settings.value.recursive);
      if (token !== scanToken) return;
      assets.value = result.assets;
      warnings.value = result.warnings;
      scanElapsedMs.value = result.elapsedMs;
      scanState.value = 'ready';
      applySelection(
        reconcileSelection(
          result.assets.map((asset) => asset.id),
          currentSelection(),
          focusIndexAfterScan,
        ),
      );
      focusIndexAfterScan = null;
    } catch (error) {
      if (token !== scanToken) return;
      assets.value = [];
      warnings.value = [];
      applySelection(emptySelection());
      focusIndexAfterScan = null;
      scanState.value = 'error';
      scanError.value = errorMessage(error);
      scanErrorCode.value = errorCode(error);
    }
  }

  let scanToken = 0;

  async function selectFolder(
    path: string,
    rootId: string,
    options: FolderTargetOptions = {},
  ): Promise<void> {
    const root = roots.value.find((candidate) => candidate.id === rootId);
    if (!root) return;
    if (!isPathInside(path, root.path)) {
      addToast('That folder is not part of the registered library.', 'error');
      return;
    }
    if (path === selectedFolder.value && rootId === selectedRootId.value && !options.force) {
      if (scanState.value === 'error') await scanFolder();
      return;
    }
    selectedRootId.value = rootId;
    selectedFolder.value = path;
    applySelection(emptySelection());
    assets.value = [];
    warnings.value = [];
    scanElapsedMs.value = 0;
    scanError.value = null;
    scanErrorCode.value = null;
    settings.value = {
      ...settings.value,
      lastRootId: rootId,
      lastFolder: path,
    };
    queuePersist();
    if (options.expandAncestors !== false) {
      void expandAncestors(path, root.path);
    }
    await scanFolder();
  }

  async function registerRoot(
    path: string,
    options: { select?: boolean; toast?: boolean } = {},
  ): Promise<AddFolderOutcome> {
    const outcome = await api.addRoot(path);
    const info = outcome.root;
    roots.value = roots.value.some((root) => root.id === info.id)
      ? roots.value.map((root) => (root.id === info.id ? info : root))
      : [...roots.value, info];
    for (const key of [...folderImageCounts.keys()]) {
      if (isPathInside(key, info.path)) folderImageCounts.delete(key);
    }
    folderCountsEpoch.value += 1;
    expanded.add(info.path);
    await loadChildren(info.path, true);
    const target = isPathInside(outcome.folder, info.path) ? outcome.folder : info.path;
    if (options.select) await selectFolder(target, info.id);
    if (options.toast !== false) {
      addToast(
        outcome.alreadyInLibrary
          ? `${basename(target)} is already in the library.`
          : outcome.restored
            ? `Restored ${basename(target)} to the library.`
            : `Added ${info.name} to the library.`,
        'success',
      );
    }
    return outcome;
  }

  async function addRoot(): Promise<void> {
    if (addingRoot.value) return;
    addingRoot.value = true;
    try {
      const picked = await openDialog({
        directory: true,
        multiple: false,
        title: 'Add a photo folder to Lumio',
      });
      if (!picked || Array.isArray(picked)) return;
      await registerRoot(picked, { select: true });
    } catch (error) {
      addToast(errorMessage(error), 'error');
    } finally {
      addingRoot.value = false;
    }
  }

  async function saveIngestSettings(next: IngestSettings): Promise<IngestSettings> {
    const saved = await api.updateIngestSettings(normalizeIngestSettings(next));
    ingestSettings.value = normalizeIngestSettings(saved);
    return ingestSettings.value;
  }

  function clearFolderCaches(pathPrefix: string): void {
    const inside = (path: string) => isPathInside(path, pathPrefix);
    for (const key of [...tree.keys()]) {
      if (inside(key)) tree.delete(key);
    }
    for (const key of [...treeErrors.keys()]) {
      if (inside(key)) treeErrors.delete(key);
    }
    for (const key of [...treeLoading]) {
      if (inside(key)) treeLoading.delete(key);
    }
    for (const key of [...expanded]) {
      if (inside(key)) expanded.delete(key);
    }
    for (const key of [...folderImageCounts.keys()]) {
      if (inside(key)) folderImageCounts.delete(key);
    }
    for (const key of [...folderImageCountsLoading]) {
      if (inside(key)) folderImageCountsLoading.delete(key);
    }
  }

  function forgetRootState(rootPath: string): void {
    clearFolderCaches(rootPath);
    folderCountsEpoch.value += 1;
  }

  function forgetSubfolderState(path: string, rootId: string): void {
    // Drop the row from its parent's cached listing so the tree updates
    // immediately without reloading the whole folder.
    const parent = parentPath(path);
    const siblings = tree.get(parent);
    if (siblings) {
      tree.set(
        parent,
        siblings.filter((child) => child.path !== path),
      );
    }

    clearFolderCaches(path);

    // Ancestor counts still include the deleted folder, so refresh all counts
    // for the root. Visible rows will re-request theirs via folderCountsEpoch.
    const root = roots.value.find((candidate) => candidate.id === rootId);
    if (root) {
      for (const key of [...folderImageCounts.keys()]) {
        if (isPathInside(key, root.path)) folderImageCounts.delete(key);
      }
    }

    folderCountsEpoch.value += 1;
  }

  function openRemoveFolder(id: string): void {
    const root = roots.value.find((candidate) => candidate.id === id);
    if (!root) return;
    openRemoveFolders([
      {
        kind: 'root',
        rootId: root.id,
        name: root.name,
        path: root.path,
      },
    ]);
  }

  function openDeleteFolder(path: string, rootId: string): void {
    const root = roots.value.find((candidate) => candidate.id === rootId);
    if (!root || !isPathInside(path, root.path) || path === root.path) return;
    openRemoveFolders([
      {
        kind: 'folder',
        rootId,
        name: basename(path),
        path,
      },
    ]);
  }

  function openRemoveFolders(targets: readonly FolderRemovalTarget[]): void {
    const pending = pruneFolderRemovalTargets(targets);
    if (!pending.length) return;
    removeFolderTargets.value = pending;
    removeFolderOpen.value = true;
    removeFolderError.value = null;
  }

  function openRemoveSelectedFolders(): void {
    openRemoveFolders(selectedFolderTargets.value);
  }

  function removeFolderFromRow(path: string, rootId: string, isRoot: boolean): void {
    if (selectedFolderPaths.value.has(path) && selectedFolderTargets.value.length > 1) {
      openRemoveSelectedFolders();
      return;
    }
    if (isRoot) openRemoveFolder(rootId);
    else openDeleteFolder(path, rootId);
  }

  function closeRemoveFolder(): void {
    if (removingFolder.value) return;
    removeFolderOpen.value = false;
    removeFolderTargets.value = [];
    removeFolderError.value = null;
  }

  function openFolderMenu(target: Omit<FolderMenuTarget, 'x' | 'y'>, x: number, y: number): void {
    closePhotoMenu();
    // Right-clicking outside the current folder selection replaces it with the
    // clicked row; right-clicking inside it keeps the whole selection.
    if (!selectedFolderPaths.value.has(target.path)) {
      applyFolderSelection(singleSelection(target.path));
    }
    folderMenuTarget.value = { ...target, x, y };
    folderMenuOpen.value = true;
  }

  function closeFolderMenu(): void {
    folderMenuOpen.value = false;
    folderMenuTarget.value = null;
  }

  function openPhotoMenu(id: string, x: number, y: number): void {
    const asset = assetIndex.value.get(id);
    if (!asset) return;
    // Right-clicking outside the current selection replaces it with the
    // clicked photo; right-clicking inside it keeps the whole selection.
    if (!selectedIds.value.has(id)) select(id);
    closeFolderMenu();
    photoMenuTarget.value = {
      id,
      name: selectedCount.value > 1 ? `${selectedCount.value} photos` : asset.fileName,
      count: selectedCount.value,
      x,
      y,
    };
    photoMenuOpen.value = true;
  }

  function closePhotoMenu(): void {
    photoMenuOpen.value = false;
    photoMenuTarget.value = null;
  }

  function deleteFromPhotoMenu(): void {
    closePhotoMenu();
    openDeleteAssets();
  }

  function removeFromFolderMenu(): void {
    const target = folderMenuTarget.value;
    if (!target) return;
    closeFolderMenu();
    const selected = selectedFolderTargets.value;
    if (selected.length > 1) {
      openRemoveFolders(selected);
      return;
    }
    if (target.isRoot) openRemoveFolder(target.rootId);
    else openDeleteFolder(target.path, target.rootId);
  }

  function refreshFromFolderMenu(): void {
    const target = folderMenuTarget.value;
    if (!target) return;
    closeFolderMenu();
    void refreshFolder(target.path, target.rootId);
  }

  function openNewFolder(): void {
    const target = folderMenuTarget.value;
    if (!target) return;
    newFolderTarget.value = { path: target.path, rootId: target.rootId, name: target.name };
    newFolderError.value = null;
    closeFolderMenu();
    newFolderOpen.value = true;
  }

  function closeNewFolder(): void {
    if (creatingFolder.value) return;
    newFolderOpen.value = false;
    newFolderTarget.value = null;
    newFolderError.value = null;
  }

  function clearNewFolderError(): void {
    newFolderError.value = null;
  }

  async function createFolder(name: string): Promise<boolean> {
    if (creatingFolder.value) return false;
    const target = newFolderTarget.value;
    if (!target) return false;

    const root = roots.value.find((candidate) => candidate.id === target.rootId);
    if (!root || !isPathInside(target.path, root.path)) {
      newFolderError.value = 'That folder is no longer part of the library.';
      return false;
    }
    const trimmed = name.trim();
    if (!trimmed) {
      newFolderError.value = 'Enter a folder name.';
      return false;
    }

    creatingFolder.value = true;
    newFolderError.value = null;
    try {
      const created = await api.createFolder(target.path, trimmed);
      // Refresh the parent listing immediately so the new row appears even
      // before the folder is scanned or selected.
      expanded.add(target.path);
      await loadChildren(target.path, true);

      // Counts in every ancestor now include the new (empty) folder.
      for (const key of [...folderImageCounts.keys()]) {
        if (isPathInside(key, root.path)) folderImageCounts.delete(key);
      }
      folderCountsEpoch.value += 1;

      // Close the dialog before selecting so a selection failure cannot leave
      // the form stuck in a loading state.
      creatingFolder.value = false;
      newFolderOpen.value = false;
      newFolderTarget.value = null;

      await selectFolder(created, target.rootId, { force: true });
      addToast(`Created ${basename(created)}.`, 'success');
      return true;
    } catch (error) {
      newFolderError.value = errorMessage(error);
      return false;
    } finally {
      creatingFolder.value = false;
    }
  }

  function fallbackFolderAfterRemoval(
    path: string,
    removedPaths: readonly string[],
  ): { path: string; rootId: string } | null {
    let candidate = parentPath(path);
    while (candidate && candidate !== path) {
      const covered = removedPaths.some((removed) => isPathInside(candidate, removed));
      if (!covered) {
        const root = roots.value.find((item) => isPathInside(candidate, item.path));
        if (root) return { path: candidate, rootId: root.id };
      }
      const next = parentPath(candidate);
      if (next === candidate) break;
      candidate = next;
    }
    const root = roots.value[0];
    return root ? { path: root.path, rootId: root.id } : null;
  }

  async function removeFolders(
    targets: readonly FolderRemovalTarget[],
    mode: RootRemovalMode,
  ): Promise<boolean> {
    if (removingFolder.value) return false;
    const pending = pruneFolderRemovalTargets(targets);
    if (!pending.length) return false;

    removingFolder.value = true;
    removeFolderError.value = null;
    const removedPaths: string[] = [];
    const failures: { target: FolderRemovalTarget; message: string }[] = [];
    const wasSelectedPath = selectedFolder.value;

    try {
      for (const target of pending) {
        try {
          if (target.kind === 'root') {
            // The backend moves the folder to Trash before forgetting it, so
            // the library entry survives if the filesystem operation fails.
            roots.value = await api.removeRoot(target.rootId, mode);
            forgetRootState(target.path);
            if (settings.value.lastRootId === target.rootId) {
              settings.value = { ...settings.value, lastRootId: null, lastFolder: null };
              queuePersist();
            }
          } else {
            await api.removeSubfolder(target.path, mode);
            forgetSubfolderState(target.path, target.rootId);
          }
          removedPaths.push(target.path);
        } catch (error) {
          failures.push({ target, message: errorMessage(error) });
        }
      }

      const removedSelected =
        !!wasSelectedPath &&
        removedPaths.some((removed) => isPathInside(wasSelectedPath, removed));
      if (removedSelected) {
        // Cancel in-flight scans and clear rows that belonged to removed folders.
        scanToken += 1;
        selectedRootId.value = null;
        selectedFolder.value = null;
        applyFolderSelection(emptySelection());
        applySelection(emptySelection());
        assets.value = [];
        warnings.value = [];
        scanState.value = 'idle';
        scanError.value = null;
        scanErrorCode.value = null;
        scanElapsedMs.value = 0;
      } else {
        // Keep the multiselection, minus any removed rows.
        const remaining = currentFolderSelection().ids.filter(
          (path) => !removedPaths.some((removed) => isPathInside(path, removed)),
        );
        const primaryId =
          selectedFolder.value && remaining.includes(selectedFolder.value)
            ? selectedFolder.value
            : (remaining[0] ?? null);
        const anchorId =
          folderSelectionAnchor.value && remaining.includes(folderSelectionAnchor.value)
            ? folderSelectionAnchor.value
            : primaryId;
        applyFolderSelection({ ids: remaining, primaryId, anchorId });
      }

      if (failures.length) {
        removeFolderTargets.value = failures.map((failure) => failure.target);
        removeFolderError.value = failures
          .map((failure) => `${failure.target.name}: ${failure.message}`)
          .join('\n');
        addToast(
          failures.length === 1
            ? 'One folder could not be removed.'
            : `${failures.length} folders could not be removed.`,
          'error',
        );
      } else {
        removeFolderOpen.value = false;
        removeFolderTargets.value = [];
      }

      if (removedPaths.length) {
        const label = removedPaths.length === 1 ? 'folder' : 'folders';
        addToast(
          mode === 'moveToTrash'
            ? `Moved ${removedPaths.length} ${label} to the Trash.`
            : `Removed ${removedPaths.length} ${label} from the library.`,
          'success',
          mode === 'moveToTrash' ? undefined : 7000,
        );
      }

      if (removedSelected) {
        const fallback = fallbackFolderAfterRemoval(wasSelectedPath, removedPaths);
        if (fallback) {
          const root = roots.value.find((candidate) => candidate.id === fallback.rootId);
          if (root?.available) {
            await selectFolder(fallback.path, root.id, { force: true });
          } else if (root) {
            selectedRootId.value = root.id;
            selectedFolder.value = fallback.path;
            scanState.value = 'error';
            scanError.value = `This folder is unavailable. ${root.error ?? ''}`.trim();
            scanErrorCode.value = 'notFound';
          }
        }
      }

      return failures.length === 0;
    } finally {
      removingFolder.value = false;
    }
  }

  function beginAssetDrag(id: string): string[] {
    if (!selectedIds.value.has(id)) select(id);
    const ids = assets.value
      .filter((asset) => selectedIds.value.has(asset.id))
      .map((asset) => asset.id);
    assetDragIds.value = ids;
    return ids;
  }

  function endAssetDrag(): void {
    assetDragIds.value = [];
    dragOverFolder.value = null;
  }

  function setDragOverFolder(path: string | null): void {
    dragOverFolder.value = path;
  }

  /**
   * Refresh everything an asset move/delete can invalidate: counts across all
   * roots, any expanded folder listings, and the currently displayed scan.
   */
  async function refreshAfterAssetMutation(folders: string[]): Promise<void> {
    for (const root of roots.value) {
      for (const key of [...folderImageCounts.keys()]) {
        if (isPathInside(key, root.path)) folderImageCounts.delete(key);
      }
    }
    folderCountsEpoch.value += 1;
    for (const folder of new Set(folders)) {
      if (tree.has(folder)) await loadChildren(folder, true);
    }
    if (selectedFolder.value) await scanFolder();
  }

  async function moveAssetsToFolder(destination: string): Promise<boolean> {
    if (movingAssets.value) return false;
    const ids = new Set(
      assetDragIds.value.length ? assetDragIds.value : [...selectedIds.value],
    );
    const targets = assets.value.filter((asset) => ids.has(asset.id));
    if (!targets.length) return false;

    movingAssets.value = true;
    try {
      const outcome = await api.moveAssets(targets, destination);
      if (!outcome.movedAssets) {
        addToast('Those photos are already in that folder.', 'info');
        return false;
      }
      focusIndexAfterScan = selectedIndex.value;
      const folders = new Set(targets.map((asset) => parentPath(asset.primaryPath)));
      folders.add(destination);
      await refreshAfterAssetMutation([...folders]);
      addToast(
        `Moved ${outcome.movedAssets} ${outcome.movedAssets === 1 ? 'photo' : 'photos'} to ${basename(destination)}.`,
        'success',
      );
      return true;
    } catch (error) {
      addToast(errorMessage(error), 'error');
      return false;
    } finally {
      movingAssets.value = false;
      endAssetDrag();
    }
  }

  function openDeleteAssets(): void {
    const targets = selectedAssets.value;
    if (!targets.length) return;
    closeFolderMenu();
    closePhotoMenu();
    deleteAssetsTargets.value = targets;
    removeAssetsError.value = null;
    deleteAssetsOpen.value = true;
  }

  function closeDeleteAssets(): void {
    if (removingAssets.value) return;
    deleteAssetsOpen.value = false;
    deleteAssetsTargets.value = [];
    removeAssetsError.value = null;
  }

  async function removeSelectedAssets(mode: AssetRemovalMode): Promise<boolean> {
    if (removingAssets.value) return false;
    const targets = deleteAssetsTargets.value;
    if (!targets.length) return false;

    removingAssets.value = true;
    removeAssetsError.value = null;
    focusIndexAfterScan = selectedIndex.value;
    try {
      const outcome = await api.removeAssets(targets, mode);
      hiddenAssets.value = outcome.hiddenAssets;
      deleteAssetsOpen.value = false;
      deleteAssetsTargets.value = [];
      const folders = [...new Set(targets.map((asset) => parentPath(asset.primaryPath)))];
      await refreshAfterAssetMutation(folders);
      addToast(
        mode === 'moveToTrash'
          ? `Moved ${outcome.removedAssets} ${outcome.removedAssets === 1 ? 'photo' : 'photos'} to the Trash.`
          : `Removed ${outcome.removedAssets} ${outcome.removedAssets === 1 ? 'photo' : 'photos'} from the library. Files are untouched; restore them in Settings → Hidden photos.`,
        'success',
        mode === 'moveToTrash' ? undefined : 8000,
      );
      return true;
    } catch (error) {
      removeAssetsError.value = errorMessage(error);
      addToast(errorMessage(error), 'error');
      return false;
    } finally {
      removingAssets.value = false;
    }
  }

  async function restoreHiddenAssets(ids: string[]): Promise<boolean> {
    if (!ids.length) return false;
    try {
      hiddenAssets.value = await api.restoreAssets(ids);
      await refreshAfterAssetMutation([]);
      addToast(
        `Restored ${ids.length} ${ids.length === 1 ? 'photo' : 'photos'} to the library.`,
        'success',
      );
      return true;
    } catch (error) {
      addToast(errorMessage(error), 'error');
      return false;
    }
  }

  async function refreshHiddenAssets(): Promise<void> {
    try {
      hiddenAssets.value = await api.getHiddenAssets();
    } catch (error) {
      console.error('Could not load hidden photos', error);
    }
  }

  async function refreshFolder(path?: string, rootId?: string): Promise<void> {
    const folder = path ?? selectedFolder.value;
    const root = roots.value.find(
      (candidate) => candidate.id === (rootId ?? selectedRootId.value),
    );
    if (!folder || !root || !isPathInside(folder, root.path)) return;

    // Counts under the root may have changed anywhere in the refreshed
    // subtree. Clear them first, then let visible rows re-request theirs.
    for (const key of [...folderImageCounts.keys()]) {
      if (isPathInside(key, root.path)) folderImageCounts.delete(key);
    }
    await loadChildren(folder, true);
    // Bump after the listing seeds this folder's children so visible rows only
    // request counts that are still missing from the cache.
    folderCountsEpoch.value += 1;

    // Only the displayed folder needs its assets rescanned.
    if (folder === selectedFolder.value) await scanFolder();
  }

  async function refresh(): Promise<void> {
    await refreshFolder();
  }

  async function clearThumbnailCache(): Promise<void> {
    if (clearingCache.value) return;
    clearingCache.value = true;
    try {
      await api.clearThumbnailCache();
      resetThumbnails();
      addToast('Thumbnail cache cleared. Previews will be rebuilt as needed.', 'success');
    } catch (error) {
      addToast(errorMessage(error), 'error');
    } finally {
      clearingCache.value = false;
    }
  }

  async function initialize(): Promise<void> {
    if (bootstrapped.value) return;
    initializing.value = true;
    initializationError.value = null;
    try {
      const bootstrap = await api.bootstrap();
      roots.value = bootstrap.roots;
      settings.value = normalizeUiSettings(bootstrap.ui);
      ingestSettings.value = normalizeIngestSettings(bootstrap.ingest);
      cacheDir.value = bootstrap.cacheDir;
      recoveryWarning.value = bootstrap.recoveryWarning;
      appVersion.value = bootstrap.appVersion;
      hiddenAssets.value = bootstrap.hiddenAssets ?? [];
      try {
        localStorage.setItem('lumio.theme', settings.value.theme);
      } catch {
        /* ignore */
      }
      setupSystemTheme();
      bootstrapped.value = true;

      if (bootstrap.recoveryWarning) {
        addToast(bootstrap.recoveryWarning, 'error', 10_000);
      }

      const lastFolder = bootstrap.ui.lastFolder;
      const lastRootId = bootstrap.ui.lastRootId;
      if (lastFolder && lastRootId) {
        const root = bootstrap.roots.find((candidate) => candidate.id === lastRootId);
        if (root && isPathInside(lastFolder, root.path)) {
          selectedRootId.value = root.id;
          selectedFolder.value = lastFolder;
          if (root.available) {
            await selectFolder(lastFolder, root.id, {
              expandAncestors: true,
              force: true,
            });
          } else {
            scanState.value = 'error';
            scanError.value = `This folder is unavailable. ${root.error ?? ''}`.trim();
            scanErrorCode.value = 'notFound';
          }
        }
      }
    } catch (error) {
      initializationError.value = errorMessage(error);
    } finally {
      initializing.value = false;
    }
  }

  function setupSystemTheme(): void {
    if (typeof window === 'undefined' || !window.matchMedia) return;
    const media = window.matchMedia('(prefers-color-scheme: dark)');
    systemDark.value = media.matches;
    media.addEventListener('change', (event) => {
      systemDark.value = event.matches;
    });
  }

  return {
    // lifecycle
    initializing,
    bootstrapped,
    initializationError,
    initialize,
    // library data
    roots,
    tree,
    treeLoading,
    treeErrors,
    expanded,
    selectedRoot,
    selectedRootId,
    selectedFolder,
    folderName,
    relativeFolderLabel,
    breadcrumb,
    hasRoots,
    // scanning
    assets,
    assetIndex,
    warnings,
    warningsCount,
    scanState,
    scanError,
    scanErrorCode,
    scanElapsedMs,
    scanRecursive,
    stats,
    selectFolder,
    scanFolder,
    refresh,
    refreshFolder,
    addRoot,
    registerRoot,
    ingestSettings,
    saveIngestSettings,
    removeFolders,
    addingRoot,
    removeFolderOpen,
    removeFolderTargets,
    removingFolder,
    removeFolderError,
    openRemoveFolder,
    openDeleteFolder,
    openRemoveFolders,
    openRemoveSelectedFolders,
    removeFolderFromRow,
    closeRemoveFolder,
    folderMenuOpen,
    folderMenuTarget,
    openFolderMenu,
    closeFolderMenu,
    removeFromFolderMenu,
    refreshFromFolderMenu,
    photoMenuOpen,
    photoMenuTarget,
    openPhotoMenu,
    closePhotoMenu,
    deleteFromPhotoMenu,
    newFolderOpen,
    newFolderTarget,
    creatingFolder,
    newFolderError,
    openNewFolder,
    closeNewFolder,
    clearNewFolderError,
    createFolder,
    // tree
    loadChildren,
    toggleExpanded,
    imageCountFor,
    ensureFolderImageCount,
    folderCountsEpoch,
    // folder selection
    folderRows,
    folderOrder,
    selectedFolderPaths,
    selectedFolderCount,
    selectedFolderTargets,
    isFolderSelected,
    selectFolderRow,
    clearFolderSelection,
    // photo selection
    selectionId,
    selectedIds,
    selectedCount,
    selectedAssets,
    selectedAsset,
    selectedIndex,
    isSelected,
    select,
    selectWithModifiers,
    selectAll,
    clearSelection,
    navigateHorizontal,
    navigateVertical,
    navRowSize,
    // asset operations & drag/drop
    hiddenAssets,
    deleteAssetsOpen,
    deleteAssetsTargets,
    removingAssets,
    removeAssetsError,
    openDeleteAssets,
    closeDeleteAssets,
    removeSelectedAssets,
    restoreHiddenAssets,
    refreshHiddenAssets,
    assetDragIds,
    dragOverFolder,
    movingAssets,
    beginAssetDrag,
    endAssetDrag,
    setDragOverFolder,
    moveAssetsToFolder,
    // fullscreen & dialogs
    fullscreenOpen,
    openFullscreen,
    closeFullscreen,
    toggleFullscreen,
    openInFilmstrip,
    settingsOpen,
    openSettings,
    closeSettings,
    shortcutsOpen,
    openShortcuts,
    closeShortcuts,
    warningsOpen,
    openWarnings,
    closeWarnings,
    infoVisible,
    toggleInfo,
    // settings
    settings,
    viewMode,
    setTheme,
    toggleTheme,
    setUiFont,
    setMonoFont,
    setUiFontSize,
    setViewMode,
    setThumbnailSize,
    setRecursive,
    setFilmstripVisible,
    toggleFilmstrip,
    toggleListAssociations,
    externalEditors,
    defaultEditor,
    addingEditor,
    addExternalEditor,
    setDefaultEditor,
    removeExternalEditor,
    editSelectionWithApplication,
    editSelectionWithDefault,
    // metadata
    metadataBusy,
    metadataError,
    applyMetadataPatch,
    setRating,
    setColorLabel,
    setCull,
    unflagSelection,
    toggleFavorite,
    shortcutBinding,
    shortcutLabel,
    shortcutIsCustom,
    shortcutMatches,
    setShortcut,
    resetShortcut,
    resetShortcuts,
    updateSettings,
    resolvedTheme,
    cacheDir,
    appVersion,
    clearThumbnailCache,
    clearingCache,
    recoveryWarning,
    recoveryWarningDismissed,
    dismissRecoveryWarning,
    // toasts
    toasts,
    addToast,
    dismissToast,
  };
});
