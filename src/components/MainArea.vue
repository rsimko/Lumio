<script setup lang="ts">
import {
  AlertTriangle,
  Aperture,
  FolderOpen,
  FolderPlus,
  FolderSearch,
  Images,
  Loader2,
  RefreshCw,
} from 'lucide-vue-next';
import { computed } from 'vue';

import { formatCount, formatDuration, pluralize } from '@/lib/format';
import { useLibraryStore } from '@/stores/library';
import GridView from '@/views/GridView.vue';
import JustifiedView from '@/views/JustifiedView.vue';
import ListView from '@/views/ListView.vue';
import MasonryView from '@/views/MasonryView.vue';
import FilmstripView from '@/views/FilmstripView.vue';
import type { RootInfo } from '@/types';

const store = useLibraryStore();

const viewComponent = computed(() => {
  switch (store.viewMode) {
    case 'masonry':
      return MasonryView;
    case 'justified':
      return JustifiedView;
    case 'list':
      return ListView;
    case 'filmstrip':
      return FilmstripView;
    default:
      return GridView;
  }
});

const showHeader = computed(() => Boolean(store.selectedFolder));
const showSkeleton = computed(
  () => store.scanState === 'loading' && store.assets.length === 0,
);
const showAssets = computed(
  () =>
    (store.scanState === 'ready' || store.scanState === 'loading') &&
    store.assets.length > 0,
);
const showEmptyFolder = computed(
  () => store.scanState === 'ready' && store.assets.length === 0,
);
const showError = computed(() => store.scanState === 'error');
const showWelcome = computed(
  () =>
    !store.initializing &&
    !store.initializationError &&
    !store.hasRoots &&
    !store.selectedFolder,
);
const showChooseFolder = computed(
  () => !store.initializing && store.hasRoots && !store.selectedFolder && !showWelcome.value,
);
const permissionHint = computed(() => store.scanErrorCode === 'permissionDenied');

function selectRoot(root: RootInfo) {
  void store.selectFolder(root.path, root.id);
}

function removeSelectedRoot() {
  if (store.selectedRootId) store.openRemoveFolder(store.selectedRootId);
}
</script>

<template>
  <section class="flex h-full min-h-0 flex-col bg-main">
    <div
      v-if="showHeader"
      class="flex h-9 shrink-0 items-center gap-2 border-b border-line bg-app/55 px-4"
    >
      <FolderOpen class="h-3.5 w-3.5 shrink-0 text-subtle" />
      <span class="truncate text-xs font-medium text-ink" :title="store.selectedFolder ?? ''">
        {{ store.folderName }}
      </span>
      <span
        v-if="store.settings.recursive"
        class="badge bg-accent-soft text-accent ring-1 ring-accent/15"
      >
        Recursive
      </span>
      <div class="ml-auto flex items-center gap-2.5 text-2xs text-subtle">
        <span v-if="store.scanState === 'loading'" class="flex items-center gap-1.5">
          <Loader2 class="h-3 w-3 animate-spin" />
          Scanning…
        </span>
        <template v-else-if="store.scanState === 'ready'">
          <span class="tabular-nums">
            {{ formatCount(store.stats.total) }}
            {{ pluralize(store.stats.total, 'photo') }}
          </span>
          <span v-if="store.scanElapsedMs" class="hidden tabular-nums md:inline">
            · scanned in {{ formatDuration(store.scanElapsedMs) }}
          </span>
        </template>
        <button
          v-if="store.warningsCount"
          class="flex items-center gap-1 rounded px-1 py-0.5 text-warning transition hover:bg-warning/10"
          title="Show scan warnings"
          @click="store.openWarnings()"
        >
          <AlertTriangle class="h-3 w-3" />
          {{ store.warningsCount }}
        </button>
      </div>
    </div>

    <div class="relative min-h-0 flex-1">
      <div
        v-if="showSkeleton"
        class="absolute inset-x-0 top-0 z-20 h-0.5 overflow-hidden bg-accent/20"
      >
        <div class="h-full w-1/3 animate-pulse bg-accent" />
      </div>

      <div v-if="store.initializing" class="flex h-full flex-col items-center justify-center gap-3">
        <Loader2 class="h-5 w-5 animate-spin text-accent" />
        <p class="text-xs text-muted">Opening your library…</p>
      </div>

      <div v-else-if="store.initializationError" class="flex h-full items-center justify-center p-8">
        <div class="max-w-md text-center">
          <div class="mx-auto mb-3 grid h-11 w-11 place-items-center rounded-full bg-danger/10 text-danger">
            <AlertTriangle class="h-5 w-5" />
          </div>
          <h2 class="text-sm font-semibold text-ink">Lumio could not start</h2>
          <p class="mt-2 text-xs leading-relaxed text-muted">{{ store.initializationError }}</p>
          <button class="btn mt-4" @click="store.initialize()">
            <RefreshCw class="h-3.5 w-3.5" />
            Try again
          </button>
        </div>
      </div>

      <div v-else-if="showWelcome" class="flex h-full items-center justify-center p-8">
        <div class="max-w-md text-center">
          <div class="mx-auto mb-4 grid h-14 w-14 place-items-center rounded-2xl bg-accent-soft text-accent ring-1 ring-accent/15">
            <Aperture class="h-7 w-7" />
          </div>
          <h1 class="text-balance text-xl font-semibold tracking-tight text-ink">
            Your photos, where they live.
          </h1>
          <p class="mx-auto mt-2 max-w-sm text-balance text-xs leading-relaxed text-muted">
            Lumio is a local-first browser for RAW photography. Add a folder and it reads
            the originals in place — nothing is imported, moved, renamed, or modified.
          </p>
          <button class="btn-primary mt-5" @click="store.addRoot()">
            <FolderPlus class="h-3.5 w-3.5" />
            Add your first folder
          </button>
          <p class="mt-4 text-2xs uppercase tracking-[0.14em] text-subtle">
            RAW + JPEG + XMP · Local only
          </p>
        </div>
      </div>

      <div v-else-if="showChooseFolder" class="flex h-full items-center justify-center p-8">
        <div class="w-full max-w-sm text-center">
          <div class="mx-auto mb-3 grid h-11 w-11 place-items-center rounded-full bg-panel-2 text-subtle">
            <Images class="h-5 w-5" />
          </div>
          <h2 class="text-sm font-semibold text-ink">Choose a folder</h2>
          <p class="mt-1 text-xs text-muted">Pick a library folder to start browsing.</p>
          <div class="mt-4 space-y-1 text-left">
            <button
              v-for="root in store.roots"
              :key="root.id"
              class="flex w-full items-center gap-2 rounded-md border border-line bg-panel px-3 py-2 text-xs transition hover:border-line-strong hover:bg-panel-2"
              :class="root.available ? 'text-ink' : 'text-muted'"
              @click="selectRoot(root)"
            >
              <FolderOpen class="h-3.5 w-3.5 shrink-0 text-subtle" />
              <span class="min-w-0 flex-1 truncate text-left">{{ root.name }}</span>
              <span
                v-if="!root.available"
                class="shrink-0 text-2xs text-danger"
                title="Folder unavailable"
              >
                Offline
              </span>
            </button>
          </div>
          <button class="btn mt-4" @click="store.addRoot()">
            <FolderPlus class="h-3.5 w-3.5" />
            Add another folder
          </button>
        </div>
      </div>

      <div v-else-if="showSkeleton" class="h-full overflow-hidden p-5">
        <div
          class="grid gap-3"
          style="grid-template-columns: repeat(auto-fill, minmax(140px, 1fr))"
        >
          <div
            v-for="cell in 24"
            :key="cell"
            class="aspect-square animate-pulse rounded bg-panel-2/70"
            :style="{ animationDelay: `${(cell % 8) * 40}ms` }"
          />
        </div>
      </div>

      <div v-else-if="showError" class="flex h-full items-center justify-center overflow-y-auto p-8">
        <div class="max-w-md text-center">
          <div class="mx-auto mb-3 grid h-11 w-11 place-items-center rounded-full bg-danger/10 text-danger">
            <AlertTriangle class="h-5 w-5" />
          </div>
          <h2 class="text-sm font-semibold text-ink">
            Could not open {{ store.folderName }}
          </h2>
          <p class="mt-2 break-words text-xs leading-relaxed text-muted">
            {{ store.scanError }}
          </p>
          <p
            v-if="permissionHint"
            class="mx-auto mt-2 max-w-sm rounded-md border border-line bg-panel px-3 py-2 text-2xs leading-relaxed text-subtle"
          >
            macOS may be blocking access. Allow Lumio under System Settings → Privacy &amp;
            Security → Files and Folders, then try again.
          </p>
          <p class="mt-3 break-all font-mono text-2xs text-subtle">{{ store.selectedFolder }}</p>
          <div class="mt-4 flex flex-wrap items-center justify-center gap-2">
            <button class="btn-primary" @click="store.scanFolder()">
              <RefreshCw class="h-3.5 w-3.5" />
              Try again
            </button>
            <button
              v-if="store.selectedRootId"
              class="btn text-danger hover:border-danger/40 hover:bg-danger/10"
              title="Remove this folder from Lumio or move it to the Trash."
              @click="removeSelectedRoot()"
            >
              Remove from library
            </button>
          </div>
        </div>
      </div>

      <div v-else-if="showEmptyFolder" class="flex h-full items-center justify-center p-8">
        <div class="max-w-sm text-center">
          <div class="mx-auto mb-3 grid h-11 w-11 place-items-center rounded-full bg-panel-2 text-subtle">
            <FolderSearch class="h-5 w-5" />
          </div>
          <h2 class="text-sm font-semibold text-ink">No photos here</h2>
          <p class="mt-1 text-xs leading-relaxed text-muted">
            {{
              store.settings.recursive
                ? 'This folder and its subfolders contain no supported photos.'
                : 'This folder contains no supported photos. Nested folders may have some.'
            }}
          </p>
          <div class="mt-4 flex items-center justify-center gap-2">
            <button class="btn" @click="store.refresh()">
              <RefreshCw class="h-3.5 w-3.5" />
              Refresh
            </button>
            <button
              v-if="!store.settings.recursive"
              class="btn-primary"
              @click="store.setRecursive(true)"
            >
              Include subfolders
            </button>
          </div>
          <p class="mt-4 text-2xs text-subtle">
            RAW, JPEG, PNG, TIFF and WebP are supported.
          </p>
        </div>
      </div>

      <template v-else-if="showAssets">
        <div class="absolute inset-x-0 top-0 z-10 h-0.5 overflow-hidden" v-if="store.scanState === 'loading'">
          <div class="h-full w-1/3 animate-pulse bg-accent" />
        </div>
        <Transition name="fade" mode="out-in">
          <component :is="viewComponent" :key="store.viewMode" />
        </Transition>
      </template>

      <div v-else class="flex h-full items-center justify-center p-8 text-center">
        <div>
          <Loader2 class="mx-auto h-5 w-5 animate-spin text-subtle" />
          <p class="mt-2 text-xs text-subtle">Preparing view…</p>
        </div>
      </div>
    </div>
  </section>
</template>
