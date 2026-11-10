<script setup lang="ts">
import { FolderPlus, FolderTree, Loader2, RefreshCw, X } from 'lucide-vue-next';

import FolderTreeNode from '@/components/FolderTreeNode.vue';
import ResizeHandle from '@/components/ResizeHandle.vue';
import { useLibraryStore } from '@/stores/library';
import type { FolderNode, RootInfo } from '@/types';

const store = useLibraryStore();

function rootNode(root: RootInfo): FolderNode {
  return {
    name: root.name,
    path: root.path,
    hasChildren: true,
    accessible: root.available,
    error: root.error,
    // Root counts are loaded lazily by the store, like any other folder row.
    imageCount: 0,
    recursiveImageCount: 0,
  };
}

function resize(width: number) {
  store.settings.libraryWidth = width;
}

function commitResize(width: number) {
  store.updateSettings({ libraryWidth: width });
}
</script>

<template>
  <aside
    class="relative flex h-full shrink-0 flex-col border-r border-line bg-panel"
    :style="{ width: `${store.settings.libraryWidth}px` }"
  >
    <div class="flex h-10 shrink-0 items-center justify-between px-3">
      <span class="panel-heading">Library</span>
      <div class="flex items-center gap-0.5">
        <button
          class="btn-icon-sm"
          :title="`Refresh selected folder (${store.shortcutLabel('library.refresh')})`"
          :disabled="!store.selectedFolder"
          @click="store.refresh()"
        >
          <RefreshCw
            class="h-3.5 w-3.5"
            :class="store.scanState === 'loading' ? 'animate-spin' : ''"
          />
        </button>
        <button
          class="btn-icon-sm"
          :class="store.settings.recursive ? 'bg-accent-soft text-accent' : ''"
          :title="`Include subfolders (${store.shortcutLabel('library.recursive')})`"
          :aria-pressed="store.settings.recursive"
          @click="store.setRecursive(!store.settings.recursive)"
        >
          <FolderTree class="h-3.5 w-3.5" />
        </button>
        <button
          class="btn-icon-sm"
          :title="`Add a folder (${store.shortcutLabel('library.addFolder')})`"
          :disabled="store.addingRoot"
          @click="store.addRoot()"
        >
          <Loader2 v-if="store.addingRoot" class="h-3.5 w-3.5 animate-spin" />
          <FolderPlus v-else class="h-3.5 w-3.5" />
        </button>
      </div>
    </div>

    <div
      v-if="store.recoveryWarning && !store.recoveryWarningDismissed"
      class="mx-2 mb-2 rounded-md border border-warning/30 bg-warning/10 px-2.5 py-2 text-2xs leading-snug text-warning"
    >
      <div class="flex items-start gap-1.5">
        <p class="min-w-0 flex-1">{{ store.recoveryWarning }}</p>
        <button
          class="mt-px shrink-0 opacity-70 transition hover:opacity-100"
          title="Dismiss"
          @click="store.dismissRecoveryWarning()"
        >
          <X class="h-3 w-3" />
        </button>
      </div>
    </div>

    <div class="min-h-0 flex-1 overflow-y-auto px-1.5 pb-2">
      <div v-if="store.initializing" class="space-y-1 px-2 py-1">
        <div
          v-for="row in 5"
          :key="row"
          class="h-6 animate-pulse rounded bg-panel-2"
          :style="{ width: `${100 - row * 9}%` }"
        />
      </div>

      <div v-else-if="!store.roots.length" class="flex flex-col items-center px-3 py-8 text-center">
        <div class="mb-3 grid h-10 w-10 place-items-center rounded-full bg-panel-2 text-subtle">
          <FolderPlus class="h-5 w-5" />
        </div>
        <p class="text-xs font-medium text-ink">No folders yet</p>
        <p class="mt-1 text-2xs leading-relaxed text-subtle">
          Add a folder that already contains photos. Lumio never moves or changes the originals.
        </p>
        <button class="btn-primary mt-3 w-full" :disabled="store.addingRoot" @click="store.addRoot()">
          <FolderPlus class="h-3.5 w-3.5" />
          Add folder
        </button>
      </div>

      <template v-else>
        <FolderTreeNode
          v-for="root in store.roots"
          :key="root.id"
          :node="rootNode(root)"
          :root-id="root.id"
          :depth="0"
          is-root
          :available="root.available"
        />
      </template>
    </div>

    <div
      class="flex h-7 shrink-0 items-center justify-between border-t border-line px-3 text-2xs text-subtle"
    >
      <span>{{ store.roots.length }} {{ store.roots.length === 1 ? 'folder' : 'folders' }}</span>
      <span v-if="store.scanState === 'loading'" class="flex items-center gap-1">
        <Loader2 class="h-3 w-3 animate-spin" />
        Scanning…
      </span>
      <span v-else-if="store.scanState === 'ready'">{{ store.stats.total }} photos</span>
      <span v-else>Ready</span>
    </div>

    <ResizeHandle
      side="right"
      :width="store.settings.libraryWidth"
      :min="220"
      :max="440"
      @resize="resize"
      @commit="commitResize"
    />
  </aside>
</template>
