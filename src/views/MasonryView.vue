<script setup lang="ts">
import { computed, ref, watch } from 'vue';

import PhotoTile from '@/components/PhotoTile.vue';
import { useBrowseScroll } from '@/composables/useBrowseScroll';
import { useScrollMetrics } from '@/composables/useScrollMetrics';
import { aspectFor } from '@/composables/useThumbnails';
import { computeMasonry, visibleItems } from '@/lib/layout';
import { useLibraryStore } from '@/stores/library';
import type { PhotoAsset } from '@/types';

const store = useLibraryStore();
const scroller = ref<HTMLElement | null>(null);
const metrics = useScrollMetrics(scroller);
const { width, height, scrollTop } = metrics;
useBrowseScroll(scroller, metrics, () => store.browseKey);

const aspects = computed(() => store.assets.map((asset) => aspectFor(asset)));

const layout = computed(() =>
  computeMasonry(aspects.value, width.value, store.settings.thumbnailSize),
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
  x: number;
  y: number;
  width: number;
  height: number;
}

const visible = computed<VisibleItem[]>(() => {
  const current = layout.value;
  const items = visibleItems(current.items, scrollTop.value, height.value, 500);
  const result: VisibleItem[] = [];
  for (const item of items) {
    const asset = store.assets[item.index];
    if (!asset) continue;
    result.push({
      asset,
      index: item.index,
      x: item.x,
      y: item.y,
      width: item.width,
      height: item.height,
    });
  }
  return result;
});

function scrollSelectionIntoView() {
  const el = scroller.value;
  const index = store.selectedIndex;
  if (!el || index < 0) return;
  const item = layout.value.items[index];
  if (!item) return;
  const top = item.y;
  const bottom = item.y + item.height;
  if (top < el.scrollTop) {
    el.scrollTop = Math.max(0, top - 20);
  } else if (bottom > el.scrollTop + el.clientHeight) {
    el.scrollTop = bottom - el.clientHeight + 20;
  }
}

watch(() => store.selectedIndex, scrollSelectionIntoView);

watch(
  () => store.browseKey,
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
          left: `${item.x}px`,
          top: `${item.y}px`,
          width: `${item.width}px`,
          height: `${item.height}px`,
        }"
      >
        <PhotoTile
          :asset="item.asset"
          :width="item.width"
          :height="item.height"
          :selected="store.isSelected(item.asset.id)"
          @select="store.selectWithModifiers(item.asset.id, $event)"
          @open="store.openInFilmstrip(item.asset.id)"
        />
      </div>
    </div>
  </div>
</template>
