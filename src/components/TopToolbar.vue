<script setup lang="ts">
import {
  Aperture,
  ChevronRight,
  Columns3,
  Film,
  FolderOpen,
  FolderPlus,
  FolderTree,
  LayoutGrid,
  List,
  Loader2,
  Moon,
  PanelRight,
  RefreshCw,
  Rows3,
  Settings,
  Sun,
} from 'lucide-vue-next';
import { computed } from 'vue';

import { useLibraryStore, type BreadcrumbItem } from '@/stores/library';
import type { ViewMode } from '@/types';

const store = useLibraryStore();

const views = [
  { mode: 'grid' as ViewMode, label: 'Grid', hint: 'Grid (G)', icon: LayoutGrid },
  { mode: 'masonry' as ViewMode, label: 'Masonry', hint: 'Masonry (M)', icon: Columns3 },
  { mode: 'justified' as ViewMode, label: 'Justified', hint: 'Justified (J)', icon: Rows3 },
  { mode: 'list' as ViewMode, label: 'List', hint: 'List (L)', icon: List },
  { mode: 'filmstrip' as ViewMode, label: 'Filmstrip', hint: 'Filmstrip (F)', icon: Film },
];

const sizeSliderVisible = computed(() =>
  ['grid', 'masonry', 'justified'].includes(store.viewMode),
);

function selectCrumb(crumb: BreadcrumbItem) {
  if (!store.selectedRootId) return;
  void store.selectFolder(crumb.path, store.selectedRootId);
}

function onSizeInput(event: Event) {
  const value = Number((event.target as HTMLInputElement).value);
  if (Number.isFinite(value)) store.setThumbnailSize(value);
}
</script>

<template>
  <header
    class="flex h-[52px] shrink-0 items-center gap-2 border-b border-line bg-panel px-3"
  >
    <div class="flex shrink-0 items-center gap-2 pr-0.5">
      <div
        class="grid h-7 w-7 place-items-center rounded-md bg-accent-soft text-accent ring-1 ring-accent/15"
      >
        <Aperture class="h-4 w-4" />
      </div>
      <span class="text-[14px] font-semibold tracking-[0.01em] text-ink">Lumio</span>
    </div>

    <div class="mx-0.5 h-5 w-px shrink-0 bg-line" />

    <nav class="flex min-w-0 flex-1 items-center gap-0.5 overflow-hidden text-xs" aria-label="Folder path">
      <FolderOpen class="mr-0.5 h-3.5 w-3.5 shrink-0 text-subtle" />
      <template v-if="store.breadcrumb.length">
        <template v-for="(crumb, index) in store.breadcrumb" :key="crumb.path">
          <button
            class="max-w-[150px] truncate rounded px-1 py-1 transition hover:bg-panel-2"
            :class="
              index === store.breadcrumb.length - 1
                ? 'font-medium text-ink'
                : 'text-muted hover:text-ink'
            "
            :title="crumb.path"
            @click="selectCrumb(crumb)"
          >
            {{ crumb.name }}
          </button>
          <ChevronRight
            v-if="index < store.breadcrumb.length - 1"
            class="h-3 w-3 shrink-0 text-subtle/70"
          />
        </template>
      </template>
      <span v-else class="truncate text-muted">No folder selected</span>
    </nav>

    <div class="flex shrink-0 items-center gap-1">
      <button class="btn" :disabled="store.addingRoot" title="Add folder (⌘O)" @click="store.addRoot()">
        <Loader2 v-if="store.addingRoot" class="h-3.5 w-3.5 animate-spin" />
        <FolderPlus v-else class="h-3.5 w-3.5" />
        <span class="hidden lg:inline">Add folder</span>
      </button>

      <button
        class="btn-icon"
        title="Refresh folder and folder tree (⌘R)"
        :disabled="!store.selectedFolder"
        @click="store.refresh()"
      >
        <RefreshCw class="h-3.5 w-3.5" :class="store.scanState === 'loading' ? 'animate-spin' : ''" />
      </button>

      <button
        class="btn-icon"
        :class="store.settings.recursive ? 'bg-accent-soft text-accent' : ''"
        title="Include subfolders (R)"
        :aria-pressed="store.settings.recursive"
        @click="store.setRecursive(!store.settings.recursive)"
      >
        <FolderTree class="h-3.5 w-3.5" />
      </button>

      <div class="mx-0.5 h-5 w-px bg-line" />

      <div class="segmented" role="group" aria-label="View mode">
        <button
          v-for="view in views"
          :key="view.mode"
          class="segmented-button"
          :class="store.viewMode === view.mode ? 'segmented-button-active' : ''"
          :title="view.hint"
          :aria-pressed="store.viewMode === view.mode"
          @click="store.setViewMode(view.mode)"
        >
          <component :is="view.icon" class="h-3.5 w-3.5" />
        </button>
      </div>

      <div v-if="sizeSliderVisible" class="ml-1 hidden items-center gap-1.5 xl:flex">
        <input
          type="range"
          min="120"
          max="480"
          step="4"
          class="w-20 cursor-pointer"
          style="accent-color: rgb(var(--c-accent))"
          :value="store.settings.thumbnailSize"
          title="Thumbnail size (use + or −)"
          aria-label="Thumbnail size"
          @input="onSizeInput"
        />
      </div>

      <div class="mx-0.5 h-5 w-px bg-line" />

      <button
        class="btn-icon"
        :class="store.settings.filmstripVisible ? 'bg-accent-soft text-accent' : ''"
        title="Toggle filmstrip (B)"
        :aria-pressed="store.settings.filmstripVisible"
        @click="store.toggleFilmstrip()"
      >
        <Film class="h-3.5 w-3.5" />
      </button>
      <button
        class="btn-icon"
        :class="store.infoVisible ? 'bg-accent-soft text-accent' : ''"
        title="Toggle info panel (I)"
        :aria-pressed="store.infoVisible"
        @click="store.toggleInfo()"
      >
        <PanelRight class="h-3.5 w-3.5" />
      </button>
      <button
        class="btn-icon"
        :title="store.resolvedTheme === 'dark' ? 'Switch to light theme (T)' : 'Switch to dark theme (T)'"
        @click="store.toggleTheme()"
      >
        <Sun v-if="store.resolvedTheme === 'dark'" class="h-3.5 w-3.5" />
        <Moon v-else class="h-3.5 w-3.5" />
      </button>
      <button class="btn-icon" title="Settings (⌘,)" @click="store.openSettings()">
        <Settings class="h-3.5 w-3.5" />
      </button>
    </div>
  </header>
</template>
