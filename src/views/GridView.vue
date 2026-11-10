<script setup lang="ts">
import { computed, ref, watch } from 'vue';

import PhotoTile from '@/components/PhotoTile.vue';
import { useScrollMetrics } from '@/composables/useScrollMetrics';
import { computeGrid, visibleRowRange } from '@/lib/layout';
import { useLibraryStore } from '@/stores/library';
import type { PhotoAsset } from '@/types';

const store = useLibraryStore();
const scroller = ref<HTMLElement | null>(null);
const { width, height, scrollTop } = useScrollMetrics(scroller);

const layout = computed(() =>
  computeGrid(store.assets.length, width.value, store.settings.thumbnailSize),
);

watch(
  () => layout.value.columns,
  (columns) => {
    store.navRowSize = Math.max(1, columns);
  },
  { immediate: true },
);

interface VisibleItem {
  asset: PhotoAsset;
  index: number;
  left: number;
  top: number;
}

const visible = computed<VisibleItem[]>(() => {
  const current = layout.value;
  const range = visibleRowRange(
    Math.max(0, scrollTop.value - current.padding),
    height.value,
    current.rowStride,
    current.rows,
    2,
  );
  const start = range.start * current.columns;
  const end = Math.min(store.assets.length, range.end * current.columns);
  const items: VisibleItem[] = [];
  for (let index = start; index < end; index += 1) {
    const asset = store.assets[index];
    if (!asset) continue;
    const row = Math.floor(index / current.columns);
    const column = index % current.columns;
    items.push({
      asset,
      index,
      left: current.padding + column * (current.cell + current.gap),
      top: current.padding + row * current.rowStride,
    });
  }
  return items;
});

function scrollSelectionIntoView() {
  const el = scroller.value;
  const index = store.selectedIndex;
  if (!el || index < 0) return;
  const current = layout.value;
  const row = Math.floor(index / current.columns);
  const top = current.padding + row * current.rowStride;
  const bottom = top + current.cell;
  if (top < el.scrollTop) {
    el.scrollTop = Math.max(0, top - current.padding);
  } else if (bottom > el.scrollTop + el.clientHeight) {
    el.scrollTop = bottom - el.clientHeight + current.padding;
  }
}

watch(
  [() => store.selectedIndex, () => layout.value.cell, height],
  scrollSelectionIntoView,
);

watch(
  () => store.selectedFolder,
  () => {
    if (scroller.value) scroller.value.scrollTop = 0;
  },
);
</script>

<template>
  <div ref="scroller" class="h-full overflow-y-auto overscroll-contain">
    <div
      class="relative mx-auto"
      :style="{ height: `${layout.totalHeight}px` }"
    >
      <div
        v-for="item in visible"
        :key="item.asset.id"
        class="absolute"
        :style="{
          left: `${item.left}px`,
          top: `${item.top}px`,
          width: `${layout.cell}px`,
          height: `${layout.cell}px`,
        }"
      >
        <PhotoTile
          :asset="item.asset"
          :width="layout.cell"
          :height="layout.cell"
          :selected="store.selectionId === item.asset.id"
          @select="store.select(item.asset.id)"
          @open="store.openFullscreen(item.asset.id)"
        />
      </div>
    </div>
  </div>
</template>
