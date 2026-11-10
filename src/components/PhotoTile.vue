<script setup lang="ts">
import { FileText } from 'lucide-vue-next';
import { computed, ref, watch } from 'vue';

import PhotoPlaceholder from '@/components/PhotoPlaceholder.vue';
import { dimensionsFor, markPreviewFailed, useThumbnail } from '@/composables/useThumbnails';
import { formatDimensions, sourceLabel } from '@/lib/format';
import type { PhotoAsset } from '@/types';

const props = withDefaults(
  defineProps<{
    asset: PhotoAsset;
    width: number;
    height: number;
    selected?: boolean;
    showFooter?: boolean;
  }>(),
  { selected: false, showFooter: true },
);

defineEmits<{
  select: [];
  open: [];
}>();

const thumb = useThumbnail(
  () => props.asset,
  () => Math.max(props.width, props.height),
);
const loaded = ref(false);

watch(
  () => thumb.value?.src,
  () => {
    loaded.value = false;
  },
);

const dimensions = computed(() => {
  const known = dimensionsFor(props.asset);
  return known ? formatDimensions(known.width, known.height) : '—';
});

const dotClass = computed(() => {
  if (!thumb.value || thumb.value.status === 'loading') return 'bg-white/50 animate-pulse';
  if (thumb.value.status === 'error') return 'bg-danger';
  switch (thumb.value.source) {
    case 'companionJpeg':
      return 'bg-success';
    case 'embeddedRaw':
      return 'bg-warning';
    case 'original':
      return 'bg-accent';
    default:
      return 'bg-danger';
  }
});

const dotTitle = computed(() => {
  if (!thumb.value || thumb.value.status === 'loading') return 'Loading preview…';
  if (thumb.value.status === 'error') return thumb.value.error ?? 'Preview unavailable';
  return sourceLabel(thumb.value.source);
});

const footerSource = computed(() => {
  if (!thumb.value || thumb.value.status === 'loading') return 'Loading preview…';
  if (thumb.value.status === 'error') return 'Preview unavailable';
  return sourceLabel(thumb.value.source);
});
</script>

<template>
  <div
    class="group relative h-full w-full overflow-hidden rounded-[5px] bg-canvas ring-1 ring-black/[0.06] transition-shadow dark:ring-white/[0.08]"
    :class="selected ? 'z-10 outline outline-2 outline-offset-2 outline-accent' : ''"
    :title="asset.fileName"
    @click="$emit('select')"
    @dblclick="$emit('open')"
  >
    <PhotoPlaceholder
      class="absolute inset-0"
      :asset="asset"
      :loading="!thumb || thumb.status === 'loading'"
      :error="thumb?.error"
    />
    <img
      v-if="thumb?.src"
      :src="thumb.src"
      :alt="asset.fileName"
      class="absolute inset-0 h-full w-full select-none object-cover transition-opacity duration-200"
      :class="loaded ? 'opacity-100' : 'opacity-0'"
      draggable="false"
      decoding="async"
      loading="lazy"
      @load="loaded = true"
      @error="markPreviewFailed(thumb)"
    />

    <div class="pointer-events-none absolute left-1.5 top-1.5 flex items-center gap-1">
      <span
        class="rounded bg-black/55 px-1 py-0.5 text-3xs font-semibold uppercase tracking-wider text-white/90 backdrop-blur-[2px]"
      >
        {{ asset.extension }}
      </span>
      <span
        v-if="asset.xmpSidecar"
        class="rounded bg-black/55 p-0.5 text-white/80 backdrop-blur-[2px]"
        title="XMP sidecar associated"
      >
        <FileText class="h-2.5 w-2.5" />
      </span>
    </div>

    <span
      class="pointer-events-none absolute right-1.5 top-1.5 h-1.5 w-1.5 rounded-full ring-1 ring-black/25"
      :class="dotClass"
      :title="dotTitle"
    />

    <div
      v-if="showFooter"
      class="pointer-events-none absolute inset-x-0 bottom-0 translate-y-1 bg-gradient-to-t from-black/75 via-black/35 to-transparent px-2 pb-1.5 pt-6 transition duration-150"
      :class="selected ? 'translate-y-0 opacity-100' : 'opacity-0 group-hover:translate-y-0 group-hover:opacity-100'"
    >
      <p class="truncate text-xs font-medium leading-tight text-white">
        {{ asset.fileName }}
      </p>
      <p class="mt-0.5 truncate text-2xs leading-tight text-white/65">
        {{ dimensions }} · {{ footerSource }}
      </p>
    </div>
  </div>
</template>
