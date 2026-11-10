<script setup lang="ts">
import { computed, ref, watch } from 'vue';

import FilmstripItem from '@/components/FilmstripItem.vue';
import { useScrollMetrics } from '@/composables/useScrollMetrics';
import { useLibraryStore } from '@/stores/library';
import type { PhotoAsset } from '@/types';

const store = useLibraryStore();
const scroller = ref<HTMLElement | null>(null);
const { width, scrollLeft } = useScrollMetrics(scroller);

const ITEM_WIDTH = 84;
const GAP = 8;
const PADDING = 12;
const STRIDE = ITEM_WIDTH + GAP;

const totalWidth = computed(() => {
  const count = store.assets.length;
  if (!count) return PADDING * 2;
  return PADDING * 2 + count * ITEM_WIDTH + (count - 1) * GAP;
});

interface StripItem {
  asset: PhotoAsset;
  index: number;
  left: number;
}

const visible = computed<StripItem[]>(() => {
  const count = store.assets.length;
  if (!count || width.value <= 0) return [];
  const start = Math.max(0, Math.floor((scrollLeft.value - PADDING) / STRIDE) - 3);
  const end = Math.min(
    count,
    Math.ceil((scrollLeft.value + width.value - PADDING) / STRIDE) + 3,
  );
  const items: StripItem[] = [];
  for (let index = start; index < end; index += 1) {
    const asset = store.assets[index];
    if (!asset) continue;
    items.push({ asset, index, left: PADDING + index * STRIDE });
  }
  return items;
});

function scrollSelectionIntoView() {
  const el = scroller.value;
  const index = store.selectedIndex;
  if (!el || index < 0) return;
  const left = PADDING + index * STRIDE;
  const right = left + ITEM_WIDTH;
  if (left < el.scrollLeft) {
    el.scrollLeft = Math.max(0, left - PADDING);
  } else if (right > el.scrollLeft + el.clientWidth) {
    el.scrollLeft = right - el.clientWidth + PADDING;
  }
}

watch([() => store.selectedIndex, width], scrollSelectionIntoView);

watch(
  () => store.selectedFolder,
  () => {
    if (scroller.value) scroller.value.scrollLeft = 0;
  },
);

function onWheel(event: WheelEvent) {
  if (!scroller.value) return;
  scroller.value.scrollLeft += event.deltaY || event.deltaX;
}

function openItem(id: string) {
  // From another layout the strip opens the filmstrip view; once already there,
  // a second double-click goes straight to the fullscreen viewer.
  if (store.viewMode === 'filmstrip') store.openFullscreen(id);
  else store.openInFilmstrip(id);
}
</script>

<template>
  <section
    class="relative h-[88px] shrink-0 border-t border-line bg-panel"
    aria-label="Filmstrip"
  >
    <div
      v-if="store.assets.length"
      ref="scroller"
      class="no-scrollbar h-full overflow-x-auto overflow-y-hidden"
      @wheel.prevent="onWheel"
    >
      <div class="relative h-full" :style="{ width: `${totalWidth}px` }">
        <div
          v-for="item in visible"
          :key="item.asset.id"
          class="absolute top-3 h-16"
          :style="{ left: `${item.left}px`, width: `${ITEM_WIDTH}px` }"
        >
          <FilmstripItem
            :asset="item.asset"
            :selected="store.isSelected(item.asset.id)"
            @select="store.selectWithModifiers(item.asset.id, $event)"
            @open="openItem(item.asset.id)"
          />
        </div>
      </div>
    </div>
    <div v-else class="flex h-full items-center justify-center text-2xs text-subtle">
      No photos to show in the filmstrip.
    </div>
    <div
      class="pointer-events-none absolute inset-y-0 left-0 w-6 bg-gradient-to-r from-panel to-transparent"
    />
    <div
      class="pointer-events-none absolute inset-y-0 right-0 w-6 bg-gradient-to-l from-panel to-transparent"
    />
  </section>
</template>
