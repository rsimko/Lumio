import { open as openDialog } from '@tauri-apps/plugin-dialog';
import { defineStore } from 'pinia';
import { computed, reactive, ref, shallowRef, watch } from 'vue';

import { api, errorCode, errorMessage } from '@/api';
import { resetThumbnails } from '@/composables/useThumbnails';
import { basename, isPathInside } from '@/lib/format';
import type {
  FolderNode,
  PhotoAsset,
  RootInfo,
  ScanWarning,
  ThemeSetting,
  UiSettings,
  ViewMode,
} from '@/types';

export const DEFAULT_UI_SETTINGS: UiSettings = {
  theme: 'system',
  viewMode: 'grid',
  recursive: false,
  thumbnailSize: 184,
  libraryWidth: 276,
  infoWidth: 320,
  lastRootId: null,
  lastFolder: null,
  filmstripVisible: true,
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

  async function loadChildren(path: string, force = false): Promise<void> {
    if (treeLoading.has(path)) return;
    if (!force && tree.has(path)) return;
    treeLoading.add(path);
    treeErrors.delete(path);
    try {
      const children = await api.listSubfolders(path);
      tree.set(path, children);
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

  async function removeRoot(id: string): Promise<void> {
    const root = roots.value.find((candidate) => candidate.id === id);
    try {
      const updated = await api.removeRoot(id);
      roots.value = updated;
      if (root) {
        expanded.delete(root.path);
        tree.delete(root.path);
        treeErrors.delete(root.path);
      }
      if (selectedRootId.value === id) {
        selectedRootId.value = null;
        selectedFolder.value = null;
        assets.value = [];
        warnings.value = [];
        selectionId.value = null;
        scanState.value = 'idle';
        scanError.value = null;
        scanErrorCode.value = null;
        scanElapsedMs.value = 0;
      }
      addToast(
        root ? `Removed ${root.name} from the library.` : 'Folder removed from the library.',
        'success',
      );
    } catch (error) {
      addToast(errorMessage(error), 'error');
    }
  }

  async function refresh(): Promise<void> {
    const folder = selectedFolder.value;
    if (!folder) return;
    await loadChildren(folder, true);
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
      settings.value = { ...DEFAULT_UI_SETTINGS, ...bootstrap.ui };
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
    // tree
    loadChildren,
    toggleExpanded,
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
