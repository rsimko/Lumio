<script setup lang="ts">
import { computed, ref, watch } from 'vue';

import PhotoListRow from '@/components/PhotoListRow.vue';
import { useScrollMetrics } from '@/composables/useScrollMetrics';
import { useLibraryStore } from '@/stores/library';
import type { PhotoAsset } from '@/types';

const store = useLibraryStore();
const scroller = ref<HTMLElement | null>(null);
const { width, height, scrollTop } = useScrollMetrics(scroller);

const ROW_HEIGHT = 58;
const compact = computed(() => width.value > 0 && width.value < 700);
const showFolder = computed(() => store.scanRecursive || store.assets.some((asset) => asset.relativeDir));

const totalHeight = computed(() => store.assets.length * ROW_HEIGHT);

interface VisibleRow {
  asset: PhotoAsset;
  index: number;
}

const visible = computed<VisibleRow[]>(() => {
  if (!store.assets.length) return [];
  const start = Math.max(0, Math.floor(scrollTop.value / ROW_HEIGHT) - 6);
  const end = Math.min(
    store.assets.length,
    Math.ceil((scrollTop.value + height.value) / ROW_HEIGHT) + 6,
  );
  const rows: VisibleRow[] = [];
  for (let index = start; index < end; index += 1) {
    const asset = store.assets[index];
    if (asset) rows.push({ asset, index });
  }
  return rows;
});

store.navRowSize = 1;

function scrollSelectionIntoView() {
  const el = scroller.value;
  const index = store.selectedIndex;
  if (!el || index < 0) return;
  const top = index * ROW_HEIGHT;
  const bottom = top + ROW_HEIGHT;
  if (top < el.scrollTop) {
    el.scrollTop = top;
  } else if (bottom > el.scrollTop + el.clientHeight) {
    el.scrollTop = bottom - el.clientHeight;
  }
}

watch([() => store.selectedIndex, height], scrollSelectionIntoView);

watch(
  () => store.selectedFolder,
  () => {
    if (scroller.value) scroller.value.scrollTop = 0;
  },
);
</script>

<template>
  <div class="flex h-full flex-col">
    <div
      class="grid h-8 shrink-0 items-center gap-3 border-b border-line bg-app/60 px-4 text-2xs font-semibold uppercase tracking-[0.1em] text-subtle"
      :class="
        compact
          ? 'grid-cols-[minmax(0,1fr)_54px_100px_72px]'
          : 'grid-cols-[minmax(0,1fr)_60px_104px_84px_112px_72px]'
      "
    >
      <span>Name</span>
      <span>Type</span>
      <span>Dimensions</span>
      <template v-if="!compact">
        <span>Size</span>
        <span>Modified</span>
      </template>
      <span>Assoc.</span>
    </div>

    <div ref="scroller" class="min-h-0 flex-1 overflow-y-auto overscroll-contain">
      <div class="relative" :style="{ height: `${totalHeight}px` }">
        <div
          v-for="row in visible"
          :key="row.asset.id"
          class="absolute inset-x-0"
          :style="{ top: `${row.index * ROW_HEIGHT}px`, height: `${ROW_HEIGHT}px` }"
        >
          <PhotoListRow
            :asset="row.asset"
            :selected="store.selectionId === row.asset.id"
            :show-folder="showFolder"
            :compact="compact"
            @select="store.select(row.asset.id)"
            @open="store.openInFilmstrip(row.asset.id)"
          />
        </div>
      </div>
    </div>
  </div>
</template>
