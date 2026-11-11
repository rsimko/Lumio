<script setup lang="ts">
import { onBeforeUnmount, onMounted } from 'vue';

import Filmstrip from '@/components/Filmstrip.vue';
import DeleteAssetsDialog from '@/components/DeleteAssetsDialog.vue';
import FolderContextMenu from '@/components/FolderContextMenu.vue';
import FullscreenViewer from '@/components/FullscreenViewer.vue';
import InfoPanel from '@/components/InfoPanel.vue';
import IngestDialog from '@/components/IngestDialog.vue';
import LibraryPanel from '@/components/LibraryPanel.vue';
import MainArea from '@/components/MainArea.vue';
import NewFolderDialog from '@/components/NewFolderDialog.vue';
import PhotoContextMenu from '@/components/PhotoContextMenu.vue';
import RailTabs from '@/components/RailTabs.vue';
import RemoveFolderDialog from '@/components/RemoveFolderDialog.vue';
import SettingsDialog from '@/components/SettingsDialog.vue';
import ShortcutsDialog from '@/components/ShortcutsDialog.vue';
import StatusBar from '@/components/StatusBar.vue';
import ToastHost from '@/components/ToastHost.vue';
import TopToolbar from '@/components/TopToolbar.vue';
import WarningsDialog from '@/components/WarningsDialog.vue';
import { useLibraryStore } from '@/stores/library';
import { useIngestStore } from '@/stores/ingest';
import type { ShortcutId } from '@/lib/shortcuts';

const store = useLibraryStore();
const ingest = useIngestStore();

interface BrowseShortcut {
  id: ShortcutId;
  run: () => void;
}

const browseShortcuts: BrowseShortcut[] = [
  {
    id: 'browse.fullscreen',
    run: () => {
      if (store.assets.length) store.openFullscreen();
    },
  },
  {
    id: 'browse.delete',
    run: () => {
      if (store.selectedCount > 0) store.openDeleteAssets();
    },
  },
  { id: 'browse.editDefault', run: () => store.editSelectionWithDefault() },
  {
    id: 'browse.thumbLarger',
    run: () => store.setThumbnailSize(store.settings.thumbnailSize + 24),
  },
  {
    id: 'browse.thumbSmaller',
    run: () => store.setThumbnailSize(store.settings.thumbnailSize - 24),
  },
  { id: 'browse.thumbReset', run: () => store.setThumbnailSize(184) },
  { id: 'view.grid', run: () => store.setViewMode('grid') },
  { id: 'view.masonry', run: () => store.setViewMode('masonry') },
  { id: 'view.justified', run: () => store.setViewMode('justified') },
  { id: 'view.list', run: () => store.setViewMode('list') },
  { id: 'view.filmstrip', run: () => store.setViewMode('filmstrip') },
  { id: 'library.addFolder', run: () => void store.addRoot() },
  { id: 'library.refresh', run: () => void store.refresh() },
  { id: 'library.selectAll', run: () => store.selectAll() },
  { id: 'library.recursive', run: () => store.setRecursive(!store.settings.recursive) },
  { id: 'library.filmstrip', run: () => store.toggleFilmstrip() },
  { id: 'library.info', run: () => store.toggleInfo() },
  { id: 'library.theme', run: () => store.toggleTheme() },
  { id: 'library.settings', run: () => store.openSettings() },
  { id: 'help.shortcuts', run: () => store.openShortcuts() },
  {
    id: 'metadata.rating1',
    run: () => store.setRating(1),
  },
  {
    id: 'metadata.rating2',
    run: () => store.setRating(2),
  },
  {
    id: 'metadata.rating3',
    run: () => store.setRating(3),
  },
  {
    id: 'metadata.rating4',
    run: () => store.setRating(4),
  },
  {
    id: 'metadata.rating5',
    run: () => store.setRating(5),
  },
  { id: 'metadata.pick', run: () => store.setCull('pick') },
  { id: 'metadata.reject', run: () => store.setCull('reject') },
  { id: 'metadata.unflag', run: () => store.unflagSelection() },
  { id: 'metadata.favorite', run: () => store.toggleFavorite() },
];

/** Metadata keys work in both browse and the fullscreen viewer. */
const metadataShortcuts: BrowseShortcut[] = browseShortcuts.filter((shortcut) =>
  shortcut.id.startsWith('metadata.'),
);

function isTypingTarget(target: EventTarget | null): boolean {
  if (!(target instanceof HTMLElement)) return false;
  return (
    target.tagName === 'INPUT' ||
    target.tagName === 'TEXTAREA' ||
    target.tagName === 'SELECT' ||
    target.isContentEditable
  );
}

function onKeydown(event: KeyboardEvent) {
  if (isTypingTarget(event.target)) {
    if (event.key === 'Escape') (event.target as HTMLElement).blur();
    return;
  }

  // The removal dialog owns the keyboard while it is open, especially the
  // confirmation field. Only Escape is handled here to dismiss it safely.
  if (store.removeFolderOpen) {
    if (event.key === 'Escape') {
      event.preventDefault();
      store.closeRemoveFolder();
    }
    return;
  }

  // The new-folder dialog owns the keyboard while it is open. The input's
  // Escape handler stops propagation, but the footer buttons still land here.
  if (store.newFolderOpen) {
    if (event.key === 'Escape') {
      event.preventDefault();
      store.closeNewFolder();
    }
    return;
  }

  if (store.folderMenuOpen) {
    if (event.key === 'Escape') {
      event.preventDefault();
      store.closeFolderMenu();
    }
    return;
  }

  if (store.photoMenuOpen) {
    if (event.key === 'Escape') {
      event.preventDefault();
      store.closePhotoMenu();
    }
    return;
  }

  if (store.deleteAssetsOpen) {
    if (event.key === 'Escape') {
      event.preventDefault();
      store.closeDeleteAssets();
    }
    return;
  }

  if (store.settingsOpen || store.shortcutsOpen || store.warningsOpen) {
    if (event.key === 'Escape') {
      event.preventDefault();
      if (store.settingsOpen) store.closeSettings();
      else if (store.shortcutsOpen) store.closeShortcuts();
      else store.closeWarnings();
    }
    return;
  }

  if (ingest.open) {
    if (event.key === 'Escape') {
      event.preventDefault();
      ingest.closeDialog();
    }
    return;
  }

  if (event.key === 'Escape') {
    event.preventDefault();
    if (store.fullscreenOpen) store.closeFullscreen();
    else if (store.selectionId) store.clearSelection();
    return;
  }

  if (store.fullscreenOpen) {
    if (store.shortcutMatches(event, 'viewer.previous')) {
      event.preventDefault();
      store.navigateHorizontal(-1);
    } else if (store.shortcutMatches(event, 'viewer.next')) {
      event.preventDefault();
      store.navigateHorizontal(1);
    } else if (store.shortcutMatches(event, 'viewer.close')) {
      event.preventDefault();
      store.closeFullscreen();
    } else if (event.key === 'Delete' || event.key === 'Backspace') {
      event.preventDefault();
      store.closeFullscreen();
      store.openDeleteAssets();
    } else {
      for (const shortcut of metadataShortcuts) {
        if (!store.shortcutMatches(event, shortcut.id)) continue;
        event.preventDefault();
        shortcut.run();
        break;
      }
    }
    return;
  }

  // Arrow navigation is intentionally fixed: Shift extends the selection.
  if (!event.metaKey && !event.ctrlKey && !event.altKey) {
    switch (event.key) {
      case 'ArrowLeft':
        event.preventDefault();
        store.navigateHorizontal(-1, event.shiftKey);
        return;
      case 'ArrowRight':
        event.preventDefault();
        store.navigateHorizontal(1, event.shiftKey);
        return;
      case 'ArrowUp':
        event.preventDefault();
        store.navigateVertical(-1, event.shiftKey);
        return;
      case 'ArrowDown':
        event.preventDefault();
        store.navigateVertical(1, event.shiftKey);
        return;
      default:
        break;
    }
  }

  for (const shortcut of browseShortcuts) {
    if (!store.shortcutMatches(event, shortcut.id)) continue;
    event.preventDefault();
    shortcut.run();
    return;
  }
}

onMounted(() => {
  window.addEventListener('keydown', onKeydown);
  void store.initialize();
});

onBeforeUnmount(() => {
  window.removeEventListener('keydown', onKeydown);
});
</script>

<template>
  <div class="flex h-full min-h-0 flex-col overflow-hidden bg-app text-ink">
    <TopToolbar />
    <div class="flex min-h-0 flex-1">
      <RailTabs />
      <LibraryPanel />
      <section class="flex min-w-0 flex-1 flex-col bg-main">
        <MainArea class="min-h-0 flex-1" />
        <Filmstrip
          v-if="
            store.viewMode === 'filmstrip' &&
            store.settings.filmstripVisible &&
            store.assets.length > 0
          "
        />
        <StatusBar />
      </section>
      <InfoPanel v-if="store.infoVisible" />
    </div>

    <FullscreenViewer v-if="store.fullscreenOpen" />
    <SettingsDialog v-if="store.settingsOpen" />
    <ShortcutsDialog v-if="store.shortcutsOpen" />
    <WarningsDialog v-if="store.warningsOpen" />
    <FolderContextMenu v-if="store.folderMenuOpen" />
    <PhotoContextMenu v-if="store.photoMenuOpen" />
    <NewFolderDialog v-if="store.newFolderOpen" />
    <RemoveFolderDialog v-if="store.removeFolderOpen" />
    <DeleteAssetsDialog v-if="store.deleteAssetsOpen" />
    <IngestDialog v-if="ingest.open" />
    <ToastHost />
  </div>
</template>
