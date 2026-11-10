import { open as openDialog } from '@tauri-apps/plugin-dialog';
import { defineStore } from 'pinia';
import { computed, reactive, ref, shallowRef, watch } from 'vue';

import { api, errorCode, errorMessage } from '@/api';
import { resetThumbnails } from '@/composables/useThumbnails';
import { basename, isPathInside } from '@/lib/format';
import type {
  FolderImageCounts,
  FolderNode,
  MonoFont,
  PhotoAsset,
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

  const selectionId = ref<string | null>(null);
  const fullscreenOpen = ref(false);
  const settingsOpen = ref(false);
  const warningsOpen = ref(false);
  const shortcutsOpen = ref(false);
  const infoVisible = ref(true);
  const addingRoot = ref(false);
  const removeFolderOpen = ref(false);
  const removeFolderTarget = ref<RootInfo | null>(null);
  const removingFolder = ref(false);
  const removeFolderError = ref<string | null>(null);
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

  const selectedAsset = computed(() => {
    if (!selectionId.value) return null;
    return assetIndex.value.get(selectionId.value) ?? null;
  });

  const selectedIndex = computed(() => {
    if (!selectionId.value) return -1;
    return assets.value.findIndex((asset) => asset.id === selectionId.value);
  });

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
    // The filmstrip view needs its strip visible to make sense.
    updateSettings({ viewMode: mode, filmstripVisible: mode === 'filmstrip' ? true : settings.value.filmstripVisible });
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
    updateSettings({ filmstripVisible: value });
  }

  function toggleFilmstrip(): void {
    setFilmstripVisible(!settings.value.filmstripVisible);
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
    selectionId.value = id;
  }

  function navigateBy(offset: number): void {
    const list = assets.value;
    if (!list.length) return;
    const current = selectedIndex.value;
    if (current < 0) {
      selectionId.value = list[0].id;
      return;
    }
    const next = Math.min(list.length - 1, Math.max(0, current + offset));
    selectionId.value = list[next].id;
  }

  function navigateHorizontal(delta: number): void {
    navigateBy(delta);
  }

  function navigateVertical(delta: number): void {
    navigateBy(delta * Math.max(1, navRowSize.value));
  }

  function openInFilmstrip(id?: string): void {
    if (id) selectionId.value = id;
    if (!selectionId.value && assets.value.length) {
      selectionId.value = assets.value[0]?.id ?? null;
    }
    if (!selectionId.value) return;
    // Opening from any browsing layout switches to the filmstrip view and makes
    // sure its strip is visible so the selected photo has context.
    setViewMode('filmstrip');
  }

  function openFullscreen(id?: string): void {
    if (id) selectionId.value = id;
    if (!selectionId.value && assets.value.length) {
      selectionId.value = assets.value[0]?.id ?? null;
    }
    if (!selectionId.value) return;
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
      const previous = selectionId.value;
      if (previous && result.assets.some((asset) => asset.id === previous)) {
        selectionId.value = previous;
      } else {
        selectionId.value = result.assets[0]?.id ?? null;
      }
    } catch (error) {
      if (token !== scanToken) return;
      assets.value = [];
      warnings.value = [];
      selectionId.value = null;
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
    selectionId.value = null;
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
      const info = await api.addRoot(picked);
      roots.value = roots.value.some((root) => root.id === info.id)
        ? roots.value.map((root) => (root.id === info.id ? info : root))
        : [...roots.value, info];
      for (const key of [...folderImageCounts.keys()]) {
        if (isPathInside(key, info.path)) folderImageCounts.delete(key);
      }
      folderCountsEpoch.value += 1;
      expanded.add(info.path);
      await loadChildren(info.path, true);
      await selectFolder(info.path, info.id);
      addToast(`Added ${info.name} to the library.`, 'success');
    } catch (error) {
      addToast(errorMessage(error), 'error');
    } finally {
      addingRoot.value = false;
    }
  }

  function forgetRootState(rootPath: string): void {
    const inside = (path: string) => isPathInside(path, rootPath);
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
    folderCountsEpoch.value += 1;
  }

  function openRemoveFolder(id: string): void {
    const root = roots.value.find((candidate) => candidate.id === id);
    if (!root) return;
    removeFolderTarget.value = root;
    removeFolderOpen.value = true;
    removeFolderError.value = null;
  }

  function closeRemoveFolder(): void {
    if (removingFolder.value) return;
    removeFolderOpen.value = false;
    removeFolderTarget.value = null;
    removeFolderError.value = null;
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
        selectionId.value = null;
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

  async function refresh(): Promise<void> {
    const folder = selectedFolder.value;
    if (!folder) return;
    folderImageCounts.clear();
    await loadChildren(folder, true);
    // Bump after the listing seeds this folder's children so visible rows only
    // request counts that are still missing from the cache.
    folderCountsEpoch.value += 1;
    await scanFolder();
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
    addRoot,
    removeRoot,
    addingRoot,
    removeFolderOpen,
    removeFolderTarget,
    removingFolder,
    removeFolderError,
    openRemoveFolder,
    closeRemoveFolder,
    // tree
    loadChildren,
    toggleExpanded,
    imageCountFor,
    ensureFolderImageCount,
    folderCountsEpoch,
    // selection
    selectionId,
    selectedAsset,
    selectedIndex,
    select,
    navigateHorizontal,
    navigateVertical,
    navRowSize,
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
