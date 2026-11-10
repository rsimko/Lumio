<script setup lang="ts">
import { Trash2 } from 'lucide-vue-next';
import { computed, nextTick, onBeforeUnmount, onMounted, ref, watch } from 'vue';

import { useLibraryStore } from '@/stores/library';

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
    placed.value = false;
    void nextTick(placeMenu);
  },
);

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
      class="fixed z-50 w-56 overflow-hidden rounded-lg border border-line bg-elevated py-1 shadow-float"
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
