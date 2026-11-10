<script setup lang="ts">
import { ChevronRight, Pencil, Trash2 } from 'lucide-vue-next';
import { computed, nextTick, onBeforeUnmount, onMounted, ref, watch } from 'vue';

import { useLibraryStore } from '@/stores/library';
import type { ExternalEditor } from '@/types';

const store = useLibraryStore();
const menu = ref<HTMLElement | null>(null);
const position = ref({ x: 0, y: 0 });
const placed = ref(false);

const target = computed(() => store.photoMenuTarget);
const deleteLabel = computed(() =>
  target.value && target.value.count > 1
    ? `Delete ${target.value.count} Photos…`
    : 'Delete Photo…',
);
const editExpanded = ref(false);
const editors = computed<ExternalEditor[]>(() => {
  const defaultPath = store.defaultEditor?.path;
  return [...store.externalEditors].sort((a, b) => {
    if (a.path === defaultPath) return -1;
    if (b.path === defaultPath) return 1;
    return a.name.localeCompare(b.name);
  });
});

function editorTitle(editor: ExternalEditor): string {
  if (editor.path !== store.defaultEditor?.path) return `Edit with ${editor.name}`;
  return `Edit with ${editor.name} (default, ${store.shortcutLabel('browse.editDefault')})`;
}

function toggleEditors(): void {
  editExpanded.value = !editExpanded.value;
}

function editWith(editorPath: string): void {
  close();
  void store.editSelectionWithApplication(editorPath);
}

function openEditorSettings(): void {
  close();
  store.openSettings();
}

function placeMenu() {
  const element = menu.value;
  const point = store.photoMenuTarget;
  if (!element || !point) return;
  const margin = 8;
  const rect = element.getBoundingClientRect();
  position.value = {
    x: Math.max(margin, Math.min(point.x, window.innerWidth - rect.width - margin)),
    y: Math.max(margin, Math.min(point.y, window.innerHeight - rect.height - margin)),
  };
  placed.value = true;
}

function close() {
  store.closePhotoMenu();
}

function onKeydown(event: KeyboardEvent) {
  if (event.key !== 'Escape') return;
  event.preventDefault();
  close();
}

function onPointerDown(event: PointerEvent) {
  // Right-clicks on another photo are handled by that tile's contextmenu
  // handler, which replaces this menu in place.
  if (event.button === 2) return;
  if (menu.value?.contains(event.target as Node)) return;
  close();
}

function onContextMenu(event: MouseEvent) {
  if (menu.value?.contains(event.target as Node)) return;
  event.preventDefault();
  close();
}

function onViewportChange() {
  close();
}

// Opening another photo's menu keeps this component mounted, so reposition
// whenever the target changes.
watch(
  () => store.photoMenuTarget,
  () => {
    editExpanded.value = false;
    placed.value = false;
    void nextTick(placeMenu);
  },
);

// Expanding the editor list changes the menu height, so keep it inside the
// viewport instead of letting it run off the bottom edge.
watch(editExpanded, () => {
  placed.value = false;
  void nextTick(placeMenu);
});

onMounted(() => {
  void nextTick(placeMenu);
  window.addEventListener('keydown', onKeydown);
  window.addEventListener('pointerdown', onPointerDown);
  window.addEventListener('contextmenu', onContextMenu);
  window.addEventListener('resize', onViewportChange);
  window.addEventListener('scroll', onViewportChange, true);
  window.addEventListener('blur', onViewportChange);
});

onBeforeUnmount(() => {
  window.removeEventListener('keydown', onKeydown);
  window.removeEventListener('pointerdown', onPointerDown);
  window.removeEventListener('contextmenu', onContextMenu);
  window.removeEventListener('resize', onViewportChange);
  window.removeEventListener('scroll', onViewportChange, true);
  window.removeEventListener('blur', onViewportChange);
});
</script>

<template>
  <Teleport to="body">
    <div
      ref="menu"
      class="fixed z-50 max-h-[70vh] w-56 overflow-y-auto rounded-lg border border-line bg-elevated py-1 shadow-float"
      :class="placed ? '' : 'invisible'"
      :style="{ left: `${position.x}px`, top: `${position.y}px` }"
      role="menu"
      :aria-label="target ? `Photo actions for ${target.name}` : 'Photo actions'"
      @pointerdown.stop
      @contextmenu.prevent
    >
      <div v-if="target" class="px-3 py-1.5">
        <p class="truncate text-2xs font-semibold uppercase tracking-wide text-subtle">
          {{ target.name }}
        </p>
      </div>
      <div class="my-1 h-px bg-line" />

      <button
        v-if="!editors.length"
        type="button"
        class="flex w-full items-center gap-2.5 px-3 py-1.5 text-left text-xs text-muted transition hover:bg-panel-2 hover:text-ink"
        role="menuitem"
        title="Add an external editor in Settings"
        @click="openEditorSettings()"
      >
        <Pencil class="h-3.5 w-3.5" />
        Edit with
        <span class="ml-auto text-2xs text-subtle">Set up…</span>
      </button>
      <template v-else>
        <button
          type="button"
          class="flex w-full items-center gap-2.5 px-3 py-1.5 text-left text-xs text-muted transition hover:bg-panel-2 hover:text-ink"
          role="menuitem"
          :aria-expanded="editExpanded"
          :title="`Edit with default application (${store.shortcutLabel('browse.editDefault')})`"
          @click="toggleEditors()"
        >
          <Pencil class="h-3.5 w-3.5" />
          Edit with
          <ChevronRight
            class="ml-auto h-3 w-3 transition-transform"
            :class="editExpanded ? 'rotate-90' : ''"
          />
        </button>
        <div
          v-if="editExpanded"
          class="mx-1.5 mb-1 rounded-md border border-line/70 bg-panel/60 py-1"
        >
          <button
            v-for="editor in editors"
            :key="editor.path"
            type="button"
            class="flex w-full items-center gap-2 px-2 py-1.5 text-left text-xs text-muted transition hover:bg-panel-2 hover:text-ink"
            role="menuitem"
            :title="editorTitle(editor)"
            @click="editWith(editor.path)"
          >
            <span class="truncate">{{ editor.name }}</span>
            <span
              v-if="editor.path === store.defaultEditor?.path"
              class="ml-auto shrink-0 rounded bg-accent-soft px-1 py-0.5 text-3xs font-semibold uppercase tracking-wide text-accent"
            >
              Default
            </span>
          </button>
        </div>
      </template>

      <div class="my-1 h-px bg-line" />

      <button
        type="button"
        class="flex w-full items-center gap-2.5 px-3 py-1.5 text-left text-xs text-danger transition hover:bg-danger/10"
        role="menuitem"
        @click="store.deleteFromPhotoMenu()"
      >
        <Trash2 class="h-3.5 w-3.5" />
        {{ deleteLabel }}
      </button>
    </div>
  </Teleport>
</template>
