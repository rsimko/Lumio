<script setup lang="ts">
import { Images } from 'lucide-vue-next';
import { computed, ref } from 'vue';

import PhotoPlaceholder from '@/components/PhotoPlaceholder.vue';
import SourceBadge from '@/components/SourceBadge.vue';
import { useScrollMetrics } from '@/composables/useScrollMetrics';
import { dimensionsFor, markPreviewFailed, useThumbnail } from '@/composables/useThumbnails';
import { formatDimensions } from '@/lib/format';
import { useLibraryStore } from '@/stores/library';

const store = useLibraryStore();
const stage = ref<HTMLElement | null>(null);
const { width, height } = useScrollMetrics(stage);

const previewSize = computed(() => Math.max(800, width.value, height.value));

const thumb = useThumbnail(
  () => store.selectedAsset,
  () => previewSize.value,
);

const dimensions = computed(() => {
  const asset = store.selectedAsset;
  if (!asset) return '—';
  const known = dimensionsFor(asset);
  return known ? formatDimensions(known.width, known.height) : '—';
});

const source = computed(() => thumb.value?.source ?? store.selectedAsset?.previewHint ?? 'placeholder');
const counter = computed(() => {
  if (store.selectedIndex < 0) return '';
  return `${store.selectedIndex + 1} / ${store.assets.length}`;
});

function openContextMenu(event: MouseEvent) {
  const asset = store.selectedAsset;
  if (!asset) return;
  store.openPhotoMenu(asset.id, event.clientX, event.clientY);
}
</script>

<template>
  <div
    ref="stage"
    class="relative flex h-full min-h-0 items-center justify-center overflow-hidden bg-canvas p-6"
    @contextmenu.prevent.stop="openContextMenu"
  >
    <template v-if="store.selectedAsset">
      <img
        v-if="thumb?.src"
        :src="thumb.src"
        :alt="store.selectedAsset.fileName"
        class="max-h-full max-w-full select-none object-contain shadow-2xl"
        draggable="false"
        @dblclick="store.openFullscreen()"
        @error="markPreviewFailed(thumb)"
      />
      <div
        v-else
        class="flex h-56 w-80 max-w-full flex-col overflow-hidden rounded-lg border border-line bg-panel shadow-panel"
      >
        <PhotoPlaceholder
          :asset="store.selectedAsset"
          :loading="!thumb || thumb.status === 'loading'"
          :error="thumb?.error"
        />
      </div>

      <div class="pointer-events-none absolute left-4 top-4 flex items-center gap-2">
        <SourceBadge :source="source" :kind="store.selectedAsset.kind" />
        <span class="rounded bg-black/35 px-1.5 py-0.5 text-2xs tabular-nums text-white/85 backdrop-blur-[2px]">
          {{ dimensions }}
        </span>
      </div>

      <div class="pointer-events-none absolute inset-x-4 bottom-3 flex items-end justify-between gap-4">
        <p class="min-w-0 truncate text-xs font-medium text-ink/80">
          {{ store.selectedAsset.fileName }}
        </p>
        <span class="shrink-0 rounded bg-black/35 px-1.5 py-0.5 text-2xs tabular-nums text-white/85 backdrop-blur-[2px]">
          {{ counter }}
        </span>
      </div>
    </template>

    <div v-else class="flex flex-col items-center text-center text-subtle">
      <Images class="mb-2 h-8 w-8 opacity-60" />
      <p class="text-sm font-medium text-muted">No photo selected</p>
      <p class="mt-1 text-xs">Choose a photo from the filmstrip below.</p>
    </div>
  </div>
</template>
