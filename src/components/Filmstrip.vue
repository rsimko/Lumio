<script setup lang="ts">
import { computed, ref, watch } from 'vue';

import FilmstripItem from '@/components/FilmstripItem.vue';
import ResizeHandle from '@/components/ResizeHandle.vue';
import { useScrollMetrics } from '@/composables/useScrollMetrics';
import { useLibraryStore } from '@/stores/library';
import type { PhotoAsset } from '@/types';

const store = useLibraryStore();
const scroller = ref<HTMLElement | null>(null);
const { width, scrollLeft } = useScrollMetrics(scroller);

const GAP = 8;
const PADDING_X = 12;
const PADDING_Y = 12;
const ITEM_ASPECT = 84 / 64;
const MIN_HEIGHT = 64;
const MAX_HEIGHT = 280;

const barHeight = computed(() =>
  Math.min(MAX_HEIGHT, Math.max(MIN_HEIGHT, store.settings.filmstripHeight)),
);
const itemHeight = computed(() => barHeight.value - PADDING_Y * 2);
const itemWidth = computed(() => Math.round(itemHeight.value * ITEM_ASPECT));
const stride = computed(() => itemWidth.value + GAP);

const totalWidth = computed(() => {
  const count = store.assets.length;
  if (!count) return PADDING_X * 2;
  return PADDING_X * 2 + count * itemWidth.value + (count - 1) * GAP;
});

interface StripItem {
  asset: PhotoAsset;
  index: number;
  left: number;
}

const visible = computed<StripItem[]>(() => {
  const count = store.assets.length;
  if (!count || width.value <= 0) return [];
  const start = Math.max(0, Math.floor((scrollLeft.value - PADDING_X) / stride.value) - 3);
  const end = Math.min(
    count,
    Math.ceil((scrollLeft.value + width.value - PADDING_X) / stride.value) + 3,
  );
  const items: StripItem[] = [];
  for (let index = start; index < end; index += 1) {
    const asset = store.assets[index];
    if (!asset) continue;
    items.push({ asset, index, left: PADDING_X + index * stride.value });
  }
  return items;
});

function scrollSelectionIntoView() {
  const el = scroller.value;
  const index = store.selectedIndex;
  if (!el || index < 0) return;
  const left = PADDING_X + index * stride.value;
  const right = left + itemWidth.value;
  if (left < el.scrollLeft) {
    el.scrollLeft = Math.max(0, left - PADDING_X);
  } else if (right > el.scrollLeft + el.clientWidth) {
    el.scrollLeft = right - el.clientWidth + PADDING_X;
  }
}

watch([() => store.selectedIndex, width, itemWidth], scrollSelectionIntoView);

watch(
  () => store.browseKey,
  () => {
    if (scroller.value) scroller.value.scrollLeft = 0;
  },
);

function resize(height: number) {
  store.settings.filmstripHeight = height;
}

function commitResize(height: number) {
  store.updateSettings({ filmstripHeight: height });
}

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
    class="relative shrink-0 border-t border-line bg-panel"
    :style="{ height: `${barHeight}px` }"
    aria-label="Filmstrip"
  >
    <ResizeHandle
      side="top"
      :width="barHeight"
      :min="MIN_HEIGHT"
      :max="MAX_HEIGHT"
      @resize="resize"
      @commit="commitResize"
    />
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
          class="absolute"
          :style="{
            top: `${PADDING_Y}px`,
            left: `${item.left}px`,
            width: `${itemWidth}px`,
            height: `${itemHeight}px`,
          }"
        >
          <FilmstripItem
            :asset="item.asset"
            :selected="store.isSelected(item.asset.id)"
            :size="itemWidth"
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
