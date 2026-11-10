<script setup lang="ts">
import { computed, ref, watch } from 'vue';

import PhotoPlaceholder from '@/components/PhotoPlaceholder.vue';
import { markPreviewFailed, useThumbnail } from '@/composables/useThumbnails';
import { ASSET_DRAG_MIME } from '@/lib/assets';
import { sourceLabel } from '@/lib/format';
import { useLibraryStore } from '@/stores/library';
import type { PhotoAsset } from '@/types';

const props = withDefaults(
  defineProps<{
    asset: PhotoAsset;
    selected?: boolean;
  }>(),
  { selected: false },
);

const emit = defineEmits<{
  select: [event: MouseEvent];
  open: [];
}>();

const store = useLibraryStore();

function onDragStart(event: DragEvent) {
  if (!event.dataTransfer) return;
  const ids = store.beginAssetDrag(props.asset.id);
  event.dataTransfer.effectAllowed = 'move';
  event.dataTransfer.setData(ASSET_DRAG_MIME, ids.join('\n'));
  event.dataTransfer.setData(
    'text/plain',
    ids.length === 1 ? props.asset.fileName : `${ids.length} photos`,
  );
}

const thumb = useThumbnail(
  () => props.asset,
  () => 76,
);
const loaded = ref(false);
watch(
  () => thumb.value?.src,
  () => {
    loaded.value = false;
  },
);

const title = computed(() => {
  const source =
    thumb.value?.status === 'ready'
      ? sourceLabel(thumb.value.source)
      : thumb.value?.status === 'error'
        ? 'Preview unavailable'
        : 'Loading preview…';
  return `${props.asset.fileName} — ${source}`;
});
</script>

<template>
  <div
    class="group relative h-full w-full overflow-hidden rounded bg-canvas ring-1 ring-black/[0.06] dark:ring-white/[0.08]"
    :class="selected ? 'outline outline-2 outline-offset-2 outline-accent' : ''"
    :title="title"
    draggable="true"
    @click="emit('select', $event)"
    @dblclick="$emit('open')"
    @dragstart="onDragStart"
    @dragend="store.endAssetDrag()"
  >
    <PhotoPlaceholder
      class="absolute inset-0"
      :asset="asset"
      :loading="!thumb || thumb.status === 'loading'"
      :error="thumb?.error"
      compact
    />
    <img
      v-if="thumb?.src"
      :src="thumb.src"
      :alt="asset.fileName"
      class="absolute inset-0 h-full w-full object-cover transition-opacity duration-150"
      :class="loaded ? 'opacity-100' : 'opacity-0'"
      draggable="false"
      loading="lazy"
      decoding="async"
      @load="loaded = true"
      @error="markPreviewFailed(thumb)"
    />
    <span
      v-if="asset.kind === 'raw'"
      class="absolute left-1 top-1 rounded bg-black/55 px-1 py-px text-3xs font-semibold uppercase tracking-wider text-white/85 backdrop-blur-[2px]"
    >
      RAW
    </span>
  </div>
</template>
