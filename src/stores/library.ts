import { open as openDialog } from '@tauri-apps/plugin-dialog';
import { defineStore } from 'pinia';
import { computed, reactive, ref, shallowRef, watch } from 'vue';

import { api, errorCode, errorMessage } from '@/api';
import { resetThumbnails } from '@/composables/useThumbnails';
import { basename, isPathInside, parentPath } from '@/lib/format';
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
  AssetRemovalMode,
  ExcludedAsset,
  FolderImageCounts,
  FolderMenuTarget,
  FolderNode,
  FolderRemovalTarget,
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
  shortcuts: {},
};

const BASE_FONT_SIZE = 13;

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
  return {
    ...merged,
    uiFont: merged.uiFont === 'system' ? 'system' : 'avenir',
    monoFont: merged.monoFont === 'system' ? 'system' : 'jetbrains',
    uiFontSize: Math.min(17, Math.max(11, size)),
    shortcuts: normalizeShortcutOverrides(merged.shortcuts),
  };
}

export const useLibraryStore = defineStore('library', () => {
  const initializing = ref(true);
  const bootstrapped = ref(false);
  const initializationError = ref<string | null>(null);

  const roots = ref<RootInfo[]>([]);
  const settings = ref<UiSettings>({ ...DEFAULT_UI_SETTINGS });
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
  const removeFolderOpen = ref(false);
  const removeFolderTarget = ref<FolderRemovalTarget | null>(null);
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
  const toasts = ref<Toast[]>([]);

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
      const outcome = await api.addRoot(picked);
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
      await selectFolder(target, info.id);
      addToast(
        outcome.restored
          ? `Restored ${basename(target)} to the library.`
          : `Added ${info.name} to the library.`,
        'success',
      );
    } catch (error) {
      addToast(errorMessage(error), 'error');
    } finally {
      addingRoot.value = false;
    }
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
    removeFolderTarget.value = {
      kind: 'root',
      rootId: root.id,
      name: root.name,
      path: root.path,
    };
    removeFolderOpen.value = true;
    removeFolderError.value = null;
  }

  function openDeleteFolder(path: string, rootId: string): void {
    const root = roots.value.find((candidate) => candidate.id === rootId);
    if (!root || !isPathInside(path, root.path) || path === root.path) return;
    removeFolderTarget.value = {
      kind: 'folder',
      rootId,
      name: basename(path),
      path,
    };
    removeFolderOpen.value = true;
    removeFolderError.value = null;
  }

  function closeRemoveFolder(): void {
    if (removingFolder.value) return;
    removeFolderOpen.value = false;
    removeFolderTarget.value = null;
    removeFolderError.value = null;
  }

  function openFolderMenu(target: Omit<FolderMenuTarget, 'x' | 'y'>, x: number, y: number): void {
    closePhotoMenu();
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

  async function removeRoot(id: string, mode: RootRemovalMode): Promise<boolean> {
    if (removingFolder.value) return false;
    const index = roots.value.findIndex((candidate) => candidate.id === id);
    const root = roots.value[index];
    if (!root) return false;

    removingFolder.value = true;
    removeFolderError.value = null;
    try {
      // The backend moves a folder to Trash before forgetting it, so the
      // library entry survives if the filesystem operation fails.
      const updated = await api.removeRoot(id, mode);
      roots.value = updated;
      forgetRootState(root.path);

      if (settings.value.lastRootId === id) {
        settings.value = { ...settings.value, lastRootId: null, lastFolder: null };
        queuePersist();
      }

      const wasSelected = selectedRootId.value === id;
      if (wasSelected) {
        // Cancel in-flight scans and clear rows that belonged to the root.
        scanToken += 1;
        selectedRootId.value = null;
        selectedFolder.value = null;
        applySelection(emptySelection());
        assets.value = [];
        warnings.value = [];
        scanState.value = 'idle';
        scanError.value = null;
        scanErrorCode.value = null;
        scanElapsedMs.value = 0;
      }

      // Close the confirmation first so it cannot be submitted twice.
      removeFolderOpen.value = false;
      removeFolderTarget.value = null;
      removingFolder.value = false;

      addToast(
        mode === 'moveToTrash'
          ? `Moved ${root.name} to the Trash.`
          : `Removed ${root.name} from the library.`,
        'success',
      );

      if (wasSelected) {
        const next = updated[Math.min(index, updated.length - 1)] ?? updated[0];
        if (next) {
          if (next.available) {
            await selectFolder(next.path, next.id, { force: true });
          } else {
            selectedRootId.value = next.id;
            selectedFolder.value = next.path;
            scanState.value = 'error';
            scanError.value = `This folder is unavailable. ${next.error ?? ''}`.trim();
            scanErrorCode.value = 'notFound';
          }
        }
      }

      return true;
    } catch (error) {
      removeFolderError.value = errorMessage(error);
      addToast(errorMessage(error), 'error');
      return false;
    } finally {
      removingFolder.value = false;
    }
  }

  async function removeSubfolder(
    path: string,
    rootId: string,
    mode: RootRemovalMode,
  ): Promise<boolean> {
    if (removingFolder.value) return false;
    const root = roots.value.find((candidate) => candidate.id === rootId);
    if (!root || !isPathInside(path, root.path) || path === root.path) return false;

    removingFolder.value = true;
    removeFolderError.value = null;
    try {
      await api.removeSubfolder(path, mode);
      forgetSubfolderState(path, rootId);

      // Close the confirmation before selecting the fallback folder.
      removeFolderOpen.value = false;
      removeFolderTarget.value = null;
      removingFolder.value = false;

      const wasSelected = !!selectedFolder.value && isPathInside(selectedFolder.value, path);
      if (wasSelected) {
        await selectFolder(parentPath(path), rootId, { force: true });
      }

      addToast(
        mode === 'moveToTrash'
          ? `Moved ${basename(path)} to the Trash.`
          : `Removed ${basename(path)} from the library. Files are untouched; add it again to restore it.`,
        'success',
        mode === 'moveToTrash' ? undefined : 7000,
      );
      return true;
    } catch (error) {
      removeFolderError.value = errorMessage(error);
      addToast(errorMessage(error), 'error');
      return false;
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
    removeRoot,
    removeSubfolder,
    addingRoot,
    removeFolderOpen,
    removeFolderTarget,
    removingFolder,
    removeFolderError,
    openRemoveFolder,
    openDeleteFolder,
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
    // selection
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
