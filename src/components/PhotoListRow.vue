<script setup lang="ts">
import { FileText } from 'lucide-vue-next';
import { computed } from 'vue';

import PhotoPlaceholder from '@/components/PhotoPlaceholder.vue';
import { dimensionsFor, markPreviewFailed, useThumbnail } from '@/composables/useThumbnails';
import {
  formatBytes,
  formatDimensions,
  formatShortDate,
  kindLabel,
  sourceLabel,
} from '@/lib/format';
import type { PhotoAsset } from '@/types';

const props = withDefaults(
  defineProps<{
    asset: PhotoAsset;
    selected?: boolean;
    showFolder?: boolean;
    compact?: boolean;
  }>(),
  { selected: false, showFolder: false, compact: false },
);

defineEmits<{
  select: [];
  open: [];
}>();

const thumb = useThumbnail(
  () => props.asset,
  () => 40,
);

const dimensions = computed(() => {
  const known = dimensionsFor(props.asset);
  return known ? formatDimensions(known.width, known.height) : '—';
});

const subline = computed(() => {
  if (props.showFolder && props.asset.relativeDir) {
    return props.asset.relativeDir;
  }
  if (thumb.value?.status === 'error') return 'Preview unavailable';
  return sourceLabel(thumb.value?.source ?? props.asset.previewHint);
});
</script>

<template>
  <div
    class="group relative grid h-[58px] cursor-default items-center gap-3 border-b border-line/60 px-4 text-xs transition-colors"
    :class="[
      selected ? 'bg-accent-soft/70' : 'hover:bg-panel-2/60',
      compact
        ? 'grid-cols-[minmax(0,1fr)_54px_100px_38px]'
        : 'grid-cols-[minmax(0,1fr)_60px_104px_84px_112px_44px]',
    ]"
    @click="$emit('select')"
    @dblclick="$emit('open')"
  >
    <span v-if="selected" class="absolute inset-y-0 left-0 w-0.5 bg-accent" />

    <div class="flex min-w-0 items-center gap-2.5">
      <div
        class="relative h-10 w-10 shrink-0 overflow-hidden rounded bg-canvas ring-1 ring-black/[0.06] dark:ring-white/[0.08]"
      >
        <PhotoPlaceholder
          v-if="!thumb?.src"
          :asset="asset"
          :loading="thumb?.status === 'loading'"
          :error="thumb?.error"
          compact
        />
        <img
          v-if="thumb?.src"
          :src="thumb.src"
          :alt="asset.fileName"
          class="h-full w-full object-cover"
          draggable="false"
          loading="lazy"
          decoding="async"
          @error="markPreviewFailed(thumb)"
        />
      </div>
      <div class="min-w-0">
        <p class="truncate text-xs font-medium text-ink">{{ asset.fileName }}</p>
        <p class="mt-0.5 flex items-center gap-1 truncate text-2xs text-subtle">
          <span class="truncate">{{ subline }}</span>
          <FileText
            v-if="asset.xmpSidecar"
            class="h-2.5 w-2.5 shrink-0 text-success"
            title="XMP sidecar associated"
          />
          <span
            v-if="asset.companionJpeg"
            class="shrink-0 rounded bg-success/15 px-1 text-[9px] font-semibold uppercase tracking-wide text-success"
            title="Companion JPEG associated"
          >
            JPG
          </span>
        </p>
      </div>
    </div>

    <span
      class="badge w-fit ring-1"
      :class="
        asset.kind === 'raw'
          ? 'bg-warning/15 text-warning ring-warning/20'
          : 'bg-panel-2 text-muted ring-line'
      "
    >
      {{ kindLabel(asset.kind, asset.extension) }}
    </span>

    <span class="truncate text-2xs tabular-nums text-muted">{{ dimensions }}</span>

    <template v-if="!compact">
      <span class="truncate text-2xs tabular-nums text-muted">{{ formatBytes(asset.fileSize) }}</span>
      <span class="truncate text-2xs text-muted">{{ formatShortDate(asset.modifiedMs) }}</span>
      <span class="flex justify-end text-subtle">
        <FileText
          v-if="asset.xmpSidecar"
          class="h-3 w-3"
          title="XMP sidecar associated"
        />
      </span>
    </template>
    <span v-else class="flex justify-end text-subtle">
      <FileText v-if="asset.xmpSidecar" class="h-3 w-3" title="XMP sidecar associated" />
    </span>
  </div>
</template>
