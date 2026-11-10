<script setup lang="ts">
import {
  AlertTriangle,
  ChevronRight,
  Folder,
  FolderOpen,
  Loader2,
  Trash2,
} from 'lucide-vue-next';
import { computed, watch } from 'vue';

import { formatCount } from '@/lib/format';
import { useLibraryStore } from '@/stores/library';
import type { FolderNode } from '@/types';

defineOptions({ name: 'FolderTreeNode' });

const props = withDefaults(
  defineProps<{
    node: FolderNode;
    rootId: string;
    depth: number;
    isRoot?: boolean;
    available?: boolean;
  }>(),
  { isRoot: false, available: true },
);

const store = useLibraryStore();

const children = computed(() => store.tree.get(props.node.path) ?? []);
const loading = computed(() => store.treeLoading.has(props.node.path));
const error = computed(() => store.treeErrors.get(props.node.path) ?? null);
const isExpanded = computed(() => store.expanded.has(props.node.path));
const isSelected = computed(() => store.selectedFolder === props.node.path);
const canExpand = computed(() => props.isRoot || props.node.hasChildren || children.value.length > 0);

const imageCount = computed(() => store.imageCountFor(props.node.path));
const countTitle = computed(() => {
  const count = imageCount.value;
  if (count === null) return undefined;
  const scope = store.settings.recursive ? 'including subfolders' : 'in this folder';
  return `${formatCount(count)} ${count === 1 ? 'photo' : 'photos'} ${scope}`;
});

watch(
  [() => props.node.path, () => store.folderCountsEpoch],
  () => {
    void store.ensureFolderImageCount(props.node.path);
  },
  { immediate: true },
);

const indent = computed(() => `${props.depth * 12}px`);

function toggle() {
  if (!canExpand.value) return;
  void store.toggleExpanded(props.node.path);
}

function select() {
  void store.selectFolder(props.node.path, props.rootId);
}

function removeFolder() {
  if (props.isRoot) store.openRemoveFolder(props.rootId);
  else store.openDeleteFolder(props.node.path, props.rootId);
}

function openMenu(event: MouseEvent) {
  store.openFolderMenu(
    {
      path: props.node.path,
      rootId: props.rootId,
      name: props.node.name,
      isRoot: props.isRoot,
    },
    event.clientX,
    event.clientY,
  );
}
</script>

<template>
  <div>
    <div
      class="group flex h-7 items-center gap-1 rounded pr-1.5 transition-colors"
      :class="
        isSelected
          ? 'bg-accent-soft text-ink'
          : 'text-muted hover:bg-panel-2 hover:text-ink'
      "
      @contextmenu.prevent.stop="openMenu"
    >
      <div class="flex w-4 shrink-0 items-center justify-center" :style="{ marginLeft: indent }">
        <button
          v-if="canExpand"
          class="flex h-4 w-4 items-center justify-center rounded text-subtle transition hover:text-ink disabled:opacity-40"
          :title="isExpanded ? 'Collapse' : 'Expand'"
          @click.stop="toggle"
        >
          <Loader2 v-if="loading" class="h-3 w-3 animate-spin" />
          <ChevronRight
            v-else
            class="h-3 w-3 transition-transform duration-150"
            :class="isExpanded ? 'rotate-90' : ''"
          />
        </button>
        <span v-else class="h-4 w-4" />
      </div>

      <component
        :is="isRoot ? (isExpanded ? FolderOpen : Folder) : Folder"
        class="h-3.5 w-3.5 shrink-0"
        :class="
          !available
            ? 'text-danger/80'
            : isSelected
              ? 'text-accent'
              : 'text-subtle group-hover:text-muted'
        "
      />

      <button
        class="min-w-0 flex-1 truncate py-1 text-left text-xs"
        :class="isSelected ? 'font-medium text-ink' : ''"
        :title="node.path"
        @click="select"
        @dblclick="toggle"
      >
        {{ node.name }}
      </button>

      <span
        v-if="imageCount !== null"
        class="shrink-0 rounded px-1 py-0.5 text-2xs font-medium tabular-nums transition-colors"
        :class="
          isSelected
            ? 'bg-accent/10 text-accent'
            : 'bg-panel-2/70 text-subtle group-hover:text-muted'
        "
        :title="countTitle"
        :aria-label="countTitle"
      >
        {{ formatCount(imageCount) }}
      </span>

      <Loader2 v-if="loading" class="h-3 w-3 shrink-0 animate-spin text-subtle" />
      <AlertTriangle
        v-if="!available"
        class="h-3 w-3 shrink-0 text-danger"
        title="This folder is currently unavailable"
      />
      <button
        class="hidden h-5 w-5 shrink-0 items-center justify-center rounded text-subtle transition hover:bg-danger/10 hover:text-danger group-hover:flex"
        title="Remove folder…"
        @click.stop="removeFolder"
      >
        <Trash2 class="h-3 w-3" />
      </button>
    </div>

    <div v-if="isExpanded">
      <div
        v-if="loading && !children.length"
        class="py-1 text-2xs text-subtle"
        :style="{ paddingLeft: `calc(${indent} + 34px)` }"
      >
        Loading folders…
      </div>
      <div
        v-else-if="error"
        class="py-1 pr-2 text-2xs leading-snug text-danger"
        :style="{ paddingLeft: `calc(${indent} + 34px)` }"
        :title="error"
      >
        {{ error }}
      </div>
      <FolderTreeNode
        v-for="child in children"
        :key="child.path"
        :node="child"
        :root-id="rootId"
        :depth="depth + 1"
      />
      <div
        v-if="!loading && !error && !children.length"
        class="py-1 text-2xs italic text-subtle"
        :style="{ paddingLeft: `calc(${indent} + 34px)` }"
      >
        No subfolders
      </div>
    </div>
  </div>
</template>
