<script setup lang="ts">
import { Flag, Heart, X } from 'lucide-vue-next';
import { computed } from 'vue';

import AssociationBadges from '@/components/AssociationBadges.vue';
import PhotoPlaceholder from '@/components/PhotoPlaceholder.vue';
import { dimensionsFor, markPreviewFailed, useThumbnail } from '@/composables/useThumbnails';
import { ASSET_DRAG_MIME } from '@/lib/assets';
import {
  formatBytes,
  formatDimensions,
  formatShortDate,
  kindLabel,
  sourceLabel,
} from '@/lib/format';
import { LIST_GRID_CLASS } from '@/lib/list-rows';
import {
  COLOR_LABEL_NAMES,
  COLOR_LABEL_SWATCHES,
  formatRating,
} from '@/lib/metadata';
import { useLibraryStore } from '@/stores/library';
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
  () => 40,
);

const dimensions = computed(() => {
  const known = dimensionsFor(props.asset);
  return known ? formatDimensions(known.width, known.height) : '—';
});

const metadata = computed(() => props.asset.metadata);

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
      compact ? LIST_GRID_CLASS.compact : LIST_GRID_CLASS.full,
    ]"
    @click="emit('select', $event)"
    @dblclick="$emit('open')"
    @contextmenu.prevent.stop="store.openPhotoMenu(asset.id, $event.clientX, $event.clientY)"
    draggable="true"
    @dragstart="onDragStart"
    @dragend="store.endAssetDrag()"
  >
    <span v-if="selected" class="absolute inset-y-0 left-0 w-0.5 bg-accent" />

    <div class="flex min-w-0 items-center gap-2.5">
      <div
        class="thumb-frame relative h-10 w-10 shrink-0 overflow-hidden bg-canvas ring-1 ring-black/[0.06] dark:ring-white/[0.08]"
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
        <p class="mt-0.5 truncate text-2xs text-subtle">
          {{ subline }}
        </p>
      </div>
    </div>

    <div class="flex min-w-0 items-center">
      <span
        v-if="metadata.rating > 0"
        class="truncate text-2xs font-semibold tracking-tight text-amber-500 tabular-nums"
        :title="`${metadata.rating} ${metadata.rating === 1 ? 'star' : 'stars'}`"
      >
        {{ formatRating(metadata.rating) }}
      </span>
      <span v-else class="text-2xs text-subtle" title="No rating">—</span>
    </div>

    <div class="flex min-w-0 items-center">
      <span
        v-if="metadata.label"
        class="h-2.5 w-2.5 shrink-0 rounded-full ring-1 ring-black/25"
        :class="COLOR_LABEL_SWATCHES[metadata.label]"
        :title="`${COLOR_LABEL_NAMES[metadata.label]} label`"
      />
      <span v-else class="text-2xs text-subtle" title="No label">—</span>
    </div>

    <div class="flex min-w-0 items-center gap-1">
      <span
        v-if="metadata.favorite"
        class="inline-flex shrink-0 text-rose-500"
        title="Favorite"
      >
        <Heart class="h-3 w-3" fill="currentColor" />
      </span>
      <span
        v-if="metadata.cull === 'pick'"
        class="inline-flex shrink-0 text-success"
        title="Picked"
      >
        <Flag class="h-3 w-3" fill="currentColor" />
      </span>
      <span
        v-else-if="metadata.cull === 'reject'"
        class="inline-flex shrink-0 text-danger"
        title="Rejected"
      >
        <X class="h-3 w-3" />
      </span>
      <span
        v-if="!metadata.favorite && metadata.cull === null"
        class="text-2xs text-subtle"
        title="No culling flag or favorite"
      >
        —
      </span>
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
    </template>
    <span class="flex items-center justify-end gap-1">
      <AssociationBadges :asset="asset" />
      <span
        v-if="!asset.companionJpeg && !asset.xmpSidecar"
        class="text-2xs text-subtle"
      >
        —
      </span>
    </span>
  </div>
</template>
