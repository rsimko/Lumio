<script setup lang="ts">
import { computed, ref, watch } from 'vue';

import PhotoPlaceholder from '@/components/PhotoPlaceholder.vue';
import { markPreviewFailed, useThumbnail } from '@/composables/useThumbnails';
import { sourceLabel } from '@/lib/format';
import type { PhotoAsset } from '@/types';

const props = withDefaults(
  defineProps<{
    asset: PhotoAsset;
    selected?: boolean;
  }>(),
  { selected: false },
);

defineEmits<{
  select: [];
  open: [];
}>();

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
    @click="$emit('select')"
    @dblclick="$emit('open')"
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
      class="absolute left-1 top-1 rounded bg-black/55 px-1 py-px text-[8px] font-semibold uppercase tracking-wider text-white/85 backdrop-blur-[2px]"
    >
      RAW
    </span>
  </div>
</template>
