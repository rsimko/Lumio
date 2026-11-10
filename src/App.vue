<script setup lang="ts">
import { onBeforeUnmount, onMounted } from 'vue';

import Filmstrip from '@/components/Filmstrip.vue';
import FullscreenViewer from '@/components/FullscreenViewer.vue';
import InfoPanel from '@/components/InfoPanel.vue';
import LibraryPanel from '@/components/LibraryPanel.vue';
import MainArea from '@/components/MainArea.vue';
import RailTabs from '@/components/RailTabs.vue';
import SettingsDialog from '@/components/SettingsDialog.vue';
import ShortcutsDialog from '@/components/ShortcutsDialog.vue';
import StatusBar from '@/components/StatusBar.vue';
import ToastHost from '@/components/ToastHost.vue';
import TopToolbar from '@/components/TopToolbar.vue';
import WarningsDialog from '@/components/WarningsDialog.vue';
import { useLibraryStore } from '@/stores/library';

const store = useLibraryStore();

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

  const modifier = event.metaKey || event.ctrlKey;

  if (modifier) {
    switch (event.key.toLowerCase()) {
      case 'o':
        event.preventDefault();
        void store.addRoot();
        return;
      case 'r':
        event.preventDefault();
        void store.refresh();
        return;
      case ',':
        event.preventDefault();
        store.openSettings();
        return;
      default:
        return;
    }
  }

  if (event.key === 'Escape') {
    event.preventDefault();
    if (store.fullscreenOpen) store.closeFullscreen();
    else if (store.settingsOpen) store.closeSettings();
    else if (store.shortcutsOpen) store.closeShortcuts();
    else if (store.warningsOpen) store.closeWarnings();
    else if (store.selectionId) store.select(null);
    return;
  }

  if (store.settingsOpen || store.shortcutsOpen || store.warningsOpen) return;

  if (store.fullscreenOpen) {
    switch (event.key) {
      case 'ArrowLeft':
        event.preventDefault();
        store.navigateHorizontal(-1);
        break;
      case 'ArrowRight':
        event.preventDefault();
        store.navigateHorizontal(1);
        break;
      case ' ':
      case 'Enter':
        event.preventDefault();
        store.closeFullscreen();
        break;
      default:
        break;
    }
    return;
  }

  switch (event.key) {
    case 'ArrowLeft':
      event.preventDefault();
      store.navigateHorizontal(-1);
      break;
    case 'ArrowRight':
      event.preventDefault();
      store.navigateHorizontal(1);
      break;
    case 'ArrowUp':
      event.preventDefault();
      store.navigateVertical(-1);
      break;
    case 'ArrowDown':
      event.preventDefault();
      store.navigateVertical(1);
      break;
    case ' ':
    case 'Enter':
      if (store.assets.length) {
        event.preventDefault();
        store.openFullscreen();
      }
      break;
    case '+':
    case '=':
      event.preventDefault();
      store.setThumbnailSize(store.settings.thumbnailSize + 24);
      break;
    case '-':
    case '_':
      event.preventDefault();
      store.setThumbnailSize(store.settings.thumbnailSize - 24);
      break;
    case '0':
      event.preventDefault();
      store.setThumbnailSize(184);
      break;
    case '?':
      event.preventDefault();
      store.openShortcuts();
      break;
    default:
      break;
  }

  switch (event.key.toLowerCase()) {
    case 'g':
      store.setViewMode('grid');
      break;
    case 'm':
      store.setViewMode('masonry');
      break;
    case 'j':
      store.setViewMode('justified');
      break;
    case 'l':
      store.setViewMode('list');
      break;
    case 'f':
      store.setViewMode('filmstrip');
      break;
    case 'r':
      store.setRecursive(!store.settings.recursive);
      break;
    case 'b':
      store.toggleFilmstrip();
      break;
    case 'i':
      store.toggleInfo();
      break;
    case 't':
      store.toggleTheme();
      break;
    default:
      break;
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
          v-if="store.settings.filmstripVisible && store.assets.length > 0"
        />
        <StatusBar />
      </section>
      <InfoPanel v-if="store.infoVisible" />
    </div>

    <FullscreenViewer v-if="store.fullscreenOpen" />
    <SettingsDialog v-if="store.settingsOpen" />
    <ShortcutsDialog v-if="store.shortcutsOpen" />
    <WarningsDialog v-if="store.warningsOpen" />
    <ToastHost />
  </div>
</template>
