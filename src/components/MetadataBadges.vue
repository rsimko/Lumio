<script setup lang="ts">
import { Flag, Heart, Star, X } from 'lucide-vue-next';
import { computed } from 'vue';

import {
  COLOR_LABEL_NAMES,
  COLOR_LABEL_SWATCHES,
  formatRating,
  hasVisibleMetadata,
} from '@/lib/metadata';
import type { PhotoAsset } from '@/types';

const props = withDefaults(
  defineProps<{
    asset: PhotoAsset;
    /** Overlay chips are drawn on photos; inline badges sit in a row. */
    variant?: 'overlay' | 'inline' | 'list';
  }>(),
  { variant: 'overlay' },
);

const metadata = computed(() => props.asset.metadata);
const visible = computed(() => hasVisibleMetadata(metadata.value));
const overlay = computed(() => props.variant === 'overlay');
const list = computed(() => props.variant === 'list');

const chipClass = computed(() =>
  overlay.value
    ? 'rounded bg-black/55 px-1 py-0.5 text-white/90 backdrop-blur-[2px]'
    : 'rounded px-1 py-0.5',
);
</script>

<template>
  <span
    v-if="visible"
    class="flex items-center gap-1 leading-none"
    :class="[overlay ? 'drop-shadow-sm' : 'text-muted', list ? 'text-2xs' : 'text-3xs']"
    aria-label="Photo metadata"
  >
    <span
      v-if="metadata.rating > 0"
      class="flex items-center gap-0.5 font-semibold tabular-nums"
      :class="
        list
          ? 'tracking-tight text-amber-500'
          : [chipClass, overlay ? 'text-amber-300' : 'text-amber-500']
      "
      :title="`${metadata.rating} ${metadata.rating === 1 ? 'star' : 'stars'}`"
    >
      <template v-if="list">{{ formatRating(metadata.rating) }}</template>
      <template v-else>
        <Star class="h-2.5 w-2.5" fill="currentColor" />
        {{ metadata.rating }}
      </template>
    </span>
    <span
      v-if="metadata.label"
      class="shrink-0 rounded-full ring-1 ring-black/25"
      :class="[COLOR_LABEL_SWATCHES[metadata.label], list ? 'h-2.5 w-2.5' : 'h-2 w-2']"
      :title="`${COLOR_LABEL_NAMES[metadata.label]} label`"
    />
    <span
      v-if="metadata.favorite"
      :class="
        list ? 'text-rose-500' : [chipClass, overlay ? 'text-rose-300' : 'text-rose-500']
      "
      title="Favorite"
    >
      <Heart class="h-2.5 w-2.5" fill="currentColor" />
    </span>
    <span
      v-if="metadata.cull === 'pick'"
      :class="
        list ? 'text-success' : [chipClass, overlay ? 'text-emerald-300' : 'text-success']
      "
      title="Picked"
    >
      <Flag class="h-2.5 w-2.5" fill="currentColor" />
    </span>
    <span
      v-else-if="metadata.cull === 'reject'"
      :class="list ? 'text-danger' : [chipClass, overlay ? 'text-red-300' : 'text-danger']"
      title="Rejected"
    >
      <X class="h-2.5 w-2.5" />
    </span>
  </span>
</template>
