<script setup lang="ts">
import {
  ChevronRight,
  Columns3,
  Film,
  FolderOpen,
  FolderPlus,
  LayoutGrid,
  List,
  Loader2,
  Moon,
  PanelBottom,
  PanelRight,
  Rows3,
  Sun,
} from 'lucide-vue-next';
import { computed, ref, watch } from 'vue';

import LumioMark from '@/components/LumioMark.vue';
import { isBrowseViewMode, nextBrowseViewMode, type BrowseViewMode } from '@/lib/views';
import { useLibraryStore, type BreadcrumbItem } from '@/stores/library';

const store = useLibraryStore();

const browseViews = [
  { mode: 'grid', label: 'Grid', icon: LayoutGrid },
  { mode: 'masonry', label: 'Masonry', icon: Columns3 },
  { mode: 'justified', label: 'Justified', icon: Rows3 },
  { mode: 'list', label: 'List', icon: List },
] as const;

const lastBrowseMode = ref<BrowseViewMode>(
  isBrowseViewMode(store.viewMode) ? store.viewMode : 'grid',
);

watch(
  () => store.viewMode,
  (mode) => {
    if (isBrowseViewMode(mode)) lastBrowseMode.value = mode;
  },
);

const browseMode = computed<BrowseViewMode>(() =>
  isBrowseViewMode(store.viewMode) ? store.viewMode : lastBrowseMode.value,
);
const browseView = computed(
  () => browseViews.find((view) => view.mode === browseMode.value) ?? browseViews[0],
);
const nextBrowseView = computed(() => {
  const mode = isBrowseViewMode(store.viewMode)
    ? nextBrowseViewMode(store.viewMode)
    : lastBrowseMode.value;
  return browseViews.find((view) => view.mode === mode) ?? browseViews[0];
});
const browseActive = computed(() => isBrowseViewMode(store.viewMode));
const viewKeys = computed(() =>
  (['view.grid', 'view.masonry', 'view.justified', 'view.list'] as const)
    .map((id) => store.shortcutLabel(id))
    .join('/'),
);
const browseTitle = computed(() =>
  browseActive.value
    ? `${browseView.value.label} view — click for ${nextBrowseView.value.label} (${viewKeys.value})`
    : `Browse layout: ${browseView.value.label} — click to open (${viewKeys.value})`,
);

function cycleBrowseView() {
  store.setViewMode(nextBrowseView.value.mode);
}

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
      <LumioMark class="h-7 w-7 shrink-0" />
      <span class="text-sm font-semibold tracking-[0.01em] text-ink">Lumio</span>
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
      <button
        class="btn"
        :disabled="store.addingRoot"
        :title="`Add folder (${store.shortcutLabel('library.addFolder')})`"
        @click="store.addRoot()"
      >
        <Loader2 v-if="store.addingRoot" class="h-3.5 w-3.5 animate-spin" />
        <FolderPlus v-else class="h-3.5 w-3.5" />
        <span class="hidden lg:inline">Add folder</span>
      </button>

      <div class="mx-0.5 h-5 w-px bg-line" />

      <div
        class="hidden items-center gap-1.5 xl:flex"
        :class="sizeSliderVisible ? '' : 'invisible'"
      >
        <input
          type="range"
          min="120"
          max="480"
          step="4"
          class="w-20 cursor-pointer"
          style="accent-color: rgb(var(--c-accent))"
          :value="store.settings.thumbnailSize"
          :title="`Thumbnail size (${store.shortcutLabel('browse.thumbLarger')} / ${store.shortcutLabel('browse.thumbSmaller')})`"
          aria-label="Thumbnail size"
          :aria-hidden="!sizeSliderVisible"
          :tabindex="sizeSliderVisible ? 0 : -1"
          @input="onSizeInput"
        />
      </div>

      <div class="segmented" role="group" aria-label="View mode">
        <button
          class="segmented-button"
          :class="browseActive ? 'segmented-button-active' : ''"
          :title="browseTitle"
          :aria-label="browseTitle"
          :aria-pressed="browseActive"
          @click="cycleBrowseView"
        >
          <component :is="browseView.icon" class="h-3.5 w-3.5" />
        </button>
        <button
          class="segmented-button"
          :class="store.viewMode === 'filmstrip' ? 'segmented-button-active' : ''"
          :title="`Filmstrip view (${store.shortcutLabel('view.filmstrip')})`"
          :aria-label="`Filmstrip view (${store.shortcutLabel('view.filmstrip')})`"
          :aria-pressed="store.viewMode === 'filmstrip'"
          @click="store.setViewMode('filmstrip')"
        >
          <PanelBottom class="h-3.5 w-3.5" />
        </button>
      </div>

      <div class="mx-0.5 h-5 w-px bg-line" />

      <button
        v-if="store.viewMode === 'filmstrip'"
        class="btn-icon"
        :class="store.settings.filmstripVisible ? 'bg-accent-soft text-accent' : ''"
        :title="`Show or hide the filmstrip (${store.shortcutLabel('library.filmstrip')})`"
        :aria-pressed="store.settings.filmstripVisible"
        @click="store.toggleFilmstrip()"
      >
        <Film class="h-3.5 w-3.5" />
      </button>
      <button
        class="btn-icon"
        :class="store.infoVisible ? 'bg-accent-soft text-accent' : ''"
        :title="`Toggle info panel (${store.shortcutLabel('library.info')})`"
        :aria-pressed="store.infoVisible"
        @click="store.toggleInfo()"
      >
        <PanelRight class="h-3.5 w-3.5" />
      </button>
      <button
        class="btn-icon"
        :title="`${store.resolvedTheme === 'dark' ? 'Switch to light theme' : 'Switch to dark theme'} (${store.shortcutLabel('library.theme')})`"
        @click="store.toggleTheme()"
      >
        <Sun v-if="store.resolvedTheme === 'dark'" class="h-3.5 w-3.5" />
        <Moon v-else class="h-3.5 w-3.5" />
      </button>
    </div>
  </header>
</template>
